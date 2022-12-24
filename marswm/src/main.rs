extern crate x11;

use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use libmars::*;
use libmars::x11::backend::*;

use crate::bindings::default_keybindings;
use crate::marswm::*;
use crate::config::*;

mod config;
mod layouts;
mod bindings;
mod marswm;
mod monitor;
mod workspace;

trait ClientList<C: Client> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>);
    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_>;
    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>);

    fn contains(&self, client_rc: &Rc<RefCell<C>>) -> bool {
        return self.clients().find(|&c| c == client_rc).is_some();
    }

    fn detach_all(&mut self) -> Vec<Rc<RefCell<C>>> {
        let mut clients = Vec::new();
        for client in self.clients() {
            clients.push(client.clone());
        }
        for client in &clients {
            self.detach_client(client);
        }

        return clients;
    }

    fn attach_all(&mut self, clients: Vec<Rc<RefCell<C>>>) {
        for client in clients {
            self.attach_client(client);
        }
    }

    fn transfer_all(from: &mut Self, other: &mut Self) {
        let clients = from.detach_all();
        other.attach_all(clients);
    }
}

fn main() {
    if env::args().find(|a| a == "print-default-config").is_some() {
        let ser = serde_yaml::to_string(&Configuration::default());
        match ser {
            Ok(ser) => println!("{}", ser),
            Err(e) => println!("Error: {}", e),
        }
        return;
    }

    let config = read_config();

    if env::args().find(|a| a == "print-default-keybindings").is_some() {
        let ser = serde_yaml::to_string(&default_keybindings(config.workspaces));
        match ser {
            Ok(ser) => println!("{}", ser),
            Err(e) => println!("Error: {}", e),
        }
        return;
    }

    let keybindings = read_keybindings(config.workspaces);

    let mut backend = X11Backend::init("marswm").unwrap();
    let mut wm = MarsWM::new(&mut backend, config, keybindings);
    wm.init(&mut backend);
    backend.run(&mut wm);
}
