extern crate x11;

use std::ffi::*;
use x11::xlib;
use x11::xinerama;
use x11::xrandr;
use std::slice;

use crate::*;
use crate::x11::atoms::X11Atom;
use crate::x11::atoms::X11Atom::*;

pub mod backend;
pub mod window;
pub mod atoms;
pub mod draw;
mod client;

pub const XLIB_NONE: u64 = 0;
const BUTTONMASK: i64 = xlib::ButtonPressMask | xlib::ButtonReleaseMask;
const MOUSEMASK: i64 = BUTTONMASK | xlib::PointerMotionMask;
const WINDOW_MIN_SIZE: u32 = 40;
// Cursor selectors (see cursorfont.h)
pub const CURSOR_NORMAL: u32 = 68;
pub const CURSOR_RESIZE: u32 = 120;
pub const CURSOR_MOVE: u32 = 52;
// Window states
pub const WITHDRAWN_STATE: i32 = 0;
pub const NORMAL_STATE: i32 = 1;
pub const ICONIC_STATE: i32 = 3;
// Motif hints
const MWM_HINTS_FLAGS_FIELD: usize = 0;
const MWM_HINTS_DECORATIONS_FIELD: usize = 2;
const MWM_HINTS_DECORATIONS: u64 = 1 << 1;
const MWM_DECOR_ALL: u64 = 1 << 0;
const MWM_DECOR_BORDER: u64 = 1 << 1;
const MWM_DECOR_TITLE: u64 = 1 << 3;


impl From<xinerama::XineramaScreenInfo> for MonitorConfig {
    fn from(info: xinerama::XineramaScreenInfo) -> MonitorConfig {
        let area = Dimensions { x: info.x_org.into(), y: info.y_org.into(),
                                w: info.width.try_into().unwrap(), h: info.height.try_into().unwrap() };
        MonitorConfig {
            dims: area,
            win_area: area,
        }
    }
}


impl From<xrandr::XRRCrtcInfo> for MonitorConfig {
    fn from(info: xrandr::XRRCrtcInfo) -> Self {
        let area = Dimensions { x: info.x, y: info.y, w: info.width, h: info.height };
        MonitorConfig {
            dims: area,
            win_area: area,
        }
    }

}


pub fn get_keysym(name: &str) -> xlib::KeySym {
    unsafe {
        let cstring = CString::new(name).unwrap();
        return xlib::XStringToKeysym(cstring.as_ptr());
    }
}

extern "C" fn on_wm_detected(_: *mut xlib::Display, _: *mut xlib::XErrorEvent) -> c_int {
    panic!("Another Window Manager seems to be running already");
}

extern "C" fn on_error(display: *mut xlib::Display, error: *mut xlib::XErrorEvent) -> c_int {
    let msg = unsafe {
        let bufsize = 1024;
        let mut buf = vec![0u8; bufsize];
        xlib::XGetErrorText(display, (*error).error_code.into(), buf.as_mut_ptr() as *mut i8,
                            (bufsize - 1) as c_int);
        let msg_cstring = CStr::from_ptr(buf.as_mut_ptr() as *mut i8);
        msg_cstring.to_str().unwrap().to_owned()
        // println!("{}", msg);
    };

    unsafe {
        match (*error).error_code {
            xlib::Success => println!("X11 non-error: {} (request code: {})", msg, (*error).request_code),
            xlib::BadMatch => println!("X11 error: {} (request code: {})", msg, (*error).request_code),
            xlib::BadWindow => println!("X11 error: {} (request code: {})", msg, (*error).request_code),
            _ => panic!("Fatal X11 error: {} (request code: {})", msg, (*error).request_code),
        }
    }

    // unsafe {
    //     match (*error).error_code {
    //         // @TODO add error types
    //         xlib::Success => println!("everything's okay"),
    //         xlib::BadRequest => panic!("bad request code"),
    //         xlib::BadValue => panic!("int parameter out of range"),
    //         xlib::BadWindow => println!("parameter not a Window"),
    //         xlib::BadPixmap => panic!("parameter not a Pixmap"),
    //         xlib::BadAtom => panic!("parameter not an Atom"),
    //         xlib::BadCursor => panic!("parameter not a Cursor"),
    //         xlib::BadFont => panic!("parameter not a Font"),
    //         xlib::BadMatch => panic!("parameter mismatch"),
    //         xlib::BadDrawable => panic!("parameter not a Pixmap or Window"),
    //         xlib::BadAccess => panic!("depending on context (see X.h)"),
    //         xlib::BadAlloc => panic!("insufficient resources"),
    //         xlib::BadColor => panic!("no such colormap"),
    //         xlib::BadGC => panic!("parameter not a GC"),
    //         xlib::BadIDChoice => panic!("choice not in range or already used"),
    //         xlib::BadName => panic!("font or color name doesn't exist"),
    //         xlib::BadLength => panic!("Request length incorrect"),
    //         xlib::BadImplementation => panic!("server is defective"),
    //         _ => panic!("unknown error occured"),
    //     }
    // }

    return 0;
}

