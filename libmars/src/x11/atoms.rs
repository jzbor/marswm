extern crate x11;

use std::fmt::{Display, Formatter, Result};
use x11::xlib;

use std::ffi::CString;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum X11Atom {
    WMDeleteWindow,
    WMProtocols,
    WMState,

    NetActiveWindow,
    NetClientList,
    NetClientListStacking,
    NetCurrentDesktop,
    NetDesktopNames,
    NetNumberOfDesktops,
    NetSupported,
    NetWMDesktop,
    NetWMWindowType,
    NetWMWindowTypeDock,
    NetWMWindowTypeDesktop,
}

const ATOMS: &'static [X11Atom; 14] = & [
    X11Atom::WMDeleteWindow,
    X11Atom::WMProtocols,
    X11Atom::WMState,

    X11Atom::NetActiveWindow,
    X11Atom::NetClientList,
    X11Atom::NetClientListStacking,
    X11Atom::NetCurrentDesktop,
    X11Atom::NetDesktopNames,
    X11Atom::NetNumberOfDesktops,
    X11Atom::NetSupported,
    X11Atom::NetWMDesktop,
    X11Atom::NetWMWindowType,
    X11Atom::NetWMWindowTypeDock,
    X11Atom::NetWMWindowTypeDesktop,
];

impl Display for X11Atom {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let string = match self {
            X11Atom::WMDeleteWindow => "WM_DELETE_WINDOW",
            X11Atom::WMProtocols => "WM_PROTOCOLS",
            X11Atom::WMState => "WM_STATE",

            X11Atom::NetActiveWindow => "_NET_ACTIVE_WINDOW",
            X11Atom::NetClientList => "_NET_CLIENT_LIST",
            X11Atom::NetClientListStacking => "_NET_CLIENT_LIST_STACKING",
            X11Atom::NetCurrentDesktop => "_NET_CURRENT_DESKTOP",
            X11Atom::NetDesktopNames => "_NET_DESKTOP_NAMES",
            X11Atom::NetNumberOfDesktops => "_NET_NUMBER_OF_DESKTOPS",
            X11Atom::NetSupported => "_NET_SUPPORTED",
            X11Atom::NetWMDesktop => "_NET_WM_DESKTOP",
            X11Atom::NetWMWindowType => "_NET_WM_WINDOW_TYPE",
            X11Atom::NetWMWindowTypeDock => "_NET_WM_WINDOW_TYPE_DOCK",
            X11Atom::NetWMWindowTypeDesktop => "_NET_WM_WINDOW_TYPE_DESKTOP",
        };
        return write!(f, "{}", string);
    }
}

impl X11Atom {
    pub fn from_xlib_atom(display: *mut xlib::Display, atom: xlib::Atom) -> Option<X11Atom> {
        let name = unsafe {
            let raw_string = xlib::XGetAtomName(display, atom);
            CString::from_raw(raw_string).into_string().unwrap()
        };
        for atom in ATOMS {
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

