extern crate x11;

use x11::xlib::{self, XDefaultRootWindow};

use crate::common::error::*;
use crate::common::x11::atoms::X11Atom::*;
use crate::common::x11::atoms::X11Atom;
use crate::common::x11::send_client_message;
use crate::common::x11::window::X11Window;
use crate::control::*;


pub struct X11Controller {
    display: *mut xlib::Display,
    root: xlib::Window,
}

impl X11Controller {
    pub fn new_from_display(display: *mut xlib::Display) -> X11Controller {
        let root = unsafe { XDefaultRootWindow(display) };
        return X11Controller {
            display, root,
        };
    }
}


impl WMController<xlib::Window> for X11Controller {
    fn current_workspace(&self) -> Result<u32> {
        require_ewmh_atom(self.display, NetCurrentDesktop)?;
        let data = self.root.x11_read_property_long(self.display, NetCurrentDesktop, xlib::XA_CARDINAL)
            .map_err(|msg| error_unknown(msg))?;
        let ws_u64 = data.get(0)
            .ok_or(error_invalid_response("reading property _NET_CURRENT_DESKTOP"))?;
        return (*ws_u64).try_into()
            .map_err(|_| error_failed_conversion(ws_u64, "u64", "u32"));
    }

    fn fullscreen_window(&self, window: xlib::Window, mode: SettingMode) -> Result<()> {
        require_ewmh_atom(self.display, NetWMStateFullscreen)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, i64::from(mode));
        data.set_long(1, NetWMStateFullscreen.to_xlib_atom(self.display) as i64);
        send_client_message(self.display, NetWMState, window, data);
        return Ok(());
    }

    fn get_workspace(&self, window: xlib::Window) -> Result<u32> {
        require_ewmh_atom(self.display, NetWMDesktop)?;

        if self.window_is_pinned(window)? {
            let data = window.x11_read_property_long(self.display, NetCurrentDesktop, xlib::XA_CARDINAL)
                .map_err(|msg| error_unknown(msg))?;
            let ws_u64 = data.get(0)
                .ok_or(error_invalid_response("reading property _NET_WM_DESKTOP"))?;
            return (*ws_u64).try_into()
                .map_err(|_| error_failed_conversion(ws_u64, "u64", "u32"));
        } else {
            return self.current_workspace();
        }
    }

    fn pin_window(&self, window: xlib::Window, mode: SettingMode) -> Result<()> {
        require_ewmh_atom(self.display, NetWMDesktop)?;
        match mode {
            SettingMode::Set => {
                let mut data = xlib::ClientMessageData::new();
                data.set_long(0, -1);
                send_client_message(self.display, NetWMDesktop, window, data);
            },
            SettingMode::Unset => {
                let current_workspace = self.current_workspace()?;
                self.send_window_to_workspace(window, current_workspace)?;
            },
            SettingMode::Toggle => {
                if self.window_is_pinned(window)? {
                    return self.pin_window(window, SettingMode::Unset);
                } else {
                    return self.pin_window(window, SettingMode::Set);
                }
            }
        }

        return Ok(());
    }

    fn send_window_to_workspace(&self, window: xlib::Window, workspace: u32) -> Result<()> {
        require_ewmh_atom(self.display, NetWMDesktop)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, workspace.into());
        send_client_message(self.display, NetWMDesktop, window, data);
        return Ok(());
    }

    fn switch_workspace(&self, workspace: u32) -> Result<()> {
        require_ewmh_atom(self.display, NetCurrentDesktop)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, workspace.into());
        send_client_message(self.display, NetCurrentDesktop, 0, data);
        return Ok(());
    }

    fn window_is_pinned(&self, window: xlib::Window) -> Result<bool> {
        require_ewmh_atom(self.display, NetWMDesktop)?;
        let data = window.x11_read_property_long(self.display, NetCurrentDesktop, xlib::XA_CARDINAL)
            .map_err(|msg| error_unknown(msg))?;
        let ws_u64 = data.get(0)
            .ok_or(error_invalid_response("reading property _NET_WM_DESKTOP"))?;
        return Ok(*ws_u64 == u64::MAX);
    }

    fn window_is_fullscreen(&self, window: xlib::Window) -> Result<bool> {
        require_ewmh_atom(self.display, NetWMState)?;
        let is_fullscreen = window.x11_net_wm_state(self.display)
            .map_err(|msg| error_unknown(msg))?
            .contains(&NetWMStateFullscreen);
        return Ok(is_fullscreen);
    }
}

impl From<SettingMode> for i64 {
    fn from(value: SettingMode) -> Self {
        return match value {
            SettingMode::Set => 1,
            SettingMode::Unset => 0,
            SettingMode::Toggle => 2,
        }
    }
}

fn require_ewmh_atom(display: *mut xlib::Display, atom: X11Atom) -> Result<()> {
    let root = unsafe {
        xlib::XDefaultRootWindow(display)
    };
    let xatom = atom.to_xlib_atom(display);
    let supported = root.x11_read_property_long(display, NetSupported, xlib::XA_ATOM)
        .map_err(|_| error_x11_unsupported_atom(NetSupported))?;

    if supported.contains(&xatom) {
        return Ok(());
    } else {
        return Err(error_x11_unsupported_atom(atom));
    }
}
