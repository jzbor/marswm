use libmars::common::*;
use libmars::interfaces::wm::Client;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use crate::*;
use crate::config::LayoutConfiguration;
use crate::layouts::*;

#[derive(PartialEq)]
pub struct Workspace<C: Client<Attributes>> {
    name: String,
    global_index: u32,
    clients: VecDeque<Rc<RefCell<C>>>,  // sorted by user
    clients_stack: VecDeque<Rc<RefCell<C>>>,  // sorted by stacking order
    win_area: Dimensions,
    cur_layout: LayoutType,
    layout_config: LayoutConfiguration,
}


impl<C: Client<Attributes>> Workspace<C> {
    pub fn new(name: String, global_index: u32, win_area: Dimensions, layout_config: LayoutConfiguration) -> Workspace<C> {
        Workspace {
            name, global_index,
            clients: VecDeque::new(),
            clients_stack: VecDeque::new(),
            win_area,
            cur_layout: layout_config.default,
            layout_config,
        }
    }

    pub fn apply_layout(&self) {
        let tiled_clients = self.clients.iter()
            .filter(|c| !c.borrow().attributes().is_floating && !c.borrow().is_dialog())
            .cloned().collect();
        Layout::get(self.cur_layout).apply_layout(self.win_area, &tiled_clients, &self.layout_config);
    }

    pub fn change_main_ratio(&mut self, i: f32) {
        let new_ratio = self.layout_config.main_ratio + i;
        if new_ratio > 0.20 && new_ratio < 0.80 {
            self.layout_config.main_ratio = new_ratio;
            self.apply_layout();
        }
    }

    pub fn current_layout(&self) -> LayoutType {
        self.cur_layout
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
        self.global_index
    }

    pub fn inc_nmain(&mut self, i: i32) {
        self.layout_config.nmain = (self.layout_config.nmain as i32 + i) as u32;
        self.apply_layout();
    }

    pub fn inc_gaps(&mut self, i: i32) {
        let new_gaps = self.layout_config.gap_width as i32 + i;
        if new_gaps >= 0 && new_gaps as u32 <= self.win_area.w() / 2 && new_gaps as u32 <= self.win_area.h() / 2 {
            self.layout_config.gap_width = new_gaps as u32;
            self.apply_layout();
        }
    }

    pub fn index_of(&self, client_rc: &Rc<RefCell<C>>) -> Option<usize> {
        self.clients.iter().position(|c| c == client_rc)
    }

    pub fn is_main(&self, client_rc: &Rc<RefCell<C>>) -> bool {
        match self.index_of(client_rc) {
            Some(idx) => (idx as u32) < self.layout_config.nmain,
            None => false,
        }
    }

    pub fn last_active_main(&self) -> Option<Rc<RefCell<C>>> {
        self.clients_stack.iter().find(|c| self.is_main(c)).cloned()
    }

    pub fn last_active_stack(&self) -> Option<Rc<RefCell<C>>> {
        self.clients_stack.iter().find(|c| !self.is_main(c)).cloned()
    }

