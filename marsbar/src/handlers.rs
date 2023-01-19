extern crate x11;

use libmars::x11::atoms::X11Atom::*;
use libmars::x11::draw::widget::*;
use libmars::x11::send_client_message;
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
        if let WidgetEvent::ButtonPressed(_) = event {
            let mut data = ClientMessageData::new();
            data.set_long(0, self.workspace_idx as i64);
            send_client_message(self.display, NetCurrentDesktop, self.root, data);
            return true;
        } else {
            return false;
        }
    }
}
