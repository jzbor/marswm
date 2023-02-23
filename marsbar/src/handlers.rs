extern crate x11;

use libmars::common::error::*;
use libmars::control::WMController;
use libmars::control::x11::X11Controller;
use libmars::draw::x11::widget::*;

pub struct StatusEventHandler {
    block: usize,
    command: String,
}

pub struct WorkspaceEventHandler {
    controller: X11Controller,
    workspace_idx: u32,
}


impl StatusEventHandler {
    pub fn new(block: usize, command: String) -> StatusEventHandler {
        return StatusEventHandler { block, command };
    }
}

impl WorkspaceEventHandler {
    pub fn new(workspace_idx: u32) -> Result<WorkspaceEventHandler> {
        let controller = X11Controller::new()?;
        return Ok(WorkspaceEventHandler { controller, workspace_idx });
    }

    fn cycle_workspace(&self, inc: i32) -> Result<()> {
        let nworkspaces = self.controller.count_workspaces()?;
        let current_workspace = self.controller.current_workspace()?;
        let new_workspace = ((current_workspace + nworkspaces) as i32 + inc) as u32 % nworkspaces;
        return self.controller.switch_workspace(new_workspace);
    }

    fn move_client(&self) -> Result<()> {
        let active = self.controller.get_active_window()?;
        return self.controller.send_window_to_workspace(active, self.workspace_idx);
    }
}


impl WidgetEventHandler for StatusEventHandler {
    fn handle_action_event(&self, event: WidgetEvent, already_handled: bool) -> bool {
        if already_handled { return true; }
        if let WidgetEvent::ButtonPressed(button) = event {
            let block_var = format!("{}", self.block);
            let button_var = format!("{}", button);
            if let Ok(mut handle) = std::process::Command::new("sh").arg("-c").arg(&self.command)
                    .env("BLOCK", block_var).env("BUTTON", button_var).spawn() {
                std::thread::spawn(move || {
                    let _ignored = handle.wait();
                });
            }
            return true;
        } else {
            return false;
        }
    }
}

impl WidgetEventHandler for WorkspaceEventHandler {
    fn handle_action_event(&self, event: WidgetEvent, already_handled: bool) -> bool {
        if already_handled { return true; }
        if let WidgetEvent::ButtonPressed(button) = event {
            let result = match button {
                1 => self.controller.switch_workspace(self.workspace_idx),
                2 => self.move_client(),
                3 => self.controller.switch_workspace(self.workspace_idx),
                4 => self.cycle_workspace(-1),
                5 => self.cycle_workspace(1),
                _ => return false,
            };

            return result.is_ok()
        }
        return false;
    }
}
