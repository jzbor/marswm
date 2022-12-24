use libmars::{ Backend, Client, WindowManager };
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;

use crate::*;
use crate::bindings::*;
use crate::monitor::*;
use crate::workspace::*;


pub struct MarsWM<C: Client> {
    config: Configuration,
    active_client: Option<Rc<RefCell<C>>>,
    monitors: Vec<Monitor<C>>,
    clients: Vec<Rc<RefCell<C>>>,
    keybindings: Vec<Keybinding>,
}

impl<C: Client> MarsWM<C> {
    pub fn new<B: Backend<C>>(backend: &mut B, config: Configuration, keybindings: Vec<Keybinding>) -> MarsWM<C> {
        let monitors: Vec<Monitor<C>> = backend.get_monitor_config().iter().map(|mc| Monitor::new(*mc, &config)).collect();
        return MarsWM {
            config,
            active_client: None,
            clients: Vec::new(),
            monitors,
            keybindings,
        };
    }

    fn clients_stacked_order(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        let clients = self.monitors.iter().flat_map(|m| m.clients());
        return Box::new(clients);
    }

    fn current_monitor_index<B: Backend<C>>(&self, backend: &B) -> usize {
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

    pub fn current_monitor<B: Backend<C>>(&self, backend: &B) -> &Monitor<C> {
        let index = self.current_monitor_index(backend);
        return self.monitors.get(index).unwrap();
    }

    pub fn current_monitor_mut<B: Backend<C>>(&mut self, backend: &B) -> &mut Monitor<C> {
        let index = self.current_monitor_index(backend);
        return self.monitors.get_mut(index).unwrap();
    }

    pub fn current_workspace<B: Backend<C>>(&self, backend: &mut B) -> &Workspace<C> {
        return self.current_monitor(backend).current_workspace();
    }

    pub fn current_workspace_mut<B: Backend<C>>(&mut self, backend: &mut B) -> &mut Workspace<C> {
        return self.current_monitor_mut(backend).current_workspace_mut();
    }

    pub fn cycle_client<B: Backend<C>>(&mut self, backend: &mut B, inc: i32) {
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

    pub fn cycle_workspace<B: Backend<C>>(&mut self, backend: &mut B, inc: i32) {
        let monitor = self.current_monitor(backend);
        let cur_workspace_idx = monitor.workspaces().position(|ws| ws == self.current_workspace(backend)).unwrap();
        let new_workspace_idx = (cur_workspace_idx as i32 + inc) as usize % self.config.workspaces;
        let monitor = self.current_monitor_mut(backend);
        monitor.switch_workspace(backend, new_workspace_idx);
    }

    pub fn decorate_active(&self, client_rc: Rc<RefCell<C>>) {
        let mut client = (*client_rc).borrow_mut();
        client.set_inner_color(self.config.theming.background_color);
        client.set_outer_color(self.config.theming.background_color);
        client.set_frame_color(self.config.theming.primary_color);
    }

    pub fn decorate_inactive(&self, client_rc: Rc<RefCell<C>>) {
        let mut client = (*client_rc).borrow_mut();
        client.set_inner_color(self.config.theming.background_color);
        client.set_outer_color(self.config.theming.background_color);
        client.set_frame_color(self.config.theming.secondary_color);
    }

    pub fn initial_position<B: Backend<C>>(&self, backend: &mut B, client_rc: &Rc<RefCell<C>>) -> (i32, i32) {
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

    pub fn get_monitor(&self, client_rc: &Rc<RefCell<C>>) -> Option<&Monitor<C>> {
        return self.monitors.iter().find(|m| m.contains(client_rc));
    }

    pub fn get_monitor_mut(&mut self, client_rc: &Rc<RefCell<C>>) -> Option<&mut Monitor<C>> {
        return self.monitors.iter_mut().find(|m| m.contains(client_rc));
    }

    pub fn get_workspace(&self, client_rc: &Rc<RefCell<C>>) -> Option<&Workspace<C>> {
        return self.monitors.iter().flat_map(|m| m.workspaces())
            .find(|ws| ws.contains(client_rc));
    }

    pub fn get_workspace_mut(&mut self, client_rc: &Rc<RefCell<C>>) -> Option<&mut Workspace<C>> {
        return self.monitors.iter_mut().flat_map(|m| m.workspaces_mut())
            .find(|ws| ws.contains(client_rc));
    }
}

impl<B: Backend<C>, C: Client> WindowManager<B, C> for MarsWM<C> {
    fn active_client(&self) -> Option<Rc<RefCell<C>>> {
        return self.active_client.clone();
    }

    fn active_workspace(&self, backend: &mut B) -> usize {
        return self.current_monitor(backend).current_workspace_idx();
    }

    fn activate_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
        let monitor = self.get_monitor_mut(&client_rc).unwrap();

        // switch workspace
        let workspace_idx_option = monitor.workspaces().enumerate()
            .find(|(_, ws)| ws.contains(&client_rc)).map(|(i, _)| i);
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
        self.handle_focus(backend, Some(client_rc));
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.clients.iter());
    }

    fn handle_button(&mut self, backend: &mut B, modifiers: u32, button: u32, client_option: Option<Rc<RefCell<C>>>) {
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
                _ => println!("unknown action"),
            }
        }
    }

    fn handle_client_switches_monitor(&mut self, client_rc: Rc<RefCell<C>>, monitor: u32) {
        println!("Client {} switched monitor to {}", client_rc.borrow().name(), monitor);
        for mon in &mut self.monitors {
            mon.detach_client(&client_rc)
        }
        if let Some(monitor) = self.monitors.iter_mut().find(|m| m.num() == monitor) {
            monitor.attach_client(client_rc);
        } else {
            panic!("Monitor {} not found", monitor);
        }

        for mon in &mut self.monitors {
            mon.restack_current();
        }
    }

    fn handle_focus(&mut self, backend: &mut B, client_option: Option<Rc<RefCell<C>>>) {
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

        backend.export_active_window(&self.active_client);
    }

    fn handle_fullscreen(&mut self, _backend: &mut B, client_rc: Rc<RefCell<C>>, state: bool) {
        if let Some(mon) = self.get_monitor_mut(&client_rc) {
            client_rc.borrow_mut().set_fullscreen(state, mon.config());
            if let Some((i, _)) = mon.workspaces().enumerate().find(|(_, ws)| ws.contains(&client_rc)) {
                mon.restack(i);
            }
        }
    }

    fn handle_fullscreen_toggle(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
        let old_state = client_rc.borrow().is_fullscreen();
        self.handle_fullscreen(backend, client_rc, !old_state)
    }

    fn handle_unfocus(&mut self, _backend: &mut B, client_rc: Rc<RefCell<C>>) {
        self.decorate_inactive(client_rc);
        self.active_client = None;
    }

    fn handle_key(&mut self, backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<C>>>) {
        let actions: Vec<BindingAction> = self.keybindings.iter().filter(|kb| kb.matches(modifiers, key))
            .map(|kb| kb.action()).collect();
        for action in actions {
            action.execute(self, backend, client_option.clone());
        }
    }

    fn init(&mut self, backend: &mut B) {
        let ws_names = self.current_monitor(backend).workspaces().map(|ws| ws.name().to_owned()).collect();
        backend.export_workspaces(ws_names);
        backend.export_current_workspace(0);

        backend.handle_existing_windows(self);
    }

    fn manage(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
        self.clients.push(client_rc.clone());
        let pos = self.initial_position(backend, &client_rc);
        client_rc.borrow_mut().set_pos(pos);
        let monitor_conf = if let Some(monitor_num) = backend.point_to_monitor(client_rc.borrow().center()) {
            let monitor = self.monitors.iter_mut().find(|m| m.num() == monitor_num).unwrap();
            monitor.attach_client(client_rc.clone());
            monitor.config()
        } else {
            self.current_monitor_mut(backend).attach_client(client_rc.clone());
            self.current_monitor(backend).config()
        };
        // self.current_monitor_mut().attach_client(client_rc.clone());

        let mut client = (*client_rc).borrow_mut();
        client.show();
        client.center_on_screen(monitor_conf);

        // configure look
        if !client.dont_decorate() {
            client.set_inner_bw(self.config.theming.inner_border_width);
            client.set_outer_bw(self.config.theming.outer_border_width);
            client.set_frame_width(self.config.theming.frame_width);
        }

        // bind buttons
        client.bind_button(MODKEY.mask(), 1);
        client.bind_button(MODKEY.mask(), 2);
        client.bind_button(MODKEY.mask() | Modifier::Shift.mask(), 2);
        client.bind_button(MODKEY.mask(), 3);

        // bind keys
        for keybinding in &self.keybindings {
            client.bind_key(keybinding.modifiers(), keybinding.key());
        }

        drop(client);

        // set client as currently focused
        self.handle_focus(backend, Some(client_rc.clone()));
        client_rc.borrow_mut().warp_pointer_to_center();

        self.current_monitor(backend).restack_current();

        let clients = <marswm::MarsWM<C> as libmars::WindowManager<B, C>>::clients(self).collect();
        let clients_stacked = self.clients_stacked_order().collect();
        backend.export_client_list(clients, clients_stacked);
    }

    fn move_to_workspace(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>, workspace_idx: usize) {
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
        client_rc.borrow().export_workspace(workspace_idx);
    }

    fn set_client_pinned(&mut self, _backend: &mut B, client_rc: Rc<RefCell<C>>, state: bool) {
        if let Some(mon) = self.get_monitor_mut(&client_rc) {
            mon.set_client_pinned(client_rc, state);
        }
    }

    fn switch_workspace(&mut self, backend: &mut B, workspace_idx: usize) {
        self.current_monitor_mut(backend).switch_workspace(backend, workspace_idx);
    }

    fn unmanage(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
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

        self.current_monitor_mut(backend).restack_current();

        let clients = <marswm::MarsWM<C> as libmars::WindowManager<B, C>>::clients(self).collect();
        let clients_stacked = self.clients_stacked_order().collect();
        backend.export_client_list(clients, clients_stacked);
    }

    fn update_monitor_config(&mut self, configs: Vec<MonitorConfig>) {
        if configs.len() == 0 {
            return;
        }

        let cur_monitor_count = self.monitors.len();

        if configs.len() < cur_monitor_count {
            let mut detached_clients = Vec::new();
            for monitor in self.monitors.iter_mut().filter(|m| m.num() >= configs.len().try_into().unwrap()) {
                detached_clients.extend(monitor.detach_all());
            }
            let last_monitor = self.monitors.get_mut(cur_monitor_count - 1).unwrap();
            last_monitor.attach_all(detached_clients);
            self.monitors.truncate(configs.len());
        } else if configs.len() > self.monitors.len() {
            for i in self.monitors.len()..configs.len() {
                let monitor = Monitor::new(*configs.get(i).unwrap(), &self.config);
                self.monitors.push(monitor);
            }
        }

        for i in 0..cmp::min(configs.len(), self.monitors.len()) {
            // let config: MonitorConfig = *configs.get(i).unwrap();
            // self.monitors.get_mut(config.num() as usize).unwrap().update_config(config);
            self.monitors.get_mut(i).unwrap()
                .update_config(*configs.get(i).unwrap());
        }
    }
}
