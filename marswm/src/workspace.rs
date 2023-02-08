use libmars::common::*;
use libmars::wm::Client;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::*;
use crate::config::LayoutConfiguration;
use crate::layouts::*;

#[derive(PartialEq)]
pub struct Workspace<C: Client> {
    name: String,
    global_index: u32,
    floating_clients: VecDeque<Rc<RefCell<C>>>, // sorted by stacking order
    pinned_clients: Vec<Rc<RefCell<C>>>, // sorted by stacking order
    tiled_clients: VecDeque<Rc<RefCell<C>>>, // sorted by user
    win_area: Dimensions,
    cur_layout: LayoutType,
    layout_config: LayoutConfiguration,
}


impl<C: Client> Workspace<C> {
    pub fn new(name: String, global_index: u32, win_area: Dimensions, layout_config: LayoutConfiguration) -> Workspace<C> {
        return Workspace {
            name, global_index,
            floating_clients: VecDeque::new(),
            pinned_clients: Vec::new(),
            tiled_clients: VecDeque::new(),
            win_area,
            cur_layout: layout_config.default,
            layout_config,
        };
    }

    pub fn apply_layout(&self) {
        Layout::get(self.cur_layout).apply_layout(self.win_area, &self.tiled_clients, &self.layout_config);
    }

    pub fn change_main_ratio(&mut self, i: f32) {
        let new_ratio = self.layout_config.main_ratio + i;
        if new_ratio > 0.20 && new_ratio < 0.80 {
            self.layout_config.main_ratio = new_ratio;
            self.apply_layout();
        }
    }

    pub fn cycle_layout(&mut self) {
        let cur_idx = LayoutType::VALUES.iter().position(|l| *l == self.cur_layout).unwrap();
        self.cur_layout = LayoutType::VALUES[(cur_idx + 1) % LayoutType::SIZE];
        self.apply_layout();
    }

    pub fn drop_fullscreen(&mut self) {
        for client_rc in self.clients() {
            let mut client = client_rc.borrow_mut();
            if client.is_fullscreen() {
                client.unset_fullscreen();
            }
        }
    }

    pub fn global_index(&self) -> u32 {
        return self.global_index;
    }

    pub fn inc_nmain(&mut self, i: i32) {
        self.layout_config.nmain = (self.layout_config.nmain as i32 + i) as u32;
        self.apply_layout();
    }

    pub fn is_floating(&self, client_rc: &Rc<RefCell<C>>) -> bool {
        return self.floating_clients.contains(client_rc);
    }

    pub fn name(&self) -> &str {
        return &self.name;
    }

    pub fn move_main(&mut self, client_rc: Rc<RefCell<C>>) {
        let nmain = self.layout_config.nmain.try_into().unwrap();
        let mut index_option = None;
        if let Some(index) = self.tiled_clients.iter().position(|c| c == &client_rc) {
            index_option = Some(index);
        }
        if let Some(index) = index_option {
            self.tiled_clients.remove(index);
            if index < nmain {
                self.tiled_clients.insert(nmain, client_rc);
            } else {
                self.tiled_clients.push_front(client_rc);
            }
            self.apply_layout();
        }
    }

    pub fn pull_pinned(&mut self) -> Vec<Rc<RefCell<C>>> {
        let mut vec = Vec::new();
        while let Some(client) = self.pinned_clients.pop() {
            // pull client from tiled clients
            let index_option = self.tiled_clients.iter().position(|c| c == &client);
            if let Some(index) = index_option {
                self.tiled_clients.remove(index);
            }

            // pull client from floating clients
            let index_option = self.floating_clients.iter().position(|c| c == &client);
            if let Some(index) = index_option {
                self.floating_clients.remove(index);
            }

            vec.push(client);
        }
        self.restack();
        return vec;
    }

    pub fn push_pinned(&mut self, clients: Vec<Rc<RefCell<C>>>) {
        self.floating_clients.extend(clients.iter().cloned());
        self.pinned_clients.extend(clients);
        self.restack();
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

    pub fn restack(&self) {
        let mut fullscreen_client = None;
        for client_rc in self.clients() {
            let mut client = client_rc.borrow_mut();
            if client.is_fullscreen() {
                if fullscreen_client.is_none() {
                    fullscreen_client = Some(client_rc);
                } else {
                    client.unset_fullscreen();
                }
            }
        }

        self.apply_layout();

        for client in self.floating_clients.iter().rev() {
            client.borrow().raise();
        }

        if let Some(client) = fullscreen_client {
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
            client_rc.borrow().export_tiled(false);
            self.floating_clients.push_front(client_rc);
        } else if !state && self.floating_clients.contains(&client_rc) {
            let index = self.floating_clients.iter().position(|c| c == &client_rc).unwrap();
            let client_rc = self.floating_clients.remove(index).unwrap();
            client_rc.borrow().export_tiled(true);
            self.tiled_clients.push_front(client_rc);
        }
        self.restack();
    }

    pub fn set_pinned(&mut self, client_rc: Rc<RefCell<C>>, state: bool) {
        if state {
            if !self.pinned_clients.contains(&client_rc) {
                self.pinned_clients.push(client_rc);
            }
        } else {
            let index_option = self.pinned_clients.iter().position(|c| c == &client_rc);
            if let Some(index) = index_option {
                self.pinned_clients.remove(index);
            }
        }
    }

    pub fn set_stack_mode(&mut self, mode: StackMode) {
        self.layout_config.stack_mode = mode;
        self.apply_layout();
    }

    pub fn set_stack_position(&mut self, position: StackPosition) {
        self.layout_config.stack_position = position;
        self.apply_layout();
    }

    pub fn stack_move(&mut self, client_rc: Rc<RefCell<C>>, inc: i32) {
        if let Some(pos) = self.tiled_clients.iter().position(|c| *c == client_rc) {
            let len = self.tiled_clients.len();
            let new_pos = (pos as i32 + inc + len as i32) as usize % len;
            self.tiled_clients.remove(pos);
            self.tiled_clients.insert(new_pos, client_rc.clone());
            self.apply_layout();
            client_rc.borrow().warp_pointer_to_center();
        }
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
            client_rc.borrow().export_tiled(true);
            self.tiled_clients.push_front(client_rc.clone());
            self.restack();
        } else {
            client_rc.borrow().export_tiled(false);
            self.floating_clients.push_front(client_rc);
        }
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.floating_clients.iter().chain(self.tiled_clients.iter()));
    }

    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>) {
        // detach from tiled clients (restack necessary)
        let index_option = self.tiled_clients.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            self.tiled_clients.remove(index);
            self.restack();
        }

        // detach from floating clients
        let index_option = self.floating_clients.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            self.floating_clients.remove(index);
        }

        // detach from pinned list
        let index_option = self.pinned_clients.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            self.pinned_clients.remove(index);
        }
    }
}
