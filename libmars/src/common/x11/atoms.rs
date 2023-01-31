extern crate x11;

use std::fmt::{Display, Formatter, Result};
use x11::xlib;
use std::ffi::CString;

use crate::*;

enum_with_values! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    vis pub enum X11Atom {
        // ICCCM
        UTF8String,
        WMDeleteWindow,
        WMIconName,
        WMIconSize,
        WMName,
        WMProtocols,
        WMState,

        // EWMH
        NetActiveWindow,
        NetClientList,
        NetClientListStacking,
        NetCloseWindow,
        NetCurrentDesktop,
        NetDesktopGeometry,
        NetDesktopNames,
        NetDesktopViewport,
        NetNumberOfDesktops,
        NetSupported,
        NetSupportingWMCheck,
        NetSystemTray,
        NetSystemTrayOpcode,
        NetSystemTrayOrientation,
        NetWMDesktop,
        NetWMName,
        NetWMState,
        NetWMStateFullscreen,
        NetWMWindowType,
        NetWMWindowTypeDesktop,
        NetWMWindowTypeDialog,
        NetWMWindowTypeDock,
        NetWMWindowTypeMenu,
        NetWorkarea,

        // Motif
        MotifWMHints,

        // XEMBED
        Manager,
        Xembed,
        XembedInfo,

        // TODO add custom hints for pinned, tiled
        // MARS (custom)
        MarsWMStateTiled,
        MarsStatus
    }
}

impl Display for X11Atom {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let string = match self {
            X11Atom::UTF8String => "UTF8_STRING",
            X11Atom::WMDeleteWindow => "WM_DELETE_WINDOW",
            X11Atom::WMIconName => "WM_ICON_NAME",
            X11Atom::WMIconSize => "WM_ICON_SIZE",
            X11Atom::WMProtocols => "WM_PROTOCOLS",
            X11Atom::WMName => "WM_NAME",
            X11Atom::WMState => "WM_STATE",

            X11Atom::NetActiveWindow => "_NET_ACTIVE_WINDOW",
            X11Atom::NetClientList => "_NET_CLIENT_LIST",
            X11Atom::NetClientListStacking => "_NET_CLIENT_LIST_STACKING",
            X11Atom::NetCloseWindow => "_NET_CLOSE_WINDOW",
            X11Atom::NetCurrentDesktop => "_NET_CURRENT_DESKTOP",
            X11Atom::NetDesktopGeometry => "_NET_DESKTOP_GEOMETRY",
            X11Atom::NetDesktopNames => "_NET_DESKTOP_NAMES",
            X11Atom::NetDesktopViewport => "_NET_DESKTOP_VIEWPORT",
            X11Atom::NetNumberOfDesktops => "_NET_NUMBER_OF_DESKTOPS",
            X11Atom::NetSupported => "_NET_SUPPORTED",
            X11Atom::NetSupportingWMCheck => "_NET_SUPPORTING_WM_CHECK",
            X11Atom::NetSystemTray => "_NET_SYSTEM_TRAY_S0",
            X11Atom::NetSystemTrayOpcode => "_NET_SYSTEM_TRAY_OPCODE",
            X11Atom::NetSystemTrayOrientation => "_NET_SYSTEM_TRAY_ORIENTATION",
            X11Atom::NetWMDesktop => "_NET_WM_DESKTOP",
            X11Atom::NetWMName => "_NET_WM_NAME",
            X11Atom::NetWMState => "_NET_WM_STATE",
            X11Atom::NetWMStateFullscreen => "_NET_WM_STATE_FULLSCREEN",
            X11Atom::NetWMWindowType => "_NET_WM_WINDOW_TYPE",
            X11Atom::NetWMWindowTypeDock => "_NET_WM_WINDOW_TYPE_DOCK",
            X11Atom::NetWMWindowTypeDesktop => "_NET_WM_WINDOW_TYPE_DESKTOP",
            X11Atom::NetWMWindowTypeDialog => "_NET_WM_WINDOW_TYPE_DIALOG",
            X11Atom::NetWMWindowTypeMenu => "_NET_WM_WINDOW_TYPE_MENU",
            X11Atom::NetWorkarea => "_NET_WORKAREA",

            X11Atom::MotifWMHints => "_MOTIF_WM_HINTS",

            X11Atom::Manager => "MANAGER",
            X11Atom::Xembed => "_XEMBED",
            X11Atom::XembedInfo => "_XEMBED_INFO",

            X11Atom::MarsWMStateTiled => "_MARS_WM_STATE_TILED",
            X11Atom::MarsStatus => "_MARS_STATUS",
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
}

