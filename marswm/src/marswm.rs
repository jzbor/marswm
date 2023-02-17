use libmars::common::*;
use libmars::wm::{ Backend, Client, WindowManager };
use std::cell::RefCell;
use std::cmp;
use std::env;
use std::marker::PhantomData;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process;
use std::rc::Rc;

use crate::*;
use crate::bindings::*;
use crate::monitor::*;
use crate::rules::*;
use crate::workspace::*;


pub struct MarsWM<B: Backend> {
    backend_phantom: PhantomData<B>,
    exec_path: PathBuf,
    config: Configuration,
    active_client: Option<Rc<RefCell<B::Client>>>,
    monitors: Vec<Monitor<B::Client>>,
    clients: Vec<Rc<RefCell<B::Client>>>,
    keybindings: Vec<Keybinding>,
    rules: Vec<Rule>,
}

impl<B: Backend> MarsWM<B> {
    pub fn new(backend: &mut B, config: Configuration, keybindings: Vec<Keybinding>, rules: Vec<Rule>)
                -> MarsWM<B> {
        // stores exec path to enable reloading after rebuild
        // might have security implications
        let mut wm = MarsWM {
            backend_phantom: PhantomData::default(),
            exec_path: env::current_exe().unwrap(),
            config,
            active_client: None,
            clients: Vec::new(),
            monitors: Vec::new(),
            keybindings,
            rules
        };

        let monitor_config = backend.get_monitor_config();
        (&mut wm as &mut dyn WindowManager<B>).update_monitor_config(backend, monitor_config);
        backend.export_current_workspace(0);

        backend.handle_existing_windows(&mut wm);

        return wm;
    }

    pub fn cleanup(&mut self, backend: &mut B) {
        for client_rc in self.clients.clone() {
            self.unmanage(backend, client_rc);
        }
    }

    fn clients_stacked_order(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<B::Client>>> + '_> {
        let clients = self.monitors.iter().flat_map(|m| m.clients());
        return Box::new(clients);
    }

    fn current_monitor_index(&self, backend: &B) -> usize {
        // TODO save last active monitor to avoid having to use the pointer (avoid backend usage)
        let cursor_pos = backend.pointer_pos();
        let monitor_by_pointer = self.monitors.iter().find(|m| {
            let dims = m.config().dimensions();
            dims.x() <= cursor_pos.0 && cursor_pos.0 < dims.x() + (dims.w() as i32)
                && dims.y() <= cursor_pos.1 && cursor_pos.1 < dims.y() + (dims.h() as i32)
        });

        if let Some(monitor) = monitor_by_pointer {
            return self.monitors.iter().position(|m| m == monitor).unwrap();
        } else {
            return 0;
        }
    }

    pub fn current_monitor(&self, backend: &B) -> &Monitor<B::Client> {
        let index = self.current_monitor_index(backend);
        return self.monitors.get(index).unwrap();
    }

    pub fn current_monitor_mut(&mut self, backend: &B) -> &mut Monitor<B::Client> {
        let index = self.current_monitor_index(backend);
        return self.monitors.get_mut(index).unwrap();
    }

    pub fn current_workspace(&self, backend: &mut B) -> &Workspace<B::Client> {
        return self.current_monitor(backend).current_workspace();
    }

    pub fn current_workspace_mut(&mut self, backend: &mut B) -> &mut Workspace<B::Client> {
        return self.current_monitor_mut(backend).current_workspace_mut();
    }

    pub fn cycle_client(&mut self, backend: &mut B, inc: i32) {
        if let Some(active) = &self.active_client {
            if active.borrow().is_fullscreen() {
                return;
            }

            let ws = self.current_workspace(backend);
            let old_idx_option = ws.tiled_clients().position(|c| c == active);
            let ws = self.current_workspace_mut(backend);
            if let Some(old_idx) = old_idx_option {
                let nclients = ws.tiled_clients().count();
                let new_idx = ((old_idx + nclients) as i32 + inc) as usize % nclients;
                let client = ws.tiled_clients().nth(new_idx).unwrap().clone();
                client.borrow().warp_pointer_to_center();
                ws.raise_client(&client);
            }
        }
    }

