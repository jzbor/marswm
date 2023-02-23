extern crate x11;

use libmars::common::error::*;
use libmars::common::x11::atoms::X11Atom::*;
use libmars::common::x11::send_client_message;
use libmars::control::WMController;
use libmars::control::x11::X11Controller;
use libmars::draw::x11::widget::*;
use x11::xlib::ClientMessageData;
use x11::xlib;

pub struct StatusEventHandler {
    block: usize,
    command: String,
}

pub struct WorkspaceEventHandler {
    display: *mut xlib::Display,
    root: xlib::Window,
    workspace_idx: u32,
}


impl StatusEventHandler {
    pub fn new(block: usize, command: String) -> StatusEventHandler {
        return StatusEventHandler { block, command };
    }
}

impl WorkspaceEventHandler {
    pub fn new(display: *mut xlib::Display, workspace_idx: u32) -> WorkspaceEventHandler {
        let root = unsafe { xlib::XDefaultRootWindow(display) };
        return WorkspaceEventHandler { display, root, workspace_idx };
    }

    fn cycle_workspace(controller: &impl WMController<xlib::Window>, inc: i32) -> Result<()> {
        let nworkspaces = controller.count_workspaces()?;
        let current_workspace = controller.current_workspace()?;
        let new_workspace = ((current_workspace + nworkspaces) as i32 + inc) as u32 % nworkspaces;
        return controller.switch_workspace(new_workspace);
    }

    fn move_client(controller: &impl WMController<xlib::Window>, workspace: u32) -> Result<()> {
        let active = controller.get_active_window()?;
        return controller.send_window_to_workspace(active, workspace);
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
            if let Ok(controller) = X11Controller::new() {
                let result = match button {
                    1 => controller.switch_workspace(self.workspace_idx),
                    2 => Self::move_client(&controller, self.workspace_idx),
                    3 => controller.switch_workspace(self.workspace_idx),
                    4 => Self::cycle_workspace(&controller, -1),
                    5 => Self::cycle_workspace(&controller, 1),
                    _ => return false,
                };

                return result.is_ok()
            }
        }
        return false;
    }
}