    pub fn move_main(&mut self, client_rc: Rc<RefCell<C>>) {
        let nmain = self.layout_config.nmain.try_into().unwrap();
        let mut index_option = None;
        if let Some(index) = self.clients.iter().position(|c| c == &client_rc) {
            index_option = Some(index);
        }
        if let Some(index) = index_option {
            let nclients = self.clients.len();
            self.clients.remove(index);
            if index < nmain && nclients > nmain {
                self.clients.insert(nmain, client_rc);
            } else {
                self.clients.push_front(client_rc);
            }
            self.apply_layout();
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn pull_pinned(&mut self) -> Vec<Rc<RefCell<C>>> {
        let mut vec = Vec::new();

        while let Some(client_rc) = self.clients.iter().find(|c| c.borrow().attributes().is_pinned).cloned() {
            // pull client from clients
            let index_option = self.clients.iter().position(|c| *c == client_rc);
            if let Some(index) = index_option {
                self.clients.remove(index);
            }

            // pull client from clients stack
            let index_option = self.clients_stack.iter().position(|c| *c == client_rc);
            if let Some(index) = index_option {
                self.clients_stack.remove(index);
            }

            vec.push(client_rc);
        }

        self.restack();
        vec
    }

    pub fn push_pinned(&mut self, clients: Vec<Rc<RefCell<C>>>) {
        self.clients.extend(clients.iter().cloned());
        self.clients_stack.extend(clients);
        self.restack();
    }

    pub fn raise_client(&mut self, client_rc: &Rc<RefCell<C>>) {
        if !self.contains(&client_rc) {
            return;
        }

        let index_option = self.clients_stack.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            let client_rc = self.clients_stack.remove(index).unwrap();
            self.clients_stack.push_front(client_rc);
        }

        let is_floating = client_rc.borrow().attributes().is_floating;
        client_rc.borrow().raise();
        if !is_floating {
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

        if self.current_layout() != LayoutType::Floating {
            self.clients_stack.iter()
                .filter(|c| c.borrow().attributes().is_floating)
                .rev()
                .for_each(|c| c.borrow().raise());
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
        if !self.contains(&client_rc) {
            return;
        }

        let is_currently_floating = client_rc.borrow().attributes().is_floating;
        if state && !is_currently_floating {
            // restore floating dimensions and save stack position
            let mut client = client_rc.borrow_mut();
            let dimensions_option = client.attributes_mut().floating_dimensions.take();
            if let Some(dimensions) = dimensions_option {
                client.move_resize(dimensions.x(), dimensions.y(), dimensions.w(), dimensions.h());
            } else {
                client.center_on_screen(self.win_area);
            }
        } else if !state && is_currently_floating {
            // save floating dimensions
            let mut client = client_rc.borrow_mut();
            let dimensions = Some(client.dimensions());
            client.attributes_mut().floating_dimensions = dimensions;
        } else {
            return;  // client already has desired state
        }

        client_rc.borrow().export_tiled(state);
        client_rc.borrow_mut().attributes_mut().is_floating = state;
        self.restack();
    }

    pub fn set_pinned(&mut self, client_rc: Rc<RefCell<C>>, state: bool) {
        client_rc.borrow_mut().attributes_mut().is_pinned = state;
        client_rc.borrow().export_pinned(state, Some(self.global_index))
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
        if let Some(pos) = self.clients.iter().position(|c| *c == client_rc) {
            let len = self.clients.len();
            let new_pos = (pos as i32 + inc + len as i32) as usize % len;
            self.clients.remove(pos);
            self.clients.insert(new_pos, client_rc.clone());
            self.apply_layout();
            client_rc.borrow().warp_pointer_to_center();
        }
    }

    pub fn stack_set_pos(&mut self, client_rc: Rc<RefCell<C>>, i: usize) {
        if let Some(pos) = self.clients.iter().position(|c| *c == client_rc) {
            self.clients.remove(pos);
            self.clients.insert(i, client_rc.clone());
            // self.apply_layout();
        }
    }

    pub fn tiled_clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.clients.iter().filter(|c| !c.borrow().attributes().is_floating));
    }

    pub fn update_window_area(&mut self, win_area: Dimensions) {
        self.win_area = win_area;
        self.apply_layout();
    }
}

impl<C: Client<Attributes>> ClientList<C> for Workspace<C> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>) {
        client_rc.borrow_mut().export_workspace(self.global_index);
        self.clients.push_front(client_rc.clone());
        self.clients_stack.push_front(client_rc);
        self.restack();
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.clients.iter());
    }

    fn detach_client(&mut self, client_rc: &Rc<RefCell<C>>) {
        // detach from clients
        let index_option = self.clients.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            self.clients.remove(index);
            self.restack();
        }

        // detach from clients stack
        let index_option = self.clients_stack.iter().position(|c| c == client_rc);
        if let Some(index) = index_option {
            self.clients_stack.remove(index);
        }
    }
}
