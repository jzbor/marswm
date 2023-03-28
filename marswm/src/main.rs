#![doc = include_str!("../README.md")]


use clap::Parser;
use libmars::utils::configuration::*;
use libmars::wm::*;
use libmars::wm::x11::backend::X11Backend;
use std::cell::RefCell;
use std::rc::Rc;

use crate::attributes::*;
use crate::bindings::*;
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

/// A dynamic window manager
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Print default config and exit
    #[clap(long)]
    print_default_config: bool,

    /// Print default button bindings and exit
    #[clap(long)]
    print_default_buttons: bool,

    /// Print default key bindings and exit
    #[clap(long)]
    print_default_keys: bool,

    /// Print current config and exit
    #[clap(long)]
    print_config: bool,

    /// Print current button bindings and exit
    #[clap(long)]
    print_buttons: bool,

    /// Print current key bindings and exit
    #[clap(long)]
    print_keys: bool,

    /// Print current window rules and exit
    #[clap(long)]
    print_rules: bool,
}


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

        clients
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
    let args = Args::parse();

    if args.print_default_config {
        print_config(&Configuration::default());
        std::process::exit(0);
    } else if args.print_default_buttons {
        print_config(&default_button_bindings());
        std::process::exit(0);
    } else if args.print_default_keys {
        print_config(&default_key_bindings(read_config().primary_workspaces));
        std::process::exit(0);
    } else if args.print_config {
        print_config(&read_config());
        std::process::exit(0);
    } else if args.print_buttons {
        print_config(&read_button_bindings());
        std::process::exit(0);
    } else if args.print_keys {
        let config = read_config();
        print_config(&read_key_bindings(config.primary_workspaces));
        std::process::exit(0);
    } else if args.print_rules {
        print_config(&read_rules());
        std::process::exit(0);
    } else {
        let config = read_config();
        let key_bindings = read_key_bindings(config.primary_workspaces);
        let button_bindings = read_button_bindings();
        let rules = read_rules();

        let mut backend = X11Backend::init("marswm").unwrap();
        let mut wm = MarsWM::new(&mut backend, config, key_bindings, button_bindings, rules);
        backend.run(&mut wm);
    }
}