    pub fn cycle_workspace(&mut self, backend: &mut B, inc: i32) {
        let monitor = self.current_monitor(backend);
        let cur_workspace_idx = monitor.workspaces().position(|ws| ws == self.current_workspace(backend)).unwrap();
        let new_workspace_idx = monitor.workspace_offset() + ((cur_workspace_idx as i32 + inc) as u32 % monitor.workspace_count());
        self.switch_workspace(backend, new_workspace_idx);
    }

    pub fn decorate_active(&self, client_rc: Rc<RefCell<B::Client>>) {
        let mut client = (*client_rc).borrow_mut();
        if self.config.theming.invert_border_color {
            client.set_inner_color(self.config.theming.inactive_color);
            client.set_outer_color(self.config.theming.inactive_color);
        } else {
            client.set_inner_color(self.config.theming.border_color);
            client.set_outer_color(self.config.theming.border_color);
        }
        client.set_frame_color(self.config.theming.active_color);
    }

    pub fn decorate_inactive(&self, client_rc: Rc<RefCell<B::Client>>) {
        let mut client = (*client_rc).borrow_mut();
        if self.config.theming.invert_border_color {
            client.set_inner_color(self.config.theming.active_color);
            client.set_outer_color(self.config.theming.active_color);
        } else {
            client.set_inner_color(self.config.theming.border_color);
            client.set_outer_color(self.config.theming.border_color);
        }
        client.set_frame_color(self.config.theming.inactive_color);
    }

    pub fn initial_position(&self, backend: &mut B, client_rc: &Rc<RefCell<B::Client>>) -> (i32, i32) {
        let win_area = self.current_monitor(backend).window_area();
        let mut pos = backend.pointer_pos();
        let client = client_rc.borrow();
        pos.0 -= (client.w() / 2) as i32;
        pos.1 -= (client.h() / 2) as i32;
        pos.0 = cmp::max(pos.0, win_area.x());
        pos.1 = cmp::max(pos.1, win_area.y());
        pos.0 = cmp::min(pos.0, win_area.x() + win_area.w() as i32 - client.w() as i32);
        pos.1 = cmp::min(pos.1, win_area.y() + win_area.h() as i32 - client.h() as i32);
        return pos;
    }

    pub fn is_tiled(&self, client_rc: &Rc<RefCell<B::Client>>) -> bool {
        let mut tiled_clients = self.monitors.iter()
            .flat_map(|m| m.workspaces())
            .flat_map(|ws| ws.tiled_clients());
        return tiled_clients.find(|c| *c == client_rc).is_some();
    }

    pub fn get_monitor(&self, client_rc: &Rc<RefCell<B::Client>>) -> Option<&Monitor<B::Client>> {
        return self.monitors.iter().find(|m| m.contains(client_rc));
    }

    pub fn get_monitor_mut(&mut self, client_rc: &Rc<RefCell<B::Client>>) -> Option<&mut Monitor<B::Client>> {
        return self.monitors.iter_mut().find(|m| m.contains(client_rc));
    }

    pub fn get_workspace(&self, client_rc: &Rc<RefCell<B::Client>>) -> Option<&Workspace<B::Client>> {
        return self.monitors.iter().flat_map(|m| m.workspaces())
            .find(|ws| ws.contains(client_rc));
    }

    pub fn get_workspace_mut(&mut self, client_rc: &Rc<RefCell<B::Client>>) -> Option<&mut Workspace<B::Client>> {
        return self.monitors.iter_mut().flat_map(|m| m.workspaces_mut())
            .find(|ws| ws.contains(client_rc));
    }

    pub fn exit(&mut self, backend: &mut B) {
        println!("Shutting down");
        self.cleanup(backend);
        backend.shutdown();
        process::exit(0);
    }

