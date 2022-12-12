use libmars::{ Backend, Client, MonitorConfig };
use std::cell::RefCell;
use std::rc::Rc;

use crate::*;
use crate::workspace::*;

pub struct Monitor<C: Client> {
    config: MonitorConfig,
    workspaces: [Workspace<C>; 4],
    cur_workspace: usize,
}

impl<C: Client> Monitor<C> {
    pub fn new(config: MonitorConfig) -> Monitor<C> {
        let workspaces = [
            Workspace::new(0, "I", config.window_area()),
            Workspace::new(1, "II", config.window_area()),
            Workspace::new(2, "III", config.window_area()),
            Workspace::new(3, "IV", config.window_area()),
        ];

        return Monitor {
            config,
            workspaces,
            cur_workspace: 0,
        };
    }

    pub fn current_workspace(&self) -> &Workspace<C> {
        return &self.workspaces[self.cur_workspace];
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        return &mut self.workspaces[self.cur_workspace];
    }

    pub fn move_to_workspace(&mut self, client_rc: Rc<RefCell<C>>, workspace_idx: usize) {
        println!("Moving client at {:?} to workspace {}", client_rc.borrow().pos(), workspace_idx);
        for ws in &mut self.workspaces {
            ws.detach_client(&client_rc);
        }

        if workspace_idx != self.cur_workspace {
            client_rc.borrow_mut().hide();
        }

        self.workspaces[workspace_idx].attach_client(client_rc);
    }

    pub fn switch_workspace(&mut self, _backend: &impl Backend<C>, workspace_idx: usize) {
        if workspace_idx == self.cur_workspace {
            return;
        }

        self.workspaces[self.cur_workspace].clients().for_each(|c| c.borrow_mut().hide());
        self.workspaces[workspace_idx].clients().for_each(|c| c.borrow_mut().show());
        self.cur_workspace = workspace_idx;
    }

    pub fn workspaces(&self) -> Box<dyn Iterator<Item = &Workspace<C>> + '_>{
        return Box::new(self.workspaces.iter());
    }

    pub fn workspaces_mut(&self) -> Box<dyn Iterator<Item = &Workspace<C>> + '_>{
        return Box::new(self.workspaces.iter());
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

impl<C: Client> PartialEq for Monitor<C> {
    fn eq(&self, other: &Self) -> bool {
        return self.config == other.config;
    }
}
