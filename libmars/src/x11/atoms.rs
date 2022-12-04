extern crate x11;

use x11::xlib;

use std::ffi::CString;


#[derive(Clone, Copy)]
pub enum X11Atom {
    WMDeleteWindow,
    WMProtocols,
    WMState,

    NetSupported,
}

pub fn xatom(display: *mut xlib::Display , atom: X11Atom) -> xlib::Atom {
    let atom_name = atom_to_str(atom).into_raw();
    unsafe {
        return xlib::XInternAtom(display, atom_name, xlib::False);
    }
}

fn atom_to_str(atom: X11Atom) -> CString {
    return match atom {
        X11Atom::WMDeleteWindow => CString::new("WM_DELETE_WINDOW"),
        X11Atom::WMProtocols => CString::new("WM_PROTOCOLS"),
        X11Atom::WMState => CString::new("WM_STATE"),
        X11Atom::NetSupported => CString::new("_NET_SUPPORTED"),
    }.expect("Atom name creation failed");
}

