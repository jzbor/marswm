//! A very basic toolkit for creating simple applications and drawing on windows.
//!
//! Currently only X11 is supported.

use std::ffi::*;
use std::mem::MaybeUninit;
use std::ptr;
use x11::xlib;

use crate::draw::canvas::*;
use crate::common::x11::atoms::X11Atom::*;
use crate::common::x11::atoms::X11Atom;
use crate::common::x11::window::X11Window;

pub mod canvas;
pub mod widget;

/// Waits for MapNotify on the specified window.
/// Discards all events before the MapNotify.
pub fn await_map_notify(display: *mut xlib::Display, window: xlib::Window) {
    loop {
        let mut event: MaybeUninit<xlib::XEvent> = MaybeUninit::uninit();
        unsafe {
            xlib::XNextEvent(display, event.as_mut_ptr());
            let event = event.assume_init();
            if event.get_type() == xlib::MapNotify
                    && event.map.window == window {
                break;
            }
        }
    }
}

pub fn close_display(display: *mut xlib::Display) {
    unsafe {
        xlib::XCloseDisplay(display);
    }
}

pub fn create_window(display: *mut xlib::Display, x: i32, y: i32, width: u32, height: u32,
                     class: &str, name: &str, win_type: Option<X11Atom>) -> Result<xlib::Window, String> {
    unsafe {
        let screen = xlib::XDefaultScreen(display);
        let border_width = 0;

        let win = xlib::XCreateSimpleWindow(display, xlib::XDefaultRootWindow(display),
                                       x, y, width, height, border_width,
                                       xlib::XBlackPixel(display, screen),
                                       xlib::XWhitePixel(display, screen));

        // subscribe to StructureNotifyMask for MapNotify events
        // subscribe to ExposureMask for Expose events
        xlib::XSelectInput(display, win, xlib::StructureNotifyMask | xlib::ExposureMask);

        // set class hint
        let class_cstring = match CString::new(class) {
            Ok(cstring) => cstring,
            Err(_) => return Err(format!("unable to convert class '{}' to C compatible string", class)),
        };
        let mut class_bytes = class_cstring.into_bytes_with_nul();
        let mut class_hint = xlib::XClassHint {
            res_name: class_bytes.as_mut_ptr() as *mut i8,
            res_class: class_bytes.as_mut_ptr() as *mut i8,
        };
        xlib::XSetClassHint(display, win, &mut class_hint);

        // set window title
        let name_cstring = match CString::new(name) {
            Ok(cstring) => cstring,
            Err(_) => return Err(format!("unable to convert name '{}' to C compatible string", class)),
        };
        let mut name_property: MaybeUninit<xlib::XTextProperty> = MaybeUninit::uninit();
        let mut data = [name_cstring.as_ptr() as *mut i8];
        if xlib::XStringListToTextProperty(data.as_mut_ptr(),
        1, name_property.as_mut_ptr()) == 0 {
            return Err(format!("unable to convert name to text property: '{}'", name));
        };
        xlib::XSetWMName(display, win, name_property.assume_init_mut());

        // set window type if requested
        if let Some(win_type) = win_type {
            let data = [win_type.to_xlib_atom(display)];
            win.x11_replace_property_long(display, NetWMWindowType, xlib::XA_ATOM, &data);
        }

        // make window visible on screen
        xlib::XMapWindow(display, win);

        // sync requests with x11 server
        xlib::XFlush(display);

        return Ok(win);
    }
}

pub fn expose_loop(display: *mut xlib::Display, canvas: &mut Canvas, draw: fn(&mut Canvas)) {
    loop {
        let mut event: MaybeUninit<xlib::XEvent> = MaybeUninit::uninit();
        unsafe {
            xlib::XNextEvent(display, event.as_mut_ptr());
            let event = event.assume_init();
            if event.get_type() == xlib::Expose
                    && event.expose.window == canvas.window() {
                draw(canvas);
                canvas.flush();
            }
        }
    }
}

pub fn open_display() -> Result<*mut xlib::Display, String> {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err("unable to connect to X server".to_owned());
        } else {
            return Ok(display);
        }
    }
}
