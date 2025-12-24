use libmars::common::*;
use libmars::platforms::x11::misc::WINDOW_MIN_SIZE;
use libmars::interfaces::wm::{ Backend, Client, WindowManager };
use std::cell::RefCell;
use std::env;
use std::marker::PhantomData;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process;
use std::rc::Rc;

use crate::*;
use crate::monitor::*;
use crate::rules::*;
use crate::workspace::*;
use crate::layouts::LayoutType;


pub struct MarsWM<B: Backend<Attributes>> {
    backend_phantom: PhantomData<B>,
    exec_path: PathBuf,
    config: Configuration,
    active_client: Option<Rc<RefCell<B::Client>>>,
    monitors: Vec<Monitor<B::Client>>,
    clients: Vec<Rc<RefCell<B::Client>>>,
    key_bindings: Vec<KeyBinding>,
    button_bindings: Vec<ButtonBinding>,
    rules: Vec<Rule>,
}

impl<B: Backend<Attributes>> MarsWM<B> {
    pub fn new(backend: &mut B, config: Configuration, key_bindings: Vec<KeyBinding>,
               button_bindings: Vec<ButtonBinding>, rules: Vec<Rule>) -> MarsWM<B> {
        // stores exec path to enable reloading after rebuild
        // might have security implications
        let mut wm = MarsWM {
            backend_phantom: PhantomData,
            exec_path: env::current_exe().unwrap(),
            config,
            active_client: None,
            clients: Vec::new(),
            monitors: Vec::new(),
            key_bindings,
            button_bindings,
            rules
        };

        let monitor_config = backend.get_monitor_config();
        (&mut wm as &mut dyn WindowManager<B, Attributes>).update_monitor_config(backend, monitor_config);
        backend.export_current_workspace(0);

        backend.handle_existing_windows(&mut wm);

        wm
    }

    pub fn apply_window_rules(&self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, app_workspace_pref: Option<u32>)
            -> Option<(u32, Vec<BindingAction>, Option<WindowPlacement>)> {
        let rules: Vec<Rule> = self.rules.iter()
            .filter(|r| r.matches(client_rc.clone()))
            .cloned().collect();
        let mut actions = vec![];
        let mut initial_placement = None;

        let current_workspace = self.current_workspace(backend).global_index();
        let mut workspace = match app_workspace_pref {
            Some(ws) => ws,
            None => current_workspace,
        };


        for rule in rules {
            if rule.ignore_window() {
                // make the window visible, but do not manage it
                client_rc.borrow_mut().show();
                return None;
            }

            if let Some(state) = rule.floating() {
                client_rc.borrow_mut().attributes_mut().is_floating = state;
            }

            if rule.initial_placement().is_some() {
                initial_placement = rule.initial_placement();
            }

            if let Some(ws) = rule.workspace() {
                let current_monitor = self.current_monitor(backend);
                if ws >= current_monitor.workspace_count() {
                    workspace = current_monitor.workspace_count() - 1;
                } else {
                    workspace = self.current_monitor(backend).workspace_offset() + ws;
                }
            }

            actions.extend(rule.actions().iter().cloned());
        }

        Some((workspace, actions, initial_placement))
    }

    pub fn cleanup(&mut self, backend: &mut B) {
        for client_rc in self.clients.clone() {
            client_rc.borrow_mut().show();
            self.unmanage(backend, client_rc);
        }
    }

