extern crate x11;

use std::ffi::*;
use x11::xlib;
use x11::xinerama;
use x11::xrandr;
use std::slice;

use crate::common::*;
use crate::common::x11::atoms::X11Atom;

pub mod atoms;
pub mod window;


pub const XLIB_NONE: u64 = 0;
pub const BUTTONMASK: i64 = xlib::ButtonPressMask | xlib::ButtonReleaseMask;
pub const MOUSEMASK: i64 = BUTTONMASK | xlib::PointerMotionMask;
pub const WINDOW_MIN_SIZE: u32 = 40;
// Cursor selectors (see cursorfont.h)
pub const CURSOR_NORMAL: u32 = 68;
pub const CURSOR_RESIZE: u32 = 120;
pub const CURSOR_MOVE: u32 = 52;
// Window states
pub const WITHDRAWN_STATE: i32 = 0;
pub const NORMAL_STATE: i32 = 1;
pub const ICONIC_STATE: i32 = 3;
// Motif hints
pub const MWM_HINTS_FLAGS_FIELD: usize = 0;
pub const MWM_HINTS_DECORATIONS_FIELD: usize = 2;
pub const MWM_HINTS_DECORATIONS: u64 = 1 << 1;
pub const MWM_DECOR_ALL: u64 = 1 << 0;
pub const MWM_DECOR_BORDER: u64 = 1 << 1;
pub const MWM_DECOR_TITLE: u64 = 1 << 3;


impl From<xinerama::XineramaScreenInfo> for MonitorConfig {
    fn from(info: xinerama::XineramaScreenInfo) -> MonitorConfig {
        let area = Dimensions { x: info.x_org.into(), y: info.y_org.into(),
                                w: info.width.try_into().unwrap(), h: info.height.try_into().unwrap() };
        MonitorConfig {
            name: format!("output{}", info.screen_number),
            dims: area,
            win_area: area,
        }
    }
}

impl From<(*mut xrandr::XRROutputInfo, *mut xrandr::XRRCrtcInfo)> for MonitorConfig {
    fn from((output_info, crtc_info): (*mut xrandr::XRROutputInfo, *mut xrandr::XRRCrtcInfo)) -> Self {
        let name = unsafe {
            let name_bytes = slice::from_raw_parts((*output_info).name as *mut u8, (*output_info).nameLen as usize);
            String::from_utf8_lossy(name_bytes).to_string()
        };
        let area = unsafe {
            Dimensions { x: (*crtc_info).x, y: (*crtc_info).y, w: (*crtc_info).width, h: (*crtc_info).height }
        };

        MonitorConfig {
            name,
            dims: area,
            win_area: area,
        }
    }

}

impl From<*mut xlib::Screen> for MonitorConfig {
    fn from(screen: *mut xlib::Screen) -> Self {
        let w = unsafe { xlib::XWidthOfScreen(screen).try_into().unwrap() };
        let h = unsafe { xlib::XHeightOfScreen(screen).try_into().unwrap() };
        let dims = Dimensions::new(0, 0, w, h);

        return MonitorConfig { name: "output".to_owned(), dims, win_area: dims };
    }
}

pub fn get_keysym(name: &str) -> xlib::KeySym {
    unsafe {
        let cstring = CString::new(name).unwrap();
        return xlib::XStringToKeysym(cstring.as_ptr());
    }
}

extern "C" fn on_error_dummy(_display: *mut xlib::Display, _error: *mut xlib::XErrorEvent) -> c_int {
    return 0;
}

pub fn query_monitor_config(display: *mut xlib::Display, ignore_overlapping: bool) -> Vec<MonitorConfig> {
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

                monitors.push(MonitorConfig::from((output_info, crtc_info)));
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
            return vec!(MonitorConfig::from(xlib::XDefaultScreenOfDisplay(display)));
        }

        if ignore_overlapping {
            let no_overlap = |m1: &MonitorConfig, m2: &MonitorConfig| {
                m1.dimensions().right() <= m2.dimensions().x()  // m1 is left of m2
                    || m1.dimensions().x() >= m2.dimensions().right()  // m1 is right of m2
                    || m1.dimensions().bottom() <= m2.dimensions().y()  // m1 is on top of m2
                    || m1.dimensions().y() >= m2.dimensions().bottom()  // m1 is below m2
            };
            let mut non_overlapping = Vec::new();
            for mon in monitors.drain(..) {
                if non_overlapping.iter().all(|m| no_overlap(m, &mon)) {
                    non_overlapping.push(mon);
                }
            }
            return non_overlapping;
        } else {
            return monitors;
        }
    }
}

pub fn sanitize_modifiers(modifiers: u32) -> u32 {
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
