extern crate x11;

use x11::keysym::*;
use x11::xlib::{Mod1Mask, Mod4Mask, ShiftMask, ControlMask};
use std::rc::Rc;
use std::cell::RefCell;

use libmars::*;
use libmars::x11::backend::*;


const MODKEY: u32 = Mod1Mask;

trait ClientList<C: Client> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>);
    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_>;
    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>);

    fn contains(&self, client_rc: &Rc<RefCell<C>>) -> bool {
        return self.clients().find(|&c| c == client_rc).is_some();
    }
}

struct Workspace<C: Client> {
    num: u32,
    name: &'static str,
    clients: Vec<Rc<RefCell<C>>>,
}

struct Monitor<C: Client> {
    config: MonitorConfig,
    workspaces: [Workspace<C>; 4],
    cur_workspace: usize,
}

struct MarsWM<C: Client> {
    focused_client: Option<Rc<RefCell<C>>>,
    monitors: Vec<Monitor<C>>,
    clients: Vec<Rc<RefCell<C>>>,
}

impl<C: Client> Workspace<C> {
    fn new(num: u32, name: &'static str) -> Workspace<C> {
        return Workspace {
            num, name,
            clients: Vec::new(),
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

    fn switch_workspace(&mut self, workspace_idx: usize) {
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
        let cur_monitor = monitors.get(0).unwrap().clone();
        return MarsWM {
            focused_client: None,
            clients: Vec::new(),
            monitors,
        };
    }

    fn current_monitor(&self) -> &Monitor<C> {
        return match &self.focused_client {
            Some(c) => self.monitors.iter().find(|mon| mon.contains(&c)),
            None => self.monitors.get(0),
        }.unwrap();
    }

    fn current_monitor_mut(&mut self) -> &mut Monitor<C> {
        return match &self.focused_client {
            Some(c) => self.monitors.iter_mut().find(|mon| mon.contains(&c)),
            None => self.monitors.get_mut(0),
        }.unwrap();
    }


    fn move_to_workspace(&mut self, client_rc: Rc<RefCell<C>>, workspace_idx: usize) {
        let mon = self.monitors.iter_mut().find(|m| m.contains(&client_rc)).unwrap();
        mon.move_to_workspace(client_rc, workspace_idx);
    }

    fn switch_workspace(&mut self, workspace_idx: usize) {
        self.current_monitor_mut().switch_workspace(workspace_idx);
    }
}

impl<B: Backend<C>, C: Client> WindowManager<B, C> for MarsWM<C> {
    fn active_client(&self) -> Option<Rc<RefCell<C>>> {
        return self.focused_client.clone();
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.clients.iter());
    }

    fn handle_button(&mut self, backend: &mut B, _modifiers: u32, button: u32, client_option: Option<Rc<RefCell<C>>>) {
        if let Some(client) = client_option {
            client.borrow().raise();
            match button {
                1 => backend.mouse_move(self, client),
                2 => client.borrow().close(),
                3 => backend.mouse_resize(self, client),
                _ => println!("unknown action"),
            }
        }
    }

    fn handle_focus(&mut self, backend: &mut B, client_option: Option<Rc<RefCell<C>>>) {
        if let Some(client_rc) = client_option {
            if let Some(focused_rc) = &self.focused_client {
                let mut focused = (**focused_rc).borrow_mut();
                focused.set_inner_color(0x000000);
                focused.set_outer_color(0x000000);
                focused.set_frame_color(0xffffff);
            }

            let mut client = (*client_rc).borrow_mut();
            client.set_inner_color(0xffffff);
            client.set_outer_color(0xffffff);
            client.set_frame_color(0x000000);
            drop(client);

            backend.set_input_focus(client_rc.clone());
            self.focused_client = Some(client_rc);
        } else {
            self.focused_client = client_option;
        }
    }

    fn handle_key(&mut self, _backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<C>>>) {
        if let Some(client_rc) = client_option {
            if modifiers == MODKEY {
                match key {
                    XK_Delete => client_rc.borrow().close(),
                    _ => println!("unknown key action"),
                }
            } else if modifiers == MODKEY | ShiftMask {
                match key {
                    XK_F1 => self.move_to_workspace(client_rc, 0),
                    XK_F2 => self.move_to_workspace(client_rc, 1),
                    XK_F3 => self.move_to_workspace(client_rc, 2),
                    XK_F4 => self.move_to_workspace(client_rc, 3),
                    _ => println!("unknown key action"),
                }
            }
        }

        if modifiers == MODKEY {
            match key {
                XK_F1 => self.switch_workspace(0),
                XK_F2 => self.switch_workspace(1),
                XK_F3 => self.switch_workspace(2),
                XK_F4 => self.switch_workspace(3),
                _ => println!("unknown key action"),
            }
        }
    }

    fn init(&mut self, backend: &mut B) {
        backend.handle_existing_windows(self);
    }

    fn manage(&mut self, _backend: &mut B, client_rc: Rc<RefCell<C>>) {
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
        client.bind_key(MODKEY, XK_F1);
        client.bind_key(MODKEY, XK_F2);
        client.bind_key(MODKEY, XK_F3);
        client.bind_key(MODKEY, XK_F1);
        client.bind_key(MODKEY | ShiftMask, XK_F2);
        client.bind_key(MODKEY | ShiftMask, XK_F3);
        client.bind_key(MODKEY | ShiftMask, XK_F4);
        client.bind_key(MODKEY | ShiftMask, XK_F4);
        client.bind_key(MODKEY, XK_Delete);
    }

    fn unmanage(&mut self, _backend: &mut B, client_rc: Rc<RefCell<C>>) {
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

        // unset client as currently focused
        if Some(client_rc) == self.focused_client {
            self.focused_client = None;
        }
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

fn main() {
    let mut backend = X11Backend::init().unwrap();
    let mut wm = MarsWM::new(&mut backend);
    backend.handle_existing_windows(&mut wm);
    backend.run(&mut wm);
}