extern "C" fn on_error_dummy(_display: *mut xlib::Display, _error: *mut xlib::XErrorEvent) -> c_int {
    return 0;
}

pub fn query_monitor_config(display: *mut xlib::Display) -> Vec<MonitorConfig> {
    unsafe {
        let mut monitors = Vec::new();

        // try xrandr
        let screen_res = xrandr::XRRGetScreenResources(display, xlib::XDefaultRootWindow(display));
        if !screen_res.is_null() {
            let outputs = slice::from_raw_parts((*screen_res).outputs, (*screen_res).noutput.try_into().unwrap());
            for output in outputs {
                let output_info = xrandr::XRRGetOutputInfo(display, screen_res, *output);
                if output_info.is_null() || (*output_info).connection as i32 != xrandr::RR_Connected
                        || (*output_info).crtc == 0 {
                    xrandr::XRRFreeOutputInfo(output_info);
                    continue;
                }

                let crtc_info = xrandr::XRRGetCrtcInfo(display, screen_res, (*output_info).crtc);
                if crtc_info.is_null() {
                    continue;
                }

                monitors.push(MonitorConfig::from(*crtc_info));
                xrandr::XRRFreeCrtcInfo(crtc_info);
                xrandr::XRRFreeOutputInfo(output_info);
            }
            xrandr::XRRFreeScreenResources(screen_res);
        }

        // try xinerama
        if monitors.is_empty() && xinerama::XineramaIsActive(display) != 0 {
            let mut screen_count = 0;
            let screens_raw = xinerama::XineramaQueryScreens(display, &mut screen_count);
            let screens_slice = slice::from_raw_parts_mut(screens_raw, screen_count.try_into().unwrap());
            monitors.extend(screens_slice.iter().map(|x| MonitorConfig::from(*x)));
            xlib::XFree(screens_slice.as_mut_ptr() as *mut c_void);
        }

        // use whole screen as fallback
        if monitors.is_empty() {
            let screen = xlib::XDefaultScreenOfDisplay(display);
            let w = xlib::XWidthOfScreen(screen).try_into().unwrap();
            let h = xlib::XHeightOfScreen(screen).try_into().unwrap();
            let dims = Dimensions { x: 0, y: 0, w, h };

            return vec![
                MonitorConfig { dims, win_area: dims }
            ];
         }

        return monitors;
    }
}

fn sanitize_modifiers(modifiers: u32) -> u32 {
    return modifiers & (xlib::ShiftMask | xlib::ControlMask | xlib::Mod1Mask | xlib::Mod2Mask
                        | xlib::Mod3Mask | xlib::Mod4Mask |xlib::Mod5Mask);
}

pub fn send_client_message(display: *mut xlib::Display, atom: X11Atom, window: xlib::Window, data: xlib::ClientMessageData) {
    let mut event = xlib::XEvent {
        client_message: xlib::XClientMessageEvent {
            type_: xlib::ClientMessage,
            serial: 0,
            send_event: xlib::True,
            display,
            window,
            message_type: atom.to_xlib_atom(display),
            format: 32,
            data,
        }
    };


    unsafe {
        let root = xlib::XDefaultRootWindow(display);
        let mask = xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask;
        let propagate = xlib::False;
        xlib::XSendEvent(display, root, propagate, mask, &mut event);
        xlib::XFlush(display);
    }
}

