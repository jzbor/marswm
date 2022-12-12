use libmars::{ Client, MonitorConfig };
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::*;
use crate::layouts::*;

pub struct Workspace<C: Client> {
    _num: u32,
    name: &'static str,
    clients: VecDeque<Rc<RefCell<C>>>,
    cur_layout: LayoutType,
}

impl<C: Client> Workspace<C> {
    pub fn new(_num: u32, name: &'static str) -> Workspace<C> {
        return Workspace {
            _num, name,
            clients: VecDeque::new(),
            cur_layout: LayoutType::Floating,
        };
    }

    pub fn apply_layout(&self, monitor_conf: MonitorConfig) {
        Layout::get(self.cur_layout).apply_layout(monitor_conf, &self.clients)
    }

    pub fn cycle_layout(&mut self, monitor_conf: MonitorConfig) {
        let cur_idx = LAYOUT_TYPES.iter().position(|l| *l == self.cur_layout).unwrap();
        self.cur_layout = LAYOUT_TYPES[(cur_idx + 1) % LAYOUT_TYPES.len()];
        self.apply_layout(monitor_conf);
    }

    pub fn name(&self) -> &'static str {
        return self.name;
    }

    pub fn pull_front(&mut self, client_rc: Rc<RefCell<C>>, monitor_conf: MonitorConfig) {
        let mut index_option = None;
        if let Some(index) = self.clients.iter().position(|c| c == &client_rc) {
            index_option = Some(index);
        }
        if let Some(index) = index_option {
            self.clients.remove(index);
            self.clients.push_front(client_rc);
            self.apply_layout(monitor_conf);
        }
    }
}

impl<C: Client> ClientList<C> for Workspace<C> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>) {
        self.clients.push_front(client_rc);
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