    fn clients_stacked_order(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<B::Client>>> + '_> {
        let clients = self.monitors.iter().flat_map(|m| m.clients());
        Box::new(clients)
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
            self.monitors.iter().position(|m| m == monitor).unwrap()
        } else {
            0
        }
    }

    pub fn current_monitor(&self, backend: &B) -> &Monitor<B::Client> {
        let index = self.current_monitor_index(backend);
        self.monitors.get(index).unwrap()
    }

    pub fn current_monitor_mut(&mut self, backend: &B) -> &mut Monitor<B::Client> {
        let index = self.current_monitor_index(backend);
        self.monitors.get_mut(index).unwrap()
    }

    pub fn current_workspace(&self, backend: &mut B) -> &Workspace<B::Client> {
        self.current_monitor(backend).current_workspace()
    }

    pub fn current_workspace_mut(&mut self, backend: &mut B) -> &mut Workspace<B::Client> {
        self.current_monitor_mut(backend).current_workspace_mut()
    }

    pub fn switch_to_main(&mut self, backend: &mut B) {
        if let Some(active) = &self.active_client {
            if active.borrow().is_fullscreen() {
                return;
            }

            let ws = self.current_workspace(backend);
            let client_opt = if ws.contains(active) && ws.is_main(active) {
                ws.last_active_stack()
            } else if ws.contains(active) {
                ws.last_active_main()
            } else {
                None
            };

            if let Some(client_rc) = client_opt {
                client_rc.borrow().warp_pointer_to_center();
                self.current_workspace_mut(backend).raise_client(&client_rc);
            }
        }
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

    pub fn cycle_monitor(&mut self, backend: &mut B, inc: i32) {
        let current_index = self.current_monitor_index(backend);
        let next_index = (current_index as i32 + inc) as usize % self.monitors.len();
        if let Some(monitor) = self.monitors.get(next_index) {
            let (x, y) = monitor.window_area().center();
            backend.warp_pointer(x, y);
        }
    }

    pub fn cycle_workspace(&mut self, backend: &mut B, inc: i32) {
        let monitor = self.current_monitor(backend);
        let cur_workspace_idx = monitor.workspaces().position(|ws| ws == self.current_workspace(backend)).unwrap();
        let new_workspace_idx = monitor.workspace_offset() + ((cur_workspace_idx as i32 + inc) as u32 % monitor.workspace_count());
        self.switch_workspace(backend, new_workspace_idx);
    }

    pub fn focus_direction(&mut self, backend: &mut B, dir: Direction) {
        if let Some(active) = self.active_client.clone() {
            if active.borrow().is_fullscreen() {
                return;
            }

            let ws = self.current_workspace_mut(backend);
            if let Some(selected) = ws.next_in_direction(active, dir) {
                selected.borrow().warp_pointer_to_center();
                ws.raise_client(&selected);
            }
        }
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
        client.set_title_color(self.config.theming.inactive_color);
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
        client.set_title_color(self.config.theming.active_color);
        client.set_frame_color(self.config.theming.inactive_color);
    }

    pub fn is_tiled(&self, client_rc: &Rc<RefCell<B::Client>>) -> bool {
        let mut tiled_clients = self.monitors.iter()
            .flat_map(|m| m.workspaces())
            .flat_map(|ws| ws.tiled_clients());
        tiled_clients.any(|c| c == client_rc)
    }

    pub fn get_monitor(&self, client_rc: &Rc<RefCell<B::Client>>) -> Option<&Monitor<B::Client>> {
        self.monitors.iter().find(|m| m.contains(client_rc))
    }

    pub fn get_monitor_mut(&mut self, client_rc: &Rc<RefCell<B::Client>>) -> Option<&mut Monitor<B::Client>> {
        self.monitors.iter_mut().find(|m| m.contains(client_rc))
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

    pub fn mouse_action_place(_backend: &mut B, wm: &mut Self, client_rc: &Rc<RefCell<B::Client>>,
                         orig_client_pos: (i32, i32), _orig_client_size: (u32, u32), delta: (i32, i32)) {
        let dest_x = orig_client_pos.0 + delta.0;
        let dest_y = orig_client_pos.1 + delta.1;
        let size = client_rc.borrow().size();
        client_rc.borrow_mut().move_resize(dest_x, dest_y, size.0, size.1);
        let client_center = client_rc.borrow().center();

        if let Some(workspace) = wm.get_workspace_mut(client_rc) {
            let other_index_option = workspace.tiled_clients()
                .enumerate()
                .find(|(_, c)| *c != client_rc && c.borrow().dimensions().contains_point(client_center))
                .map(|(i, _)| i);
            if let Some(index) = other_index_option {
                workspace.stack_set_pos(client_rc.clone(), index);
                workspace.apply_layout();
            }
        }
    }

    pub fn mouse_action_resize_centered(_backend: &mut B, _wm: &mut Self, client_rc: &Rc<RefCell<B::Client>>,
                         orig_client_pos: (i32, i32), orig_client_size: (u32, u32), delta: (i32, i32)) {
        let dest_w = orig_client_size.0 as i32 + 2 * delta.0;
        let dest_h = orig_client_size.1 as i32 + 2 * delta.1;
        let dest_w: u32 = if dest_w < WINDOW_MIN_SIZE as i32 { WINDOW_MIN_SIZE } else { dest_w as u32 };
        let dest_h: u32 = if dest_h < WINDOW_MIN_SIZE as i32 { WINDOW_MIN_SIZE } else { dest_h as u32 };
        let dest_x: i32 = orig_client_pos.0 - delta.0;
        let dest_y: i32 = orig_client_pos.1 - delta.1;
        client_rc.borrow_mut().move_resize(dest_x, dest_y, dest_w, dest_h);
    }


    pub fn mouse_place(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        let mut client = client_rc.borrow_mut();
        let was_fullscreen = client.is_fullscreen();
        if client.is_fullscreen() {
            client.unset_fullscreen();
        }

        client.attributes_mut().is_moving = true;
        client.raise();
        drop(client);

        backend.mouse_action(self, client_rc.clone(), 52, Self::mouse_action_place);

        client_rc.borrow_mut().attributes_mut().is_moving = false;
        if let Some(ws) = self.get_workspace_mut(&client_rc) {
            ws.drop_fullscreen();
            ws.restack()
        }

        if was_fullscreen
            && let Some(mon) = self.get_monitor(&client_rc) {
                client_rc.borrow_mut().set_fullscreen(mon.config());
            }
    }

    pub fn mouse_resize_centered(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        let client = client_rc.borrow_mut();
        if client.is_fullscreen() {
            return;
        }

        client.warp_pointer_to_corner();
        client.raise();
        drop(client);

        backend.mouse_action(self, client_rc.clone(), 120, Self::mouse_action_resize_centered);

        client_rc.borrow_mut().attributes_mut().is_moving = false;
        self.current_workspace_mut(backend).restack();
    }

    pub fn move_client_to_monitor(&mut self, client_rc: Rc<RefCell<B::Client>>, inc: i32) {
        let client_index_opt = self.get_monitor(&client_rc).and_then(|cm| self.monitors.iter().position(|m| m == cm));
        let client_index = if let Some(ci) = client_index_opt { ci } else { return };
        let target_index = (client_index as i32 + inc) as usize % self.monitors.len();

        self.monitors[client_index].detach_client(&client_rc);
        Self::fix_client_to_area(client_rc.clone(), self.monitors[target_index].window_area());
        self.monitors[target_index].attach_client(client_rc.clone());

        if client_rc.borrow().is_fullscreen() {
            client_rc.borrow_mut().set_fullscreen(self.monitors[target_index].config())
        }
    }

    fn fix_client_to_area(client_rc: Rc<RefCell<B::Client>>, area: Dimensions) {
        let client_dimensions_orig = client_rc.borrow().dimensions();
        let mut client_dimensions = client_dimensions_orig;

        if client_dimensions.x() < area.x() {
            client_dimensions.set_x(area.x());
        }
        if client_dimensions.y() < area.y() {
            client_dimensions.set_y(area.y());
        }
        if client_dimensions.right() > area.right() {
            client_dimensions.set_x(area.right() - client_dimensions.w() as i32);
        }
        if client_dimensions.bottom() > area.bottom() {
            client_dimensions.set_y(area.bottom() - client_dimensions.h() as i32);
        }

        if client_dimensions != client_dimensions_orig {
            // center window if it's completely off
            if !area.contains_point(client_dimensions_orig.center()) {
                client_rc.borrow_mut().center_on_screen(area);
            } else {
                client_rc.borrow_mut().set_dimensions(client_dimensions);
            }
        }
    }

    fn relative_workspace_idx(&self, absolute_idx: u32) -> (usize, u32) {
        if absolute_idx < self.config.primary_workspaces {
            (0, absolute_idx)
        } else {
            let mon_idx = 1 + ((absolute_idx - self.config.primary_workspaces) / self.config.secondary_workspaces);
            let rel_idx = (absolute_idx - self.config.primary_workspaces) % self.config.secondary_workspaces;
            (mon_idx as usize, rel_idx)
        }
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

    pub fn swap_direction(&mut self, backend: &mut B, dir: Direction) {
        if let Some(active) = self.active_client.clone() {
            let ws = self.current_workspace_mut(backend);
            if let Some(selected) = ws.next_in_direction(active.clone(), dir) {
                ws.swap_clients(active, selected);
            }
        }
    }

    pub fn switch_prev_workspace(&mut self, backend: &mut B) {
        let prev_index = self.current_monitor(backend).prev_workspace().global_index();
        self.switch_workspace(backend, prev_index);
    }
}

impl<B: Backend<Attributes>> WindowManager<B, Attributes> for MarsWM<B> {
    fn active_client(&self) -> Option<Rc<RefCell<B::Client>>> {
        self.active_client.clone()
    }

    fn active_workspace(&self, backend: &mut B) -> u32 {
        self.current_monitor(backend).current_workspace().global_index()
    }

    fn activate_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        let monitor = self.get_monitor_mut(&client_rc).unwrap();

        // switch workspace
        let option = monitor.workspaces()
            .find(|ws| ws.contains(&client_rc)).map(|ws| ws.global_index());

        if let Some(workspace_idx) = option {
            self.switch_workspace(backend, workspace_idx);
            self.current_workspace_mut(backend).raise_client(&client_rc);
        } else {
            // this might be the case for pinned clients
            client_rc.borrow().raise();
        }

        self.focus_client(backend, Some(client_rc.clone()));
        client_rc.borrow().warp_pointer_to_center();
    }

    fn center_client(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        if is_floating!(self, &client_rc) && let Some(mon) = self.get_monitor(&client_rc) {
            client_rc.borrow_mut().center_on_screen(mon.config().window_area());
        }
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<B::Client>>> + '_> {
        Box::new(self.clients.iter())
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
        if client_option == self.active_client {
            return;
        } else if let Some(client_rc) = client_option {
            self.decorate_active(client_rc.clone());
            backend.set_input_focus(client_rc.clone());

            if let Some(old_client_rc) = self.active_client.take() {
                self.decorate_inactive(old_client_rc);
            }

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

    fn handle_button(&mut self, backend: &mut B, modifiers: u32, button: u32, target: ButtonTarget,
                     client_option: Option<Rc<RefCell<B::Client>>>) {
        if let Some(client) = client_option.clone() {
            if let Some(ws) = self.get_workspace_mut(&client) {
                ws.raise_client(&client);
            } else {
                // this might be the case for pinned windows for example
                client.borrow().raise();
            };
        }

        let actions: Vec<BindingAction> = self.button_bindings.iter().filter(|bb| bb.matches(modifiers, button, target))
            .map(|kb| kb.action()).collect();
        for action in actions {
            action.execute(self, backend, client_option.clone());
        }
    }

    fn handle_key(&mut self, backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<B::Client>>>) {
        let actions: Vec<BindingAction> = self.key_bindings.iter().filter(|kb| kb.matches(modifiers, key))
            .map(|kb| kb.action()).collect();
        for action in actions {
            action.execute(self, backend, client_option.clone());
        }
    }

    fn manage(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, workspace_preference: Option<u32>) {
        let (workspace_idx, actions, initial_placement)
                = match self.apply_window_rules(backend, client_rc.clone(), workspace_preference) {
            Some(params) => params,
            None => return,
        };

        self.clients.push(client_rc.clone());

        // attach client to monitor or workspace
        let (mon, rel_ws) = self.relative_workspace_idx(workspace_idx);
        if let Some(workspace) = self.monitors.get_mut(mon).and_then(|m| m.workspace_mut(rel_ws)) {
            workspace.attach_client(client_rc.clone());
        } else if let Some(monitor) = backend.point_to_monitor(client_rc.borrow().center())
                .and_then(|m| self.monitors.get_mut(m as usize)) {
            monitor.attach_client(client_rc.clone());
        } else {
            self.current_monitor_mut(backend).attach_client(client_rc.clone());
        }


        let mut client = (*client_rc).borrow_mut();

        // configure look
        if !client.dont_decorate() {
            client.set_inner_bw(self.config.theming.inner_border_width);
            client.set_outer_bw(self.config.theming.outer_border_width);
            client.set_frame_width(self.config.theming.frame_width);
            if self.config.theming.show_title {
                let _ignore_result = client.add_title(&self.config.theming.font,
                                                      self.config.theming.title_hpadding,
                                                      self.config.theming.title_vpadding,
                                                      self.config.theming.active_color);
            }
        } else {
            client.set_inner_bw(self.config.theming.no_decoration.inner_border_width);
            client.set_outer_bw(self.config.theming.no_decoration.outer_border_width);
            client.set_frame_width(self.config.theming.no_decoration.frame_width);
        }


        // bind keys and buttons
        for key_binding in &self.key_bindings {
            client.bind_key(key_binding.modifiers(), key_binding.key());
        }
        for button_binding in &self.button_bindings {
            for target in button_binding.targets() {
                if *target != ButtonTarget::Root {
                    client.bind_button(button_binding.modifiers(), button_binding.button(), *target);
                }
            }
        }

        drop(client);

        if Some(self.current_workspace(backend)) == self.get_workspace(&client_rc) {
            client_rc.borrow_mut().show();
        }

        // Center client on screen and set focused
        if let Some(monitor) = self.get_monitor(&client_rc) {
            let pointer_pos = backend.pointer_pos();
            let placement = match initial_placement {
                Some(placement) => placement,
                None => self.config.initial_placement,
            };
            let pos = placement.calc(client_rc.borrow().dimensions(), monitor.window_area(), pointer_pos);
            client_rc.borrow_mut().set_pos(pos);
        }

        // adjust workspace to new client
        if let Some(workspace) = self.get_workspace_mut(&client_rc) {
            workspace.drop_fullscreen();
            workspace.restack();
        }

        // set client as currently focused
        if client_rc.borrow().is_visible() {
            self.focus_client(backend, Some(client_rc.clone()));
            client_rc.borrow_mut().warp_pointer_to_center();
        }

        let clients = <marswm::MarsWM<B> as WindowManager<B, Attributes>>::clients(self).collect();
        let clients_stacked = self.clients_stacked_order().collect();
        backend.export_client_list(clients, clients_stacked);

        // apply window rule actions
        actions.iter().for_each(|a| a.execute(self, backend, Some(client_rc.clone())))
    }

    fn move_request(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, x: i32, y: i32) -> bool {
        if is_floating!(self, &client_rc) {
            let mut client = client_rc.borrow_mut();
            let (width, height) = client.size();
            client.move_resize(x, y, width, height);
            true
        } else {
            false
        }
    }

    fn move_to_workspace(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, workspace_idx: u32) {
        let (mon_idx, rel_idx) = self.relative_workspace_idx(workspace_idx);

        if self.monitors.get(mon_idx).map(|m| m.workspace(rel_idx)).is_none() {
            // avoid loosing windows on invalid indices
            return;
        }

        let from_workspace = match self.get_workspace_mut(&client_rc) {
            Some(workspace) => workspace,
            None => return,
        };
        from_workspace.detach_client(&client_rc);

        let to_workspace = match self.monitors.get_mut(mon_idx).and_then(|m| m.workspace_mut(rel_idx)) {
            Some(workspace) => workspace,
            None => panic!("Window is detached but cannot be attached again"),
        };
        to_workspace.attach_client(client_rc.clone());

        // adjust position to be on workspace
        Self::fix_client_to_area(client_rc.clone(), self.get_monitor_mut(&client_rc).unwrap().window_area());

        self.decorate_inactive(client_rc.clone());
        if !self.monitors.iter().map(|m| m.current_workspace()).any(|ws| ws.contains(&client_rc)) {
            client_rc.borrow_mut().hide();
        }

        // TODO focus other client or drop focus
        // hacky workaround:
        self.active_client = None;
        backend.export_active_window(&self.active_client);
    }

    fn resize_request(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, width: u32, height: u32) -> bool {
        let window_area = self.get_monitor(&client_rc).map(|m| m.window_area());
        let client_is_dialog = client_rc.borrow().is_dialog();

        if is_floating!(self, &client_rc) || client_is_dialog {
            let mut client = client_rc.borrow_mut();
            let (x, y) = client.pos();
            let client_was_centered = window_area.map(|a| client.is_centered_on_screen(a))
                .unwrap_or(false);

            client.move_resize(x, y, width, height);

            if client_is_dialog && client_was_centered
                && let Some(area) = window_area {
                    client.center_on_screen(area);
                }

            true
        } else {
            false
        }
    }

    fn set_client_pinned(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, state: bool) {
        if let Some(ws) = self.get_workspace_mut(&client_rc) {
            ws.set_pinned(client_rc, state);
        }
    }

    fn tile_client(&mut self, _backend: &mut B, client_rc: Rc<RefCell<B::Client>>, state: bool) {
        if let Some(ws) = self.get_workspace_mut(&client_rc)
            && ws.current_layout() != LayoutType::Floating {
                ws.set_floating(client_rc, !state);
            }
    }

    fn switch_workspace(&mut self, backend: &mut B, workspace_idx: u32) {
        // abort if we are already on the specified workspace
        if self.current_workspace(backend).global_index() == workspace_idx {
            return;
        }

        let (mon_idx, rel_idx) = self.relative_workspace_idx(workspace_idx);

        // switch monitor if necessary
        if mon_idx >= self.monitors.len() {
            return;
        } else if mon_idx != self.current_monitor_index(backend) {
            let (x, y) = self.monitors[mon_idx].config().dimensions().center();
            backend.warp_pointer(x, y);
        }

        // take pinned clients from old workspace and hide other clients
        let from_workspace = self.monitors[mon_idx].current_workspace_mut();
        let pinned_clients = from_workspace.pull_pinned();

        // redecorate and hide old clients
        for client_rc in from_workspace.clients().cloned().collect::<Vec<_>>() {
            self.decorate_inactive(client_rc.clone());
            client_rc.borrow_mut().hide();
        }

        // set workspace index to new workspace
        self.monitors[mon_idx].set_cur_workspace(rel_idx);

        // attach pinned clients to new workspace and show clients
        let to_workspace = self.monitors[mon_idx].current_workspace_mut();
        to_workspace.push_pinned(pinned_clients);
        to_workspace.clients().for_each(|c| c.borrow_mut().show());

        // select new window to be focused
        let new_active = if let Some(client_rc) = to_workspace.clients().find(|c| c.borrow().is_fullscreen()) {
            Some(client_rc.clone())
        } else { to_workspace.clients().next().cloned() };
        self.focus_client(backend, new_active);

        backend.export_current_workspace(workspace_idx);
    }

    fn toggle_fullscreen_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        let old_state = client_rc.borrow().is_fullscreen();
        self.fullscreen_client(backend, client_rc, !old_state)
    }

    fn toggle_tile_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>) {
        let is_tiled = self.is_tiled(&client_rc);
        self.tile_client(backend, client_rc, !is_tiled);
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
            backend.export_active_window(&self.active_client);
        }

        let clients = <marswm::MarsWM<B> as WindowManager<B, Attributes>>::clients(self).collect();
        let clients_stacked = self.clients_stacked_order().collect();
        backend.export_client_list(clients, clients_stacked);
    }

    fn update_monitor_config(&mut self, backend: &mut B, configs: Vec<MonitorConfig>) {
        if configs.is_empty() {
            return;
        }

        let cur_monitor_count = self.monitors.len();

        if configs.len() < cur_monitor_count {
            let mut trailing_monitors = self.monitors.iter_mut().skip(configs.len() - 1);
            let last_monitor = trailing_monitors.next().unwrap();
            let extra_monitors = trailing_monitors;
            for monitor in extra_monitors {
                Monitor::transfer_all(monitor, last_monitor);
            }
            self.monitors.truncate(configs.len());
        } else if configs.len() > cur_monitor_count {
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

        for (i, monitor) in self.monitors.iter_mut().enumerate() {
            monitor.update_config(configs.get(i).unwrap().clone());
            let window_area = monitor.window_area();
            for client_rc in monitor.clients() {
                Self::fix_client_to_area(client_rc.clone(), window_area);
            }

        }

        // export desktop settings
        let workspace_info = self.monitors.iter()
            .flat_map(|m| m.workspaces().map(|ws| (ws.name().to_owned(), m.dimensions(), m.window_area())))
            .collect();
        backend.export_workspaces(workspace_info);
    }
}