    pub fn restart(&mut self, backend: &mut B) {
        println!("Restarting");
        self.cleanup(backend);
        backend.shutdown();

        // get args without exec_path
        let args = env::args().skip(1);
        eprintln!("Path: {:?}", self.exec_path);
        eprintln!("Args: {:?}", args);

        let mut command = process::Command::new(self.exec_path.clone());
        let command = command.args(args);
        let error = command.exec();
        eprintln!("{}", error);
        process::exit(1);
    }
}

impl<B: Backend> WindowManager<B> for MarsWM<B> {
    fn active_client(&self) -> Option<Rc<RefCell<B::Client>>> {
        return self.active_client.clone();
    }

    fn active_workspace(&self, backend: &mut B) -> u32 {
        return self.current_monitor(backend).current_workspace().global_index();
    }

    fn activate_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        let monitor = self.get_monitor_mut(&client_rc).unwrap();

        // switch workspace
        let workspace_idx_option = monitor.workspaces().enumerate()
            .find(|(_, ws)| ws.contains(&client_rc)).map(|(i, _)| i as u32);
        if let Some(workspace_idx) = workspace_idx_option {
            monitor.switch_workspace(backend, workspace_idx);
        }

        if let Some(workspace) = monitor.workspaces_mut().find(|ws| ws.contains(&client_rc)) {
            workspace.raise_client(&client_rc);
        } else {
            // this might be the case for pinned clients
            client_rc.borrow().raise();
        }
        // client_rc.borrow().warp_pointer_to_center();
        self.focus_client(backend, Some(client_rc));
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<B::Client>>> + '_> {
        return Box::new(self.clients.iter());
    }

    fn client_switches_monitor(&mut self, client_rc: Rc<RefCell<B::Client>>, monitor: u32) {
        for mon in &mut self.monitors {
            mon.detach_client(&client_rc)
        }

        if let Some(monitor) = self.monitors.get_mut(monitor as usize) {
            monitor.attach_client(client_rc);
        } else {
            panic!("Monitor {} not found", monitor);
        }
    }

    fn focus_client(&mut self, backend: &mut B, client_option: Option<Rc<RefCell<B::Client>>>) {
        if let Some(client_rc) = client_option {
            // if let Some(focused_rc) = &self.active_client {
            //     self.decorate_inactive(focused_rc.clone());
            // }

            self.decorate_active(client_rc.clone());
            backend.set_input_focus(client_rc.clone());

            self.active_client = Some(client_rc);
        } else {
            self.active_client = None;
        }

        let ws = self.active_workspace(backend);
        backend.export_current_workspace(ws);
        backend.export_active_window(&self.active_client);
    }

    fn fullscreen_client(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, state: bool) {
        if let Some(mon) = self.get_monitor_mut(&client_rc) {
            if state {
                client_rc.borrow_mut().set_fullscreen(mon.config());
            } else {
                client_rc.borrow_mut().unset_fullscreen();
            }

            if let Some(ws) = self.get_workspace(&client_rc) {
                ws.restack();
            }
        }
    }

    fn handle_button(&mut self, backend: &mut B, modifiers: u32, button: u32, client_option: Option<Rc<RefCell<B::Client>>>) {
        if let Some(client) = client_option {
            if let Some(ws) = self.get_workspace_mut(&client) {
                ws.raise_client(&client);
            } else {
                // this might be the case for pinned windows for example
                client.borrow().raise();
            };

            match button {
                1 => {
                    backend.mouse_move(self, client, button);
                    self.current_monitor_mut(backend).restack_current();
                },
                2 => if modifiers & Modifier::Shift.mask() != 0 {
                    client.borrow().close();
                } else if let Some(ws) = self.get_workspace_mut(&client) {
                    ws.toggle_floating(client);
                },
                3 => {
                    backend.mouse_resize(self, client, button);
                    self.current_monitor_mut(backend).restack_current();
                },
                _ => (),
            }
        }
    }

    fn handle_key(&mut self, backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<B::Client>>>) {
        let actions: Vec<BindingAction> = self.keybindings.iter().filter(|kb| kb.matches(modifiers, key))
            .map(|kb| kb.action()).collect();
        for action in actions {
            action.execute(self, backend, client_option.clone());
        }
    }

    fn manage(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, workspace_preference: Option<u32>) {
        self.clients.push(client_rc.clone());
        let pos = self.initial_position(backend, &client_rc);
        client_rc.borrow_mut().set_pos(pos);

        let monitor = if let Some(monitor_num) = backend.point_to_monitor(client_rc.borrow().center()) {
            self.monitors.get_mut(monitor_num as usize).unwrap()
        } else {
            self.current_monitor_mut(backend)
        };
        monitor.attach_client(client_rc.clone());
        let monitor_conf = monitor.config().clone();

        if let Some(workspace) = workspace_preference {
            self.move_to_workspace(backend, client_rc.clone(), workspace);
        }

        let mut client = (*client_rc).borrow_mut();
        client.show();
        client.center_on_screen(&monitor_conf);

        // configure look
        if !client.dont_decorate() {
            client.set_inner_bw(self.config.theming.inner_border_width);
            client.set_outer_bw(self.config.theming.outer_border_width);
            let fw = self.config.theming.frame_width;
            client.set_frame_width(fw);
        }

        // bind buttons
        client.bind_button(DEFAULT_MODKEY.mask(), 1);
        client.bind_button(DEFAULT_MODKEY.mask(), 2);
        client.bind_button(DEFAULT_MODKEY.mask() | Modifier::Shift.mask(), 2);
        client.bind_button(DEFAULT_MODKEY.mask(), 3);

        // bind keys
        for keybinding in &self.keybindings {
            client.bind_key(keybinding.modifiers(), keybinding.key());
        }

        drop(client);

        if let Some(workspace) = workspace_preference {
            self.move_to_workspace(backend, client_rc.clone(), workspace);
        }

        // adjust workspace to new client
        if let Some(workspace) = self.get_workspace_mut(&client_rc) {
            workspace.drop_fullscreen();
            workspace.restack();
        }

        // set client as currently focused
        self.focus_client(backend, Some(client_rc.clone()));
        client_rc.borrow_mut().warp_pointer_to_center();

        let clients = <marswm::MarsWM<B> as WindowManager<B>>::clients(self).collect();
        let clients_stacked = self.clients_stacked_order().collect();
        backend.export_client_list(clients, clients_stacked);

        // apply window rules
        let actions: Vec<BindingAction> = self.rules.iter()
            .filter(|r| r.matches(client_rc.clone()))
            .flat_map(|r| r.actions())
            .cloned().collect();
        actions.iter()
            .for_each(|a| a.execute(self, backend, Some(client_rc.clone())))
    }

    fn move_request(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, x: i32, y: i32) -> bool {
        if let Some(ws) = self.get_workspace(&client_rc) {
            if ws.is_floating(&client_rc) {
                let mut client = client_rc.borrow_mut();
                let (width, height) = client.size();
                client.move_resize(x, y, width, height);
                return true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    fn move_to_workspace(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, workspace_idx: u32) {
        let mon = self.get_monitor_mut(&client_rc).unwrap();
        if workspace_idx >= mon.workspace_count() {
            return;
        }

        mon.move_to_workspace(client_rc.clone(), workspace_idx);
        self.decorate_inactive(client_rc.clone());
        // TODO focus other client or drop focus
        // hacky workaround:
        self.active_client = None;

        backend.export_active_window(&self.active_client);
        if let Some(workspace) = self.get_workspace(&client_rc) {
            client_rc.borrow().export_workspace(workspace.global_index());
        }
    }

    fn resize_request(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, width: u32, height: u32) -> bool {
        if let Some(ws) = self.get_workspace(&client_rc) {
            if ws.is_floating(&client_rc) {
                let mut client = client_rc.borrow_mut();
                let (x, y) = client.pos();
                client.move_resize(x, y, width, height);
                return true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    }

    fn set_client_pinned(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, state: bool) {
        if let Some(ws) = self.get_workspace_mut(&client_rc) {
            ws.set_pinned(client_rc, state);
        }
    }

    fn tile_client(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, state: bool) {
        if let Some(ws) = self.get_workspace_mut(&client_rc) {
            ws.set_floating(client_rc, !state);
        }
    }

    fn switch_workspace(&mut self, backend: &mut B, workspace_idx: u32) {
        let (mon_idx, rel_idx) = if workspace_idx < self.config.primary_workspaces {
            (0, workspace_idx)
        } else {
            let mon_idx = 1 + ((workspace_idx - self.config.primary_workspaces) / self.config.secondary_workspaces);
            let rel_idx = (workspace_idx - self.config.primary_workspaces) % self.config.secondary_workspaces;
            (mon_idx as usize, rel_idx)
        };

        // switch monitor if necessary
        if mon_idx >= self.monitors.len() {
            return;
        } else if mon_idx != self.current_monitor_index(backend) {
            let (x, y) = self.monitors[mon_idx].config().dimensions().center();
            backend.warp_pointer(x, y);
        }

        self.current_monitor_mut(backend).switch_workspace(backend, rel_idx);
        self.active_client = None;
        backend.export_active_window(&self.active_client);
    }

    fn toggle_fullscreen_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        let old_state = client_rc.borrow().is_fullscreen();
        self.fullscreen_client(backend, client_rc, !old_state)
    }

    fn toggle_tile_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        let is_tiled = self.is_tiled(&client_rc);
        self.tile_client(backend, client_rc, !is_tiled);
    }

    fn unfocus_client(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        self.decorate_inactive(client_rc);
        self.active_client = None;
    }

    fn unmanage(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        // remove from clients list
        let index_option = self.clients.iter().position(|c| c == &client_rc);
        if let Some(index) = index_option {
            self.clients.remove(index);
        }

        // remove from monitor data structure
        for mon in &mut self.monitors {
            mon.detach_client(&client_rc)
        }

        // unset client as currently active
        if Some(client_rc) == self.active_client {
            self.active_client = None;
        }

        let clients = <marswm::MarsWM<B> as WindowManager<B>>::clients(self).collect();
        let clients_stacked = self.clients_stacked_order().collect();
        backend.export_client_list(clients, clients_stacked);
    }

    fn update_monitor_config(&mut self, backend: &mut B, configs: Vec<MonitorConfig>) {
        if configs.len() == 0 {
            return;
        }

        let cur_monitor_count = self.monitors.len();

        if configs.len() < cur_monitor_count {
            let mut detached_clients = Vec::new();
            let extra_monitors = self.monitors.iter_mut().enumerate()
                .filter(|(i, _)| *i >= configs.len())
                .map(|(_, m)| m);
            for monitor in extra_monitors {
                detached_clients.extend(monitor.detach_all());
            }
            let last_monitor = self.monitors.get_mut(cur_monitor_count - 1).unwrap();
            last_monitor.attach_all(detached_clients);
            self.monitors.truncate(configs.len());
        } else if configs.len() > self.monitors.len() {
            for i in self.monitors.len()..configs.len() {
                let primary = i == 0;
                let workspace_offset = if primary {
                    0
                } else {
                    self.config.primary_workspaces + (i - 1) as u32 * self.config.secondary_workspaces
                };

                let monitor = Monitor::new(configs.get(i).unwrap().clone(), &self.config, primary, workspace_offset);
                self.monitors.push(monitor);
            }
        }

        for i in 0..cmp::min(configs.len(), self.monitors.len()) {
            // let config: MonitorConfig = *configs.get(i).unwrap();
            // self.monitors.get_mut(config.num() as usize).unwrap().update_config(config);
            self.monitors.get_mut(i).unwrap()
                .update_config(configs.get(i).unwrap().clone());
        }

        // export desktop settings
        let workspace_info = self.monitors.iter()
            .flat_map(|m| m.workspaces().map(|ws| (ws.name().to_owned(), m.dimensions(), m.window_area())))
            .collect();
        backend.export_workspaces(workspace_info);
    }
}
