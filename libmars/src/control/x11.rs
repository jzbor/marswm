extern crate x11;

use std::ptr;
use std::ffi::*;
use x11::xlib::{self, XDefaultRootWindow};

use crate::common::error::*;
use crate::common::x11::atoms::X11Atom::*;
use crate::common::x11::atoms::X11Atom;
use crate::common::x11::send_client_message;
use crate::common::x11::window::X11Window;
use crate::control::*;

pub type EventHandlerFn = Option<unsafe extern "C" fn(_: *mut xlib::Display, _: *mut xlib::XErrorEvent) -> c_int>;

pub struct X11Controller {
    display: *mut xlib::Display,
    root: xlib::Window,
    event_handler: EventHandlerFn,
}

impl X11Controller {
    pub fn new_from_display(display: *mut xlib::Display, event_handler: EventHandlerFn) -> X11Controller {
        let root = unsafe { XDefaultRootWindow(display) };
        X11Controller {
            display, root, event_handler,
        }
    }

    pub fn new() -> Result<X11Controller> {
        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(MarsError::x11_open_display());
            }
            Ok(Self::new_from_display(display, None))
        }
    }
}


impl WMController<xlib::Window> for X11Controller {
    fn activate_window(&self, window: xlib::Window) -> Result<()> {
        require_ewmh_atom(self.display, NetActiveWindow)?;
        let data = xlib::ClientMessageData::new();
        send_client_message(self.display, NetActiveWindow, window, data);
        Ok(())
    }

    fn close_window(&self, window: xlib::Window) -> Result<()> {
        if require_ewmh_atom(self.display, NetCloseWindow).is_ok() {
            let data = xlib::ClientMessageData::new();
            send_client_message(self.display, NetCloseWindow, window, data);
        } else {
            window.x11_close(self.display, self.event_handler);
        }

        Ok(())
    }

    fn count_workspaces(&self) -> Result<u32> {
        require_ewmh_atom(self.display, NetNumberOfDesktops)?;
        let data = self.root.x11_read_property_long(self.display, NetNumberOfDesktops, xlib::XA_CARDINAL)
            .map_err(MarsError::unknown)?;
        let ws_u64 = data.first()
            .ok_or(MarsError::invalid_response("reading property _NET_NUMBER_OF_DESKTOPS"))?;
        (*ws_u64).try_into()
            .map_err(|_| MarsError::failed_conversion(ws_u64, stringify!(u64), stringify!(u32)))
    }

    fn current_workspace(&self) -> Result<u32> {
        require_ewmh_atom(self.display, NetCurrentDesktop)?;
        let data = self.root.x11_read_property_long(self.display, NetCurrentDesktop, xlib::XA_CARDINAL)
            .map_err(MarsError::unknown)?;
        let ws_u64 = data.first()
            .ok_or(MarsError::invalid_response("reading property _NET_CURRENT_DESKTOP"))?;
        (*ws_u64).try_into()
            .map_err(|_| MarsError::failed_conversion(ws_u64, stringify!(u64), stringify!(u32)))
    }

    fn fullscreen_window(&self, window: xlib::Window, mode: SettingMode) -> Result<()> {
        require_ewmh_atom(self.display, NetWMState)?;
        require_ewmh_atom(self.display, NetWMStateFullscreen)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, i64::from(mode));
        data.set_long(1, NetWMStateFullscreen.to_xlib_atom(self.display) as i64);
        send_client_message(self.display, NetWMState, window, data);
        Ok(())
    }

    fn get_active_window(&self) -> Result<xlib::Window> {
        require_ewmh_atom(self.display, NetActiveWindow)?;
        let data = self.root.x11_read_property_long(self.display, NetActiveWindow, xlib::XA_WINDOW)
                .map_err(MarsError::unknown)?;
        return data.first().copied()
            .ok_or(MarsError::invalid_response("reading property _NET_ACTIVE_WINDOW"));
    }

    fn get_workspace(&self, window: xlib::Window) -> Result<u32> {
        require_ewmh_atom(self.display, NetWMDesktop)?;

        if self.window_is_pinned(window)? {
            let data = window.x11_read_property_long(self.display, NetCurrentDesktop, xlib::XA_CARDINAL)
                .map_err(MarsError::unknown)?;
            let ws_u64 = data.first()
                .ok_or(MarsError::invalid_response("reading property _NET_WM_DESKTOP"))?;
            (*ws_u64).try_into()
                .map_err(|_| MarsError::failed_conversion(ws_u64, stringify!(u64), stringify!(u32)))
        } else {
            self.current_workspace()
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

        Ok(())
    }

    fn send_window_to_workspace(&self, window: xlib::Window, workspace: u32) -> Result<()> {
        require_ewmh_atom(self.display, NetWMDesktop)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, workspace.into());
        send_client_message(self.display, NetWMDesktop, window, data);
        Ok(())
    }

    fn set_status(&self, status: String) -> Result<()> {
        let data = vec!(status);
        self.root.x11_set_text_list_property(self.display, MarsStatus, &data);
        unsafe { xlib::XFlush(self.display); }
        Ok(())
    }

    fn switch_workspace(&self, workspace: u32) -> Result<()> {
        require_ewmh_atom(self.display, NetCurrentDesktop)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, workspace.into());
        send_client_message(self.display, NetCurrentDesktop, 0, data);
        Ok(())
    }

    fn tile_window(&self, window: xlib::Window, mode: SettingMode) -> Result<()> {
        require_ewmh_atom(self.display, NetWMState)?;
        require_ewmh_atom(self.display, MarsWMStateTiled)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, i64::from(mode));
        data.set_long(1, MarsWMStateTiled.to_xlib_atom(self.display) as i64);
        send_client_message(self.display, NetWMState, window, data);
        Ok(())
    }

    fn window_is_fullscreen(&self, window: xlib::Window) -> Result<bool> {
        require_ewmh_atom(self.display, NetWMState)?;
        let is_fullscreen = window.x11_net_wm_state(self.display)
            .map_err(MarsError::unknown)?
            .contains(&NetWMStateFullscreen);
        Ok(is_fullscreen)
    }

    fn window_is_pinned(&self, window: xlib::Window) -> Result<bool> {
        require_ewmh_atom(self.display, NetWMDesktop)?;
        let data = window.x11_read_property_long(self.display, NetWMDesktop, xlib::XA_CARDINAL)
            .map_err(MarsError::unknown)?;
        let ws_u64 = data.first()
            .ok_or(MarsError::invalid_response("reading property _NET_WM_DESKTOP"))?;
        Ok(*ws_u64 == u64::MAX)
    }

    fn window_is_tiled(&self, window: xlib::Window) -> Result<bool> {
        require_ewmh_atom(self.display, NetWMState)?;
        let is_fullscreen = window.x11_net_wm_state(self.display)
            .map_err(MarsError::unknown)?
            .contains(&MarsWMStateTiled);
        Ok(is_fullscreen)
    }

    fn workspaces(&self) -> Result<Vec<String>> {
        require_ewmh_atom(self.display, NetDesktopNames)?;
        self.root.x11_get_text_list_property(self.display, NetDesktopNames)
            .map_err(MarsError::unknown)
    }
}

impl From<SettingMode> for i64 {
    fn from(value: SettingMode) -> Self {
        match value {
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
        .map_err(|_| MarsError::x11_unsupported_atom(NetSupported))?;

    if supported.contains(&xatom) {
        Ok(())
    } else {
        Err(MarsError::x11_unsupported_atom(atom))
    }
}
