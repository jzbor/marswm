extern crate x11;

use x11::xlib;

use std::ffi::CString;


pub enum XAtom {
    WMDeleteWindow,
    WMProtocols,
    WMState,
}

pub fn xatom(display: *mut xlib::Display , atom: XAtom) -> xlib::Atom {
    let atom_name = atom_to_str(atom).into_raw();
    unsafe {
        return xlib::XInternAtom(display, atom_name, xlib::False);
    }
}

fn atom_to_str(atom: XAtom) -> CString {
    return match atom {
        XAtom::WMDeleteWindow => CString::new("WM_DELETE_WINDOW"),
        XAtom::WMProtocols => CString::new("WM_PROTOCOLS"),
        XAtom::WMState => CString::new("WM_STATE"),
    }.expect("Atom name creation failed");
}

