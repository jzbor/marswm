use libmars::{ Backend, Client, WindowManager };
use std::cmp;
use std::cell::RefCell;
use std::rc::Rc;

use crate::*;
use crate::bindings::*;
use crate::monitor::*;
use crate::workspace::*;


const PRIMARY_COLOR: u64 = 0x31748f;
const SECONDARY_COLOR: u64 = 0xe0def4;
const FRAME_WIDTH: u32 = 5;
const INNER_BORDER_WIDTH: u32 = 2;
const OUTER_BORDER_WIDTH: u32 = 2;


pub struct MarsWM<C: Client> {
    active_client: Option<Rc<RefCell<C>>>,
    monitors: Vec<Monitor<C>>,
    clients: Vec<Rc<RefCell<C>>>,
}

impl<C: Client> MarsWM<C> {
    pub fn new<B: Backend<C>>(backend: &mut B) -> MarsWM<C> {
        let monitors: Vec<Monitor<C>> = backend.get_monitor_config().iter().map(|mc| Monitor::new(*mc)).collect();
        return MarsWM {
            active_client: None,
            clients: Vec::new(),
            monitors,
        };
    }

    pub fn current_monitor(&self) -> &Monitor<C> {
        return match &self.active_client {
            Some(c) => self.monitors.iter().find(|mon| mon.contains(&c)),
            None => self.monitors.get(0),
        }.unwrap();
    }

    pub fn current_monitor_mut(&mut self) -> &mut Monitor<C> {
        return match &self.active_client {
            Some(c) => self.monitors.iter_mut().find(|mon| mon.contains(&c)),
            None => self.monitors.get_mut(0),
        }.unwrap();
    }

    pub fn current_workspace(&self) -> &Workspace<C> {
        return self.current_monitor().current_workspace();
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        return self.current_monitor_mut().current_workspace_mut();
    }

    pub fn cycle_client(&mut self) {
        if let Some(active) = &self.active_client {
            let ws = self.current_workspace();
            if let Some(old_idx) = ws.clients().position(|c| c == active) {
                let new_idx = (old_idx + 1) % ws.clients().count();
                let client = ws.clients().nth(new_idx).unwrap();
                client.borrow().warp_pointer_to_center();
                client.borrow().raise();
            }
        }
    }

    pub fn decorate_active(&self, client_rc: Rc<RefCell<C>>) {
        let mut client = (*client_rc).borrow_mut();
        client.set_inner_color(SECONDARY_COLOR);
        client.set_outer_color(SECONDARY_COLOR);
        client.set_frame_color(PRIMARY_COLOR);
    }

    pub fn decorate_inactive(&self, client_rc: Rc<RefCell<C>>) {
        let mut client = (*client_rc).borrow_mut();
        client.set_inner_color(PRIMARY_COLOR);
        client.set_outer_color(PRIMARY_COLOR);
        client.set_frame_color(SECONDARY_COLOR);
    }

    pub fn visible_clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return self.current_monitor().current_workspace().clients();
    }
}

impl<B: Backend<C>, C: Client> WindowManager<B, C> for MarsWM<C> {
    fn active_client(&self) -> Option<Rc<RefCell<C>>> {
        return self.active_client.clone();
    }

