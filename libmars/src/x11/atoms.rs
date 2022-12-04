extern crate x11;

use std::fmt::{Display, Formatter, Result};
use x11::xlib;

use std::ffi::CString;


#[derive(Clone, Copy)]
pub enum X11Atom {
    WMDeleteWindow,
    WMProtocols,
    WMState,

    NetActiveWindow,
    NetClientList,
    NetSupported,
}

impl Display for X11Atom {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let string = match self {
            X11Atom::WMDeleteWindow => "WM_DELETE_WINDOW",
            X11Atom::WMProtocols => "WM_PROTOCOLS",
            X11Atom::WMState => "WM_STATE",

            X11Atom::NetActiveWindow => "_NET_ACTIVE_WINDOW",
            X11Atom::NetClientList => "_NET_CLIENT_LIST",
            X11Atom::NetSupported => "_NET_SUPPORTED",
        };
        return write!(f, "{}", string);
    }
}

impl X11Atom {
    pub fn to_xlib_atom(&self, display: *mut xlib::Display) -> xlib::Atom {
        let atom_name = CString::new(self.to_string()).unwrap().into_raw();
        unsafe {
            return xlib::XInternAtom(display, atom_name, xlib::False);
        }
    }
}

