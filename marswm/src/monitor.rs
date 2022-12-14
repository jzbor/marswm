use libmars::{ Backend, Client, MonitorConfig };
use std::cell::RefCell;
use std::rc::Rc;

use crate::*;
use crate::workspace::*;

pub struct Monitor<C: Client> {
    config: MonitorConfig,
    workspaces: Vec<Workspace<C>>,
    cur_workspace: usize,
}

impl<C: Client> Monitor<C> {
    pub fn new(config: MonitorConfig) -> Monitor<C> {
        let workspaces: Vec<Workspace<C>> = WORKSPACE_NAMES.iter().take(NUM_WORKSPACES)
            .enumerate().map(|(i, name)| Workspace::new(i, name, config.window_area()))
            .collect();

        assert!(workspaces.len() == NUM_WORKSPACES);

        return Monitor {
            config,
            workspaces,
            cur_workspace: 0,
        };
    }

    pub fn config(&self) -> &MonitorConfig {
        return &self.config;
    }

    pub fn current_workspace(&self) -> &Workspace<C> {
        return &self.workspaces[self.cur_workspace];
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        return &mut self.workspaces[self.cur_workspace];
    }

    pub fn dimensions(&self) -> Dimensions {
        return self.config.dimensions();
    }

    pub fn move_to_workspace(&mut self, client_rc: Rc<RefCell<C>>, workspace_idx: usize) {
        if workspace_idx >= self.workspaces.len() {
            return;
        }

        for ws in &mut self.workspaces {
            ws.detach_client(&client_rc);
        }

        if workspace_idx != self.cur_workspace {
            client_rc.borrow_mut().hide();
        }

        self.workspaces[workspace_idx].attach_client(client_rc);
    }

    pub fn num(&self) -> u32 {
        return self.config.num();
    }

    pub fn switch_workspace(&mut self, backend: &impl Backend<C>, workspace_idx: usize) {
        if workspace_idx == self.cur_workspace {
            return;
        }

        self.workspaces[self.cur_workspace].clients().for_each(|c| c.borrow_mut().hide());
        self.workspaces[workspace_idx].clients().for_each(|c| c.borrow_mut().show());
        self.cur_workspace = workspace_idx;
        backend.export_current_workspace(workspace_idx);
    }

    pub fn update_config(&mut self, config: MonitorConfig) {
        self.config = config;
        for ws in &mut self.workspaces {
            ws.update_window_area(config.window_area());
        }
    }

    pub fn window_area(&self) -> Dimensions {
        return self.config.window_area();
    }

    pub fn workspaces(&self) -> Box<dyn Iterator<Item = &Workspace<C>> + '_>{
        return Box::new(self.workspaces.iter());
    }

    pub fn workspaces_mut(&mut self) -> Box<dyn Iterator<Item = &mut Workspace<C>> + '_>{
        return Box::new(self.workspaces.iter_mut());
    }
}

impl<C: Client> ClientList<C> for Monitor<C> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>) {
        client_rc.borrow().export_workspace(self.cur_workspace);
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