    fn activate_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
        let monitor = self.monitors.iter_mut().find(|m| m.contains(&client_rc)).unwrap();
        let workspace_idx = monitor.workspaces().enumerate()
            .find(|(_, ws)| ws.contains(&client_rc)).map(|(i, _)| i).unwrap();
        monitor.switch_workspace(backend, workspace_idx);
        self.handle_focus(backend, Some(client_rc));
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.clients.iter());
    }

    fn handle_button(&mut self, backend: &mut B, _modifiers: u32, button: u32, client_option: Option<Rc<RefCell<C>>>) {
        if let Some(client) = client_option {
            client.borrow().raise();
            match button {
                1 => {
                    backend.mouse_move(self, client, button);
                    self.current_workspace_mut().apply_layout();
                },
                2 => client.borrow().close(),
                3 => {
                    backend.mouse_resize(self, client, button);
                    self.current_workspace_mut().apply_layout();
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
            mon.current_workspace_mut().apply_layout();
        }
    }

    fn handle_focus(&mut self, backend: &mut B, client_option: Option<Rc<RefCell<C>>>) {
        if let Some(client_rc) = client_option {
            // if let Some(focused_rc) = &self.active_client {
            //     self.decorate_inactive(focused_rc.clone());
            // }

            self.decorate_active(client_rc.clone());

            self.active_client = Some(client_rc);
        } else {
            self.active_client = None;
        }

        backend.export_active_window(&self.active_client);
    }

    fn handle_unfocus(&mut self, _backend: &mut B, client_rc: Rc<RefCell<C>>) {
        self.decorate_inactive(client_rc);
        self.active_client = None;
    }

    fn handle_key(&mut self, backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<C>>>) {
        keybindings().iter().for_each(|kb| { kb.check(modifiers, key, self, backend, client_option.clone()); });
    }

    fn init(&mut self, backend: &mut B) {
        let ws_names = self.current_monitor().workspaces().map(|ws| ws.name().to_owned()).collect();
        backend.export_workspaces(ws_names);
        backend.export_current_workspace(0);

        backend.handle_existing_windows(self);
    }

    fn manage(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
        self.clients.push(client_rc.clone());
        client_rc.borrow_mut().set_pos(backend.pointer_pos());
        if let Some(monitor_num) = backend.point_to_monitor(client_rc.borrow().center()) {
            let monitor = self.monitors.iter_mut().find(|m| m.num() == monitor_num).unwrap();
            monitor.attach_client(client_rc.clone());
        } else {
            self.current_monitor_mut().attach_client(client_rc.clone());
        }
        // self.current_monitor_mut().attach_client(client_rc.clone());

        let mut client = (*client_rc).borrow_mut();
        client.show();
        client.raise();

        // configure look
        client.set_inner_bw(INNER_BORDER_WIDTH);
        client.set_outer_bw(OUTER_BORDER_WIDTH);
        client.set_frame_width(FRAME_WIDTH);
        client.set_inner_color(0x000000);
        client.set_outer_color(0x000000);
        client.set_frame_color(0xffffff);

        // bind buttons
        client.bind_button(MODKEY, 1);
        client.bind_button(MODKEY, 2);
        client.bind_button(MODKEY, 3);

        // bind keys
        for keybinding in keybindings::<B, C>() {
            client.bind_key(keybinding.modifiers(), keybinding.key());
        }

        drop(client);

        backend.export_client_list(&self.clients);

        self.current_workspace_mut().apply_layout();
    }

    fn move_to_workspace(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>, workspace_idx: usize) {
        let mon = self.monitors.iter_mut().find(|m| m.contains(&client_rc)).unwrap();
        mon.move_to_workspace(client_rc.clone(), workspace_idx);
        self.decorate_inactive(client_rc.clone());
        // TODO focus other client or drop focus
        // hacky workaround:
        self.active_client = None;

        backend.export_active_window(&self.active_client);
        client_rc.borrow().export_workspace(workspace_idx);
        self.current_workspace_mut().apply_layout();
    }

    fn switch_workspace(&mut self, backend: &mut B, workspace_idx: usize) {
        self.current_monitor_mut().switch_workspace(backend, workspace_idx);
        backend.export_current_workspace(workspace_idx);
        // TODO focus other client or drop focus
        // hacky workaround:
        self.active_client = None;
        backend.export_active_window(&self.active_client);
        self.current_workspace_mut().apply_layout();
    }

    fn unmanage(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
        // remove from clients list
        let mut index_option = None;
        if let Some(index) = self.clients.iter().position(|c| c == &client_rc) {
            index_option = Some(index);
        }
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

        backend.export_client_list(&self.clients);

        self.current_workspace_mut().apply_layout();
    }

    fn update_monitor_config(&mut self, configs: Vec<MonitorConfig>) {
        println!("Monitor config update");
        if configs.len() == 0 {
            return;
        }

        if configs.len() > self.monitors.len() {
            let mut detached_clients = Vec::new();
            for monitor in self.monitors.iter_mut().filter(|m| m.num() >= configs.len().try_into().unwrap()) {
                detached_clients.extend(monitor.detach_all());
            }
            let last_monitor = self.monitors.get_mut(configs.len() - 1).unwrap();
            last_monitor.attach_all(detached_clients);
        } else if configs.len() < self.monitors.len() {
            println!("Adding new monitor");
            for i in self.monitors.len()..configs.len() {
                let monitor = Monitor::new(*configs.get(i).unwrap());
                self.monitors.push(monitor);
            }
            println!("Adding new monitor");
        }

        println!("Monitors:");
        for mon in &self.monitors {
            println!("{}", mon.num());
        }

        for i in 0..cmp::min(configs.len(), self.monitors.len()) {
            // let config: MonitorConfig = *configs.get(i).unwrap();
            // self.monitors.get_mut(config.num() as usize).unwrap().update_config(config);
            self.monitors.get_mut(i).unwrap()
                .update_config(*configs.get(i).unwrap());
        }
    }
}
