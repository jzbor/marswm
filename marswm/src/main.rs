extern crate x11;

use x11::keysym::*;
use x11::xlib::{Mod1Mask, Mod4Mask};
use std::rc::Rc;
use std::cell::RefCell;

use libmars::*;
use libmars::x11::backend::*;


const MODKEY: u32 = Mod1Mask;

struct MarsWM<C: Client> {
    clients: Vec<Rc<RefCell<C>>>,
    focused: Option<Rc<RefCell<C>>>,
}

impl<C: Client> MarsWM<C> {
    fn new() -> MarsWM<C> {
        return MarsWM {
            clients: Vec::new(),
            focused: None,
        };
    }
}

impl<B: Backend<C>, C: Client> WindowManager<B, C> for MarsWM<C> {
    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.clients.iter());
    }

    fn clients_mut(&mut self) -> Box<dyn Iterator<Item = &mut Rc<RefCell<C>>> + '_> {
        return Box::new(self.clients.iter_mut());
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
            if let Some(focused_rc) = &self.focused {
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
            self.focused = Some(client_rc);
        } else {
            self.focused = client_option;
        }
    }

    fn handle_key(&mut self, _backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<C>>>) {
        println!("key event");
        if modifiers == MODKEY {
            match key {
                XK_F4 => if let Some(c) = client_option {
                    c.borrow().close();
                },
                _ => println!("unknown key action"),
            }
        }
    }

    fn init(&mut self, backend: &mut B) {
        backend.handle_existing_windows(self);
    }

    fn manage(&mut self, _backend: &mut B, client_rc: Rc<RefCell<C>>) {
        self.clients.push(client_rc.clone());
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
        client.bind_key(MODKEY, XK_F4);
    }

    fn unmanage(&mut self, _backend: &mut B, client_rc: Rc<RefCell<C>>) {
        // remove client from data structure
        let index = self.clients.iter().position(|c| c == &client_rc).unwrap();
        self.clients.remove(index);

        // unset client as currently focused
        if Some(client_rc) == self.focused {
            self.focused = None;
        }
        println!("Clients: {:?}", self.clients.len());
    }
}

fn main() {
    let mut wm = MarsWM::new();
    let mut backend = X11Backend::init().unwrap();
    backend.handle_existing_windows(&mut wm);
    backend.run(&mut wm);
}
