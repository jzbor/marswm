extern crate x11;

use std::collections::VecDeque;
use std::ffi::*;
use std::mem::MaybeUninit;
use std::ptr;
use std::slice;
use x11::xinerama;
use x11::xlib;
use x11::xrandr;

use crate::common::error::*;
use crate::common::*;
use crate::common::x11::atoms::*;
use crate::common::x11::atoms::X11Atom::*;
use crate::common::x11::window::*;

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

impl From<(*mut xlib::Display, &xrandr::XRRMonitorInfo)> for MonitorConfig {
    fn from((display, monitor_info): (*mut xlib::Display, &xrandr::XRRMonitorInfo)) -> Self {
        let name = match atom_name(display, monitor_info.name) {
            Some(name) => name,
            None => format!("monitor{}", monitor_info.noutput),
        };
        let area = Dimensions { x: monitor_info.x, y: monitor_info.y,
                                w: monitor_info.width as u32, h: monitor_info.height as u32 };

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

/// Close an X11 connection
pub fn close_display(display: *mut xlib::Display) {
    if !display.is_null() {
        unsafe {
            xlib::XCloseDisplay(display);
        }
    }
}

/// Creates and maps a top-level window to be used by an application
pub fn create_window(display: *mut xlib::Display, dimensions: Dimensions, class: &str, name: &str,
                     win_type: Option<X11Atom>) -> Result<xlib::Window> {
    let (x, y, width, height) = dimensions.as_tuple();
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
            Err(_) => return Err(MarsError::failed_conversion(class, stringify!(&str), stringify!(CString))),
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
            Err(_) => return Err(MarsError::failed_conversion(name, stringify!(&str), stringify!(CString))),
        };
        let mut name_property: MaybeUninit<xlib::XTextProperty> = MaybeUninit::uninit();
        let mut data = [name_cstring.as_ptr() as *mut i8];
        if xlib::XStringListToTextProperty(data.as_mut_ptr(),
        1, name_property.as_mut_ptr()) == 0 {
            return Err(MarsError::failed_conversion(name, stringify!(&str), stringify!(xlib::XTextProperty)));
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

/// Open a new X11 connection
pub fn open_display() -> Result<*mut xlib::Display> {
    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            return Err(MarsError::x11_open_display());
        } else {
            return Ok(display);
        }
    }
}

/// Convert key names to [xlib::KeySym]
///
/// * `name` - Name as defined in `X11/keysymdef.h` (without the 'XK_' prefix)
pub fn get_keysym(name: &str) -> xlib::KeySym {
    unsafe {
        let cstring = CString::new(name).unwrap();
        return xlib::XStringToKeysym(cstring.as_ptr());
    }
}

pub extern "C" fn on_error_dummy(_display: *mut xlib::Display, _error: *mut xlib::XErrorEvent) -> c_int {
    return 0;
}

/// Get the current monitor configuration
///
/// * `ignore_overlapping` - Ignore monitors that overlap with other monitors
pub fn query_monitor_config(display: *mut xlib::Display, ignore_overlapping: bool) -> Vec<MonitorConfig> {
    unsafe {
        let mut monitors = VecDeque::new();

        // try xrandr
        let mut nmonitors = 0;
        let root = xlib::XDefaultRootWindow(display);
        let monitors_ptr = xrandr::XRRGetMonitors(display, root, xlib::True, &mut nmonitors);
        if !monitors_ptr.is_null() && nmonitors > 0 {
            let xrr_monitors = slice::from_raw_parts(monitors_ptr, nmonitors as usize);
            for xrr_monitor in xrr_monitors {
                if xrr_monitor.primary != 0 {
                    monitors.push_front(MonitorConfig::from((display, xrr_monitor)));
                } else {
                    monitors.push_back(MonitorConfig::from((display, xrr_monitor)));
                }
            }
        }
        xlib::XFree(monitors_ptr as *mut c_void);

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
            return monitors.into();
        }
    }
}

/// Remove unrelated mask bits on button or key events
pub fn sanitize_modifiers(modifiers: u32) -> u32 {
    return modifiers & (xlib::ShiftMask | xlib::ControlMask | xlib::Mod1Mask | xlib::Mod2Mask
                        | xlib::Mod3Mask | xlib::Mod4Mask |xlib::Mod5Mask);
}

/// Send a ClientMessage to the default root window
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
