extern crate x11;

use std::ffi::*;
use std::mem::MaybeUninit;
use std::ptr;
use std::slice;
use x11::xlib;

use crate::*;
use crate::x11::*;
use crate::x11::atoms::*;


pub trait X11Window {
    fn x11_attributes(&self, display: *mut xlib::Display) -> Result<xlib::XWindowAttributes, String>;
    fn x11_class_hint(&self, display: *mut xlib::Display) -> Result<(String, String), String>;
    fn x11_replace_property_long(&self, display: *mut xlib::Display, property: X11Atom, prop_type: c_ulong, data: &[c_ulong]);
    fn x11_set_text_list_property(&self, display: *mut xlib::Display, property: X11Atom, list: Vec<CString>);
    fn x11_dimensions(&self, display: *mut xlib::Display) -> Result<Dimensions, String>;
    fn x11_geometry(&self, display: *mut xlib::Display) -> Result<(u64, i32, i32, u32, u32, u32, u32), String>;
    fn x11_get_window_types(&self, display: *mut xlib::Display) -> Vec<xlib::Atom>;
    fn x11_is_transient_for(&self, display: *mut xlib::Display) -> Option<xlib::Window>;
    fn x11_message(&self, display: *mut xlib::Display, msg_type: atoms::X11Atom, msg_format: c_int, msg_data: xlib::ClientMessageData);
    fn x11_wm_protocols(&self, display: *mut xlib::Display) -> Vec<xlib::Atom>;
    fn x11_wm_normal_hints(&self, display: *mut xlib::Display) -> Result<(xlib::XSizeHints, c_long), String>;
}

impl X11Window for xlib::Window {
    fn x11_attributes(&self, display: *mut xlib::Display) -> Result<xlib::XWindowAttributes, String> {
        unsafe {
            let mut attributes: MaybeUninit<xlib::XWindowAttributes> = MaybeUninit::uninit();
            match xlib::XGetWindowAttributes(display, *self, attributes.as_mut_ptr()) {
                0 => return Err(String::from("Unable to retrieve attributes")),
                _ => return Ok(attributes.assume_init()),
            }
        }
    }

    fn x11_class_hint(&self, display: *mut xlib::Display) -> Result<(String, String), String> {
        unsafe {
            let mut class_hints: MaybeUninit<xlib::XClassHint> = MaybeUninit::zeroed();
            let status = xlib::XGetClassHint(display, *self, class_hints.as_mut_ptr());
            if status != 0 {
                let class_hints = class_hints.assume_init();
                if !class_hints.res_name.is_null() && !class_hints.res_class.is_null() {
                    let res_name = CStr::from_ptr(class_hints.res_name)
                        .to_str().unwrap().to_owned();
                    let res_class = CStr::from_ptr(class_hints.res_class)
                        .to_str().unwrap().to_owned();
                    let ret_val = Ok((res_name, res_class));
                    xlib::XFree(class_hints.res_name as *mut c_void);
                    xlib::XFree(class_hints.res_class as *mut c_void);
                    return ret_val;
                } else {
                    return Err("Class hint returned null ptr".to_owned());
                }
            } else {
                return Err("Error getting class hint from window".to_owned());
            }
        }
    }

    fn x11_replace_property_long(&self, display: *mut xlib::Display, property: X11Atom, prop_type: c_ulong, data: &[c_ulong]) {
        unsafe {
            xlib::XChangeProperty(display,
                                  *self,
                                  property.to_xlib_atom(display),
                                  prop_type,
                                  32,
                                  xlib::PropModeReplace,
                                  data.as_ptr().cast::<u8>(),
                                  data.len() as i32);
        }
    }

    fn x11_set_text_list_property(&self, display: *mut xlib::Display, property: X11Atom, list: Vec<CString>) {
        let mut pointers: Vec<*mut i8> = list.iter().map(|cstr| cstr.clone().into_raw()).collect();
        let slice = &mut pointers;
        unsafe {
            let mut text: MaybeUninit<xlib::XTextProperty> = MaybeUninit::uninit();
            let size = slice.len().try_into().unwrap();
            xlib::Xutf8TextListToTextProperty(display, slice.as_mut_ptr(), size, xlib::XUTF8StringStyle, text.as_mut_ptr());
            xlib::XSetTextProperty(display, *self, &mut text.assume_init(), property.to_xlib_atom(display));
        }
    }

