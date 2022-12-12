extern crate x11;

use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;

use libmars::*;
use libmars::x11::backend::*;

use crate::marswm::*;

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
}

fn main() {
    let mut backend = X11Backend::init().unwrap();
    let mut wm = MarsWM::new(&mut backend);
    wm.init(&mut backend);
    backend.run(&mut wm);
}
