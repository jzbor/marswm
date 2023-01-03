use libmars::{ Backend, Client, MonitorConfig };
use std::cell::RefCell;
use std::rc::Rc;

use crate::*;
use crate::workspace::*;
use crate::config::Configuration;

pub struct Monitor<C: Client> {
    config: MonitorConfig,
    workspaces: Vec<Workspace<C>>,
    cur_workspace: u32,
    prev_workspace: u32,
}

impl<C: Client> Monitor<C> {
    pub fn new(monitor_config: MonitorConfig, config: &Configuration) -> Monitor<C> {
        let workspaces: Vec<Workspace<C>> = WORKSPACE_NAMES.iter().take(config.workspaces as usize)
            .map(|name| Workspace::new(name, monitor_config.window_area(),
                                            config.layout, config.default_layout))
            .collect();

        assert!(workspaces.len() == config.workspaces as usize);

        return Monitor {
            config: monitor_config,
            workspaces,
            cur_workspace: 0,
            prev_workspace: 0,
        };
    }

    pub fn config(&self) -> &MonitorConfig {
        return &self.config;
    }

    pub fn current_workspace_idx(&self) -> u32 {
        return self.cur_workspace;
    }

    pub fn current_workspace(&self) -> &Workspace<C> {
        return &self.workspaces[self.cur_workspace as usize];
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        return &mut self.workspaces[self.cur_workspace as usize];
    }

    pub fn move_to_workspace(&mut self, client_rc: Rc<RefCell<C>>, workspace_idx: u32) {
        if workspace_idx >= self.workspace_count() {
            return;
        }

        for ws in &mut self.workspaces.iter_mut() {
            if ws.contains(&client_rc) {
                ws.detach_client(&client_rc);
            }
        }

        if workspace_idx != self.cur_workspace {
            client_rc.borrow_mut().hide();
        }

        self.workspaces[workspace_idx as usize].attach_client(client_rc);
    }

    pub fn num(&self) -> u32 {
        return self.config.num();
    }

    pub fn restack_current(&self) {
        self.workspaces[self.cur_workspace as usize].restack();
    }

    pub fn workspace_count(&self) -> u32 {
        return self.workspaces.len() as u32;
    }

    pub fn switch_prev_workspace(&mut self, backend: &impl Backend<C>) {
        self.switch_workspace(backend, self.prev_workspace);
    }

    pub fn switch_workspace(&mut self, backend: &impl Backend<C>, workspace_idx: u32) {
        if workspace_idx == self.cur_workspace {
            return;
        }

        // transfer pinned clients
        let pinned_clients = self.workspaces[self.cur_workspace as usize].pull_pinned();
        pinned_clients.iter().for_each(|c| c.borrow().export_workspace(workspace_idx));
        self.workspaces[workspace_idx as usize].push_pinned(pinned_clients);

        // show and hide clients accordingly
        self.workspaces[self.cur_workspace as usize].clients()
            .for_each(|c| c.borrow_mut().hide());
        self.workspaces[workspace_idx as usize].clients().for_each(|c| c.borrow_mut().show());

        // set new workspace index
        self.prev_workspace = self.cur_workspace;
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
        self.workspaces[self.cur_workspace as usize].attach_client(client_rc);
    }

    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_> {
        return Box::new(self.workspaces.iter().flat_map(|ws| ws.clients()));
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
