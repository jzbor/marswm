use libmars::Client;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::*;
use crate::layouts::*;

#[derive(PartialEq)]
pub struct Workspace<C: Client> {
    _num: usize,
    name: &'static str,
    floating_clients: VecDeque<Rc<RefCell<C>>>, // sorted by stacking order
    tiled_clients: VecDeque<Rc<RefCell<C>>>, // sorted by user
    win_area: Dimensions,
    cur_layout: LayoutType,
    nmain: u32,
}


pub const WORKSPACE_NAMES: &'static [&str; 10] = &[
    "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X",
];


impl<C: Client> Workspace<C> {
    pub fn new(_num: usize, name: &'static str, win_area: Dimensions) -> Workspace<C> {
        return Workspace {
            _num, name,
            floating_clients: VecDeque::new(),
            tiled_clients: VecDeque::new(),
            win_area,
            cur_layout: LayoutType::Floating,
            nmain: 1,
        };
    }

    pub fn apply_layout(&self) {
        let inset: u32 = 27;
        let mut win_area = self.win_area;
        win_area.set_y(win_area.y() + inset as i32);
        win_area.set_h(win_area.h() - inset);
        Layout::get(self.cur_layout).apply_layout(win_area, &self.tiled_clients, self.nmain);
    }

    pub fn cycle_layout(&mut self) {
        let cur_idx = LayoutType::VALUES.iter().position(|l| *l == self.cur_layout).unwrap();
        self.cur_layout = LayoutType::VALUES[(cur_idx + 1) % LayoutType::SIZE];
        self.apply_layout();
    }

    pub fn dec_nmain(&mut self) {
        if self.nmain > 0 {
            self.nmain -= 1;
            self.apply_layout();
        }
    }

    pub fn inc_nmain(&mut self) {
        self.nmain += 1;
        self.apply_layout();
    }

    pub fn name(&self) -> &'static str {
        return self.name;
    }

    pub fn pull_front(&mut self, client_rc: Rc<RefCell<C>>) {
        let mut index_option = None;
        if let Some(index) = self.tiled_clients.iter().position(|c| c == &client_rc) {
            index_option = Some(index);
        }
        if let Some(index) = index_option {
            self.tiled_clients.remove(index);
            self.tiled_clients.push_front(client_rc);
            self.apply_layout();
        }
    }

    pub fn raise_client(&mut self, client_rc: &Rc<RefCell<C>>) {
        if self.floating_clients.contains(client_rc)
                || self.tiled_clients.contains(client_rc) {
            client_rc.borrow().raise();
        }

        // push to front if client is floating (floating clients are sorted by stacking order)
        let index_option = self.floating_clients.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            let client_rc = self.floating_clients.remove(index).unwrap();
            self.floating_clients.push_front(client_rc);
        } else {
            // if the raised client was a tiled client a restack is necessary
            self.restack();
        }
    }

    pub fn restack(&mut self) {
        self.apply_layout();
        for client in self.floating_clients.iter().rev() {
            client.borrow().raise();
        }
    }

    pub fn set_layout(&mut self, layout: LayoutType) {
        self.cur_layout = layout;
        self.apply_layout();
    }

    pub fn set_floating(&mut self, client_rc: Rc<RefCell<C>>, state: bool) {
        if state && self.tiled_clients.contains(&client_rc) {
            let index = self.tiled_clients.iter().position(|c| c == &client_rc).unwrap();
            let client_rc = self.tiled_clients.remove(index).unwrap();
            self.floating_clients.push_front(client_rc);
        } else if !state && self.floating_clients.contains(&client_rc) {
            let index = self.floating_clients.iter().position(|c| c == &client_rc).unwrap();
            let client_rc = self.floating_clients.remove(index).unwrap();
            self.tiled_clients.push_front(client_rc);
        }
        self.restack();
    }

    pub fn tiled_clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.tiled_clients.iter());
    }

    pub fn toggle_floating(&mut self, client_rc: Rc<RefCell<C>>) {
        let state = self.tiled_clients.contains(&client_rc);
        self.set_floating(client_rc, state);
    }

    pub fn update_window_area(&mut self, win_area: Dimensions) {
        self.win_area = win_area;
        self.apply_layout();
    }
}

impl<C: Client> ClientList<C> for Workspace<C> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>) {
        if !client_rc.borrow().is_dialog() {
            self.tiled_clients.push_front(client_rc.clone());
        } else {
            self.floating_clients.push_front(client_rc);
        }
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.floating_clients.iter().chain(self.tiled_clients.iter()));
    }

    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>) {
        let index_option = self.tiled_clients.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            self.tiled_clients.remove(index);
        }

        let index_option = self.floating_clients.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            self.floating_clients.remove(index);
        }
    }
}
