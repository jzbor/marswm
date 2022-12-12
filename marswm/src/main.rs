extern crate x11;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use libmars::*;
use libmars::x11::backend::*;

use crate::layouts::*;
use crate::bindings::*;

mod layouts;
mod bindings;


trait ClientList<C: Client> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>);
    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_>;
    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>);

    fn contains(&self, client_rc: &Rc<RefCell<C>>) -> bool {
        return self.clients().find(|&c| c == client_rc).is_some();
    }
}

struct Workspace<C: Client> {
    _num: u32,
    name: &'static str,
    clients: Vec<Rc<RefCell<C>>>,
    cur_layout: LayoutType,
}

struct Monitor<C: Client> {
    config: MonitorConfig,
    workspaces: [Workspace<C>; 4],
    cur_workspace: usize,
}

pub struct MarsWM<C: Client> {
    active_client: Option<Rc<RefCell<C>>>,
    monitors: Vec<Monitor<C>>,
    clients: Vec<Rc<RefCell<C>>>,
    layouts: HashMap<LayoutType, Layout<C>>,
}

impl<C: Client> Workspace<C> {
    fn new(_num: u32, name: &'static str) -> Workspace<C> {
        return Workspace {
            _num, name,
            clients: Vec::new(),
            cur_layout: LayoutType::Floating,
        };
    }
}

impl<C: Client> Monitor<C> {
    fn new(config: MonitorConfig) -> Monitor<C> {
        let workspaces = [
            Workspace::new(0, "I"),
            Workspace::new(1, "II"),
            Workspace::new(2, "III"),
            Workspace::new(3, "IV"),
        ];

        return Monitor {
            config,
            workspaces,
            cur_workspace: 0,
        };
    }

    fn current_workspace(&self) -> &Workspace<C> {
        return &self.workspaces[self.cur_workspace];
    }

    fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        return &mut self.workspaces[self.cur_workspace];
    }

    fn move_to_workspace(&mut self, client_rc: Rc<RefCell<C>>, workspace_idx: usize) {
        println!("Moving client at {:?} to workspace {}", client_rc.borrow().pos(), workspace_idx);
        for ws in &mut self.workspaces {
            ws.detach_client(&client_rc);
        }

        if workspace_idx != self.cur_workspace {
            client_rc.borrow_mut().hide();
        }

        self.workspaces[workspace_idx].attach_client(client_rc);
    }

    fn switch_workspace(&mut self, _backend: &impl Backend<C>, workspace_idx: usize) {
        if workspace_idx == self.cur_workspace {
            return;
        }

        self.workspaces[self.cur_workspace].clients().for_each(|c| c.borrow_mut().hide());
        self.workspaces[workspace_idx].clients().for_each(|c| c.borrow_mut().show());
        self.cur_workspace = workspace_idx;
    }
}

impl<C: Client> MarsWM<C> {
    fn new<B: Backend<C>>(backend: &mut B) -> MarsWM<C> {
        let monitors: Vec<Monitor<C>> = backend.get_monitor_config().iter().map(|mc| Monitor::new(*mc)).collect();
        return MarsWM {
            active_client: None,
            clients: Vec::new(),
            monitors,
            layouts: LAYOUT_TYPES.iter().map(|lt| (*lt, Layout::new(*lt))).collect(),
        };
    }

    fn apply_layout(&self, monitor: &Monitor<C>) {
        self.layouts.get(&monitor.current_workspace().cur_layout).unwrap()
            .apply_layout(monitor.config, self.visible_clients().cloned().collect())
    }

    fn current_monitor(&self) -> &Monitor<C> {
        return match &self.active_client {
            Some(c) => self.monitors.iter().find(|mon| mon.contains(&c)),
            None => self.monitors.get(0),
        }.unwrap();
    }

    fn current_monitor_mut(&mut self) -> &mut Monitor<C> {
        return match &self.active_client {
            Some(c) => self.monitors.iter_mut().find(|mon| mon.contains(&c)),
            None => self.monitors.get_mut(0),
        }.unwrap();
    }

    fn current_workspace(&self) -> &Workspace<C> {
        return self.current_monitor().current_workspace();
    }

    fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        return self.current_monitor_mut().current_workspace_mut();
    }

    fn cycle_layout(&mut self) {
        let cur_idx = LAYOUT_TYPES.iter().position(|l| *l == self.current_workspace().cur_layout).unwrap();
        self.current_workspace_mut().cur_layout = LAYOUT_TYPES[(cur_idx + 1) % LAYOUT_TYPES.len()];
    }

    fn decorate_active(&self, client_rc: Rc<RefCell<C>>) {
        let mut client = (*client_rc).borrow_mut();
        client.set_inner_color(0xffffff);
        client.set_outer_color(0xffffff);
        client.set_frame_color(0x000000);
    }

    fn decorate_inactive(&self, client_rc: Rc<RefCell<C>>) {
        let mut client = (*client_rc).borrow_mut();
        client.set_inner_color(0x000000);
        client.set_outer_color(0x000000);
        client.set_frame_color(0xffffff);
    }

    fn visible_clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return self.current_monitor().current_workspace().clients();
    }
}

impl<B: Backend<C>, C: Client> WindowManager<B, C> for MarsWM<C> {
    fn active_client(&self) -> Option<Rc<RefCell<C>>> {
        return self.active_client.clone();
    }

    fn activate_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
        let monitor = self.monitors.iter_mut().find(|m| m.contains(&client_rc)).unwrap();
        let workspace_idx = monitor.workspaces.iter().enumerate()
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
                    self.apply_layout(self.current_monitor());
                },
                2 => client.borrow().close(),
                3 => {
                    backend.mouse_resize(self, client, button);
                    self.apply_layout(self.current_monitor());
                },
                _ => println!("unknown action"),
            }
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

    #[allow(non_upper_case_globals)]
    fn handle_key(&mut self, backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<C>>>) {
        keybindings().iter().for_each(|kb| { kb.check(modifiers, key, self, backend, client_option.clone()); });
    }

    fn init(&mut self, backend: &mut B) {
        let ws_names = self.current_monitor().workspaces.iter().map(|ws| ws.name.to_owned()).collect();
        backend.export_workspaces(ws_names);
        backend.export_current_workspace(0);

        backend.handle_existing_windows(self);
    }

    fn manage(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>) {
        self.clients.push(client_rc.clone());
        self.current_monitor_mut().attach_client(client_rc.clone());

        let mut client = (*client_rc).borrow_mut();
        client.show();
        client.raise();

        // configure look
        client.set_inner_bw(3);
        client.set_outer_bw(3);
        client.set_frame_width(10);
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

        self.apply_layout(self.current_monitor());
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
        self.apply_layout(self.current_monitor());
    }

    fn switch_workspace(&mut self, backend: &mut B, workspace_idx: usize) {
        self.current_monitor_mut().switch_workspace(backend, workspace_idx);
        backend.export_current_workspace(workspace_idx);
        // TODO focus other client or drop focus
        // hacky workaround:
        self.active_client = None;
        backend.export_active_window(&self.active_client);
        self.apply_layout(self.current_monitor());
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

        self.apply_layout(self.current_monitor());
    }
}

impl<C: Client> ClientList<C> for Workspace<C> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>) {
        self.clients.push(client_rc);
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.clients.iter());
    }

    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>) {
        let mut index_option = None;
        if let Some(index) = self.clients().position(|c| c == client_rc) {
            index_option = Some(index);
        }
        if let Some(index) = index_option {
            self.clients.remove(index);
        }
    }
}

impl<C: Client> ClientList<C> for Monitor<C> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>) {
        self.workspaces[self.cur_workspace].attach_client(client_rc);
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.workspaces.iter().map(|ws| ws.clients()).flatten());
    }

    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>) {
        for ws in &mut self.workspaces {
            ws.detach_client(client_rc);
        }
    }
}

impl<C: Client> PartialEq for Monitor<C> {
    fn eq(&self, other: &Self) -> bool {
        return self.config == other.config;
    }
}

fn main() {
    let mut backend = X11Backend::init().unwrap();
    let mut wm = MarsWM::new(&mut backend);
    wm.init(&mut backend);
    backend.run(&mut wm);
}