    fn x11_dimensions(&self, display: *mut xlib::Display) -> Result<Dimensions, String> {
        return match self.x11_geometry(display) {
            Ok((_root, x, y, w, h, _bw, _depth)) => Ok(Dimensions { x, y, w, h }),
            Err(e) => Err(e),
        };
    }

    fn x11_geometry(&self, display: *mut xlib::Display) -> Result<(u64, i32, i32, u32, u32, u32, u32), String> {
        unsafe {
            let mut root: u64 = 0;
            let mut x: i32 = 0;
            let mut y: i32 = 0;
            let mut w: u32 = 0;
            let mut h: u32 = 0;
            let mut bw: u32 = 0;
            let mut depth: u32 = 0;
            match xlib::XGetGeometry(display, *self, &mut root, &mut x, &mut y, &mut w, &mut h, &mut bw, &mut depth) {
                0 => return Err(String::from("Unable to retrieve geometry")),
                _ => return Ok((root, x, y, w, h, bw, depth)),
            }
        }
    }

    fn x11_get_window_types(&self, display: *mut xlib::Display) -> Vec<xlib::Atom> {
        let mut types = Vec::new();

        let mut da = XLIB_NONE;
        let mut di = 0;
        let mut nitems = 0;
        let mut dl = 0;
        unsafe {
            let mut win_types_ptr: *mut u8 = ptr::null_mut();
            let result = xlib::XGetWindowProperty(display, *self, NetWMWindowType.to_xlib_atom(display),
                0, 8, xlib::False, xlib::XA_ATOM,
                &mut da, &mut di, &mut nitems, &mut dl, &mut win_types_ptr);
            if result == xlib::Success.into() {
                let temp = slice::from_raw_parts(win_types_ptr as *mut xlib::Atom, nitems.try_into().unwrap());
                types = temp.to_vec();
            }
        }
        return types;
    }

    fn x11_is_transient_for(&self, display: *mut xlib::Display) -> Option<xlib::Window> {
        unsafe {
            let mut window: xlib::Window = XLIB_NONE;
            match xlib::XGetTransientForHint(display, *self, &mut window) {
                0 => return None,
                _ => return Some(window),
            }
        }
    }

    fn x11_message(&self, display: *mut xlib::Display, msg_type: atoms::X11Atom, msg_format: c_int, msg_data: xlib::ClientMessageData) {
        unsafe {
            let msg_type_x11 = msg_type.to_xlib_atom(display);
            let msg_event = xlib::XClientMessageEvent {
                type_: xlib::ClientMessage,
                serial: 0,
                send_event: xlib::False,
                display,
                window: *self,
                message_type: msg_type_x11,
                format: msg_format,
                data: msg_data,
            };

            let mut event = xlib::XEvent { client_message: msg_event };

            xlib::XSendEvent(display, *self, xlib::False, 0, &mut event);
        }
    }

    fn x11_wm_protocols(&self, display: *mut xlib::Display) -> Vec<xlib::Atom> {
        let mut supported_atoms = Vec::new();
        unsafe {
            let mut atoms: *mut xlib::Atom = ptr::null_mut();
            let mut natoms: c_int = 0;
            xlib::XGetWMProtocols(display, *self, &mut atoms, &mut natoms);
            for n in slice::from_raw_parts(atoms, natoms as usize) {
                supported_atoms.push(*n);
            }
        }
        return supported_atoms;
    }

    fn x11_wm_normal_hints(&self, display: *mut xlib::Display) -> Result<(xlib::XSizeHints, c_long), String> {
        unsafe {
            let mut size_hints: MaybeUninit<xlib::XSizeHints> = MaybeUninit::uninit();
            let mut supplied_hints: c_long = 0;
            match xlib::XGetWMNormalHints(display, *self, size_hints.as_mut_ptr(), &mut supplied_hints) {
                0 => return Err(String::from("Unable to retrieve WMNormalHints")),
                _ => return Ok((size_hints.assume_init(), supplied_hints)),
            }
        }
    }
}
