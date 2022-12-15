extern crate x11;

use std::fmt::{Display, Formatter, Result};
use x11::xlib;
use std::ffi::CString;

use crate::*;

#[macro_export]
macro_rules! xatom {
    ($atom:expr, $display:expr) => { $atom.to_xlib_atom(display) }
}


enum_with_values! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    vis pub enum X11Atom {
        UTF8String,
        WMDeleteWindow,
        WMProtocols,
        WMState,

        NetActiveWindow,
        NetClientList,
        NetClientListStacking,
        NetCloseWindow,
        NetCurrentDesktop,
        NetDesktopNames,
        NetNumberOfDesktops,
        NetSupported,
        NetSupportingWMCheck,
        NetWMDesktop,
        NetWMName,
        NetWMState,
        NetWMStateFullscreen,
        NetWMWindowType,
        NetWMWindowTypeDock,
        NetWMWindowTypeDesktop,
        NetWMWindowTypeDialog,
        NetWMWindowTypeMenu,

        MotifWMHints
    }
}

impl Display for X11Atom {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let string = match self {
            X11Atom::UTF8String => "UTF8_STRING",
            X11Atom::WMDeleteWindow => "WM_DELETE_WINDOW",
            X11Atom::WMProtocols => "WM_PROTOCOLS",
            X11Atom::WMState => "WM_STATE",

            X11Atom::NetActiveWindow => "_NET_ACTIVE_WINDOW",
            X11Atom::NetClientList => "_NET_CLIENT_LIST",
            X11Atom::NetClientListStacking => "_NET_CLIENT_LIST_STACKING",
            X11Atom::NetCloseWindow => "_NET_CLOSE_WINDOW",
            X11Atom::NetCurrentDesktop => "_NET_CURRENT_DESKTOP",
            X11Atom::NetDesktopNames => "_NET_DESKTOP_NAMES",
            X11Atom::NetNumberOfDesktops => "_NET_NUMBER_OF_DESKTOPS",
            X11Atom::NetSupported => "_NET_SUPPORTED",
            X11Atom::NetSupportingWMCheck => "_NET_SUPPORTING_WM_CHECK",
            X11Atom::NetWMDesktop => "_NET_WM_DESKTOP",
            X11Atom::NetWMName => "_NET_WM_NAME",
            X11Atom::NetWMState => "_NET_WM_STATE",
            X11Atom::NetWMStateFullscreen => "_NET_WM_STATE_FULLSCREEN",
            X11Atom::NetWMWindowType => "_NET_WM_WINDOW_TYPE",
            X11Atom::NetWMWindowTypeDock => "_NET_WM_WINDOW_TYPE_DOCK",
            X11Atom::NetWMWindowTypeDesktop => "_NET_WM_WINDOW_TYPE_DESKTOP",
            X11Atom::NetWMWindowTypeDialog => "_NET_WM_WINDOW_TYPE_DIALOG",
            X11Atom::NetWMWindowTypeMenu => "_NET_WM_WINDOW_TYPE_MENU",

            X11Atom::MotifWMHints => "_MOTIF_WM_HINTS",
        };
        return write!(f, "{}", string);
    }
}

impl X11Atom {
    pub fn from_xlib_atom(display: *mut xlib::Display, atom: xlib::Atom) -> Option<X11Atom> {
        let name = unsafe {
            let raw_string = xlib::XGetAtomName(display, atom);
            // FIXME use CStr and XFree instead
            CString::from_raw(raw_string).into_string().unwrap()
        };
        for atom in Self::VALUES {
            if atom.to_string() == name {
                return Some(*atom);
            }
        }
        return None;
    }

    pub fn to_xlib_atom(&self, display: *mut xlib::Display) -> xlib::Atom {
        let atom_name = CString::new(self.to_string()).unwrap().into_raw();
        unsafe {
            return xlib::XInternAtom(display, atom_name, xlib::False);
        }
    }

    // TODO remove
    pub fn publish(display: *mut xlib::Display) {
        Self::VALUES.iter().for_each(|a| { a.to_xlib_atom(display); });
    }
}

