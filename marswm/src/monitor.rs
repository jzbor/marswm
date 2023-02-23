use libmars::common::*;
use libmars::wm::{ Backend, Client };
use std::cell::RefCell;
use std::rc::Rc;

use crate::*;
use crate::config::Configuration;
use crate::workspace::*;

pub struct Monitor<C: Client<Attributes>> {
    config: MonitorConfig,
    workspaces: Vec<Workspace<C>>,
    cur_workspace: u32,
    prev_workspace: u32,
    workspace_offset: u32,
}

impl<C: Client<Attributes>> Monitor<C> {
    pub fn new(monitor_config: MonitorConfig, config: &Configuration, primary: bool, workspace_offset: u32)
                -> Monitor<C> {

        let workspaces: Vec<Workspace<C>> = if primary {
            (0..config.primary_workspaces)
                .map(|i| Workspace::new((i + 1).to_string(), workspace_offset + i,  monitor_config.window_area(), config.layout))
                .collect()
        } else {
            (0..config.secondary_workspaces)
                .map(|i| {
                    let name = if config.secondary_workspaces == 1 {
                        monitor_config.name().to_owned()
                    } else {
                        format!("{}:{}", monitor_config.name(), i + 1)
                    };
                    Workspace::new(name, workspace_offset + i, monitor_config.window_area(), config.layout)
                })
                .collect()
        };

        return Monitor {
            config: monitor_config,
            workspaces,
            cur_workspace: 0,
            prev_workspace: 0,
            workspace_offset,
        };
    }

    pub fn config(&self) -> &MonitorConfig {
        return &self.config;
    }

    pub fn current_workspace(&self) -> &Workspace<C> {
        return &self.workspaces[self.cur_workspace as usize];
    }

    pub fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        return &mut self.workspaces[self.cur_workspace as usize];
    }

    pub fn dimensions(&self) -> Dimensions {
        return self.config.dimensions();
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

    pub fn restack_current(&self) {
        self.workspaces[self.cur_workspace as usize].restack();
    }

    pub fn workspace_count(&self) -> u32 {
        return self.workspaces.len() as u32;
    }

    pub fn workspace_offset(&self) -> u32 {
        return self.workspace_offset
    }

    pub fn workspace(&self, index: u32) -> Option<&Workspace<C>> {
        return self.workspaces.get(index as usize);
    }

    pub fn workspace_mut(&mut self, index: u32) -> Option<&mut Workspace<C>> {
        return self.workspaces.get_mut(index as usize);
    }

    pub fn switch_prev_workspace(&mut self, backend: &impl Backend<Attributes>) {
        self.switch_workspace(backend, self.prev_workspace);
    }

    pub fn switch_workspace(&mut self, backend: &impl Backend<Attributes>, workspace_idx: u32) {
        if workspace_idx == self.cur_workspace {
            return;
        } else if workspace_idx >= self.workspace_count() {
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
        backend.export_current_workspace(self.workspaces[self.cur_workspace as usize].global_index());
    }

    pub fn update_config(&mut self, config: MonitorConfig) {
        self.config = config;
        for ws in &mut self.workspaces {
            ws.update_window_area(self.config.window_area());
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

impl<C: Client<Attributes>> ClientList<C> for Monitor<C> {
    fn attach_client(&mut self, client_rc: Rc<RefCell<C>>) {
        let workspace = &mut self.workspaces[self.cur_workspace as usize];
        client_rc.borrow().export_workspace(workspace.global_index());
        workspace.attach_client(client_rc);
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

impl<C: Client<Attributes>> PartialEq for Monitor<C> {
    fn eq(&self, other: &Self) -> bool {
        return self.config == other.config;
    }
}
