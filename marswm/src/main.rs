#![doc = include_str!("../README.md")]


use libmars::utils::configuration::*;
use libmars::wm::*;
use libmars::wm::x11::backend::X11Backend;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;

use crate::attributes::*;
use crate::bindings::default_keybindings;
use crate::config::*;
use crate::marswm::*;

mod attributes;
mod bindings;
mod config;
mod layouts;
mod marswm;
mod monitor;
mod rules;
mod workspace;

trait ClientList<C: Client<Attributes>> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>);
    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_>;
    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>);

    fn contains(&self, client_rc: &Rc<RefCell<C>>) -> bool {
        return self.clients().any(|c| c == client_rc);
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
    if env::args().any(|a| a == "print-default-config") {
        print_config(&Configuration::default());
        return;
    }

    let config = read_config();

    if env::args().any(|a| a == "print-default-keybindings") {
        print_config(&default_keybindings(config.primary_workspaces));
        return;
    }

    let keybindings = read_keybindings(config.primary_workspaces);
    let rules = read_rules();

    let mut backend = X11Backend::init("marswm").unwrap();
    let mut wm = MarsWM::new(&mut backend, config, keybindings, rules);
    backend.run(&mut wm);
}
