extern crate x11;

use x11::xlib;
use std::mem;

use std::slice;

use crate::*;
use crate::x11::*;
use crate::x11::atoms::*;


pub trait X11Window {
    fn x11_attributes(&self, display: *mut xlib::Display) -> Result<xlib::XWindowAttributes, String>;
    fn x11_replace_property_long(&self, display: *mut xlib::Display, property: X11Atom, prop_type: c_ulong, data: &[c_ulong]);
    fn x11_dimensions(&self, display: *mut xlib::Display) -> Result<Dimensions, String>;
    fn x11_geometry(&self, display: *mut xlib::Display) -> Result<(u64, i32, i32, u32, u32, u32, u32), String>;
    fn x11_is_transient_for(&self, display: *mut xlib::Display) -> Option<xlib::Window>;
    fn x11_message(&self, display: *mut xlib::Display, msg_type: atoms::X11Atom, msg_format: c_int, msg_data: xlib::ClientMessageData);
    fn x11_wm_protocols(&self, display: *mut xlib::Display) -> Vec<xlib::Atom>;
    fn x11_wm_normal_hints(&self, display: *mut xlib::Display) -> Result<(xlib::XSizeHints, c_long), String>;
}

#[derive(PartialEq)]
pub struct X11Client {
    display: *mut xlib::Display,
    root: u64,
    window: u64,
    frame: u64,

    x: i32, y: i32, // x, y position
    w: u32, h: u32, // width, height
    ibw: u32, obw: u32, // inner and outer border width
    fw: u32,        // frame width

    actively_reparenting: bool,
    visible: bool,
}

impl X11Client {
    pub fn new(display: *mut xlib::Display, root: u64, window: xlib::Window) -> X11Client {
        let attributes = window.x11_attributes(display)
            .expect("Unable to retrieve attributes for new client");
        let x = attributes.x;
        let y = attributes.y;
        let w: u32 = attributes.width.try_into().unwrap();
        let h: u32 = attributes.height.try_into().unwrap();

        // create frame window and reparent the original window
        let frame = unsafe {
            let frame = xlib:: XCreateSimpleWindow(display, root, x, y, w, h, 0, 0, 0);
            xlib::XSelectInput(display, frame,
                    xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask
                    | xlib::ButtonPressMask | xlib::PointerMotionMask | xlib::ButtonReleaseMask
                    | xlib::ButtonMotionMask | xlib::EnterWindowMask | xlib::LeaveWindowMask);
            xlib::XAddToSaveSet(display, window);
            xlib::XReparentWindow(display, window, frame, 0, 0);
            xlib::XResizeWindow(display, window, w, h);

            frame
        };

        return X11Client {
            display, root, window, frame,
            x, y, w, h,
            ibw: 0,
            obw: 0,
            fw: 0,

            actively_reparenting: false,
            visible: false,
        };
    }

    pub fn destroy_frame(&self) {
        unsafe {
            xlib::XUnmapWindow(self.display, self.frame);
            // The following lines were here before, but generate "parameter not a window" errors
            //xlib::XReparentWindow(self.display, self.window, self.root, 0, 0);
            //xlib::XRemoveFromSaveSet(self.display, self.window);
            xlib::XDestroyWindow(self.display, self.frame);
        }

    }

    pub fn frame(&self) -> u64 {
        return self.frame;
    }

    pub fn is_reparenting(&self) -> bool {
        return self.actively_reparenting;
    }

    pub fn set_reparenting(&mut self, status: bool) {
        self.actively_reparenting = status;
    }

    fn set_state(&self, state: i32) {
        let data = state;
        let state_atom = X11Atom::WMState.to_xlib_atom(self.display);

        unsafe {
            xlib::XChangeProperty(self.display, self.window, state_atom, state_atom,
                                  32, xlib::PropModeReplace, mem::transmute(&data), 1);
        }
    }

    fn supports_protocol(&self, atom: atoms::X11Atom) -> bool {
        let xatom = atom.to_xlib_atom(self.display);
        return self.window.x11_wm_protocols(self.display).contains(&xatom);
    }

    pub fn window(&self) -> u64 {
        return self.window;
    }
}

impl Client for X11Client {
    fn bind_button(&mut self, modifiers: u32, button: u32) {
        let mask: u32 = (xlib::ButtonPressMask | xlib::ButtonReleaseMask | xlib::ButtonMotionMask)
            .try_into().unwrap();
        unsafe {
            xlib::XGrabButton(self.display, button, modifiers, self.frame, xlib::False, mask,
                              xlib::GrabModeAsync, xlib::GrabModeAsync, self.frame, 0);
        }
    }

    fn bind_key(&mut self, modifiers: u32, key: u32) {
        unsafe {
            let keycode = xlib::XKeysymToKeycode(self.display, key.into());
            xlib::XGrabKey(self.display, keycode.into(), modifiers, self.frame, xlib::False,
                            xlib::GrabModeAsync, xlib::GrabModeAsync);
        }
    }

    fn close(&self) {
        if self.supports_protocol(X11Atom::WMDeleteWindow) {
            let msg_type = X11Atom::WMProtocols;
            let mut msg_data = xlib::ClientMessageData::new();
            msg_data.set_long(0, X11Atom::WMDeleteWindow.to_xlib_atom(self.display) as i64);
            self.x11_message(self.display, msg_type, 32, msg_data);
        } else {
            unsafe {
                xlib::XGrabServer(self.display);
                xlib::XSetErrorHandler(Some(on_error_dummy));
                xlib::XSetCloseDownMode(self.display, xlib::DestroyAll);
                xlib::XKillClient(self.display, self.window);
                xlib::XSync(self.display, xlib::False);
                xlib::XSetErrorHandler(Some(on_error));
                xlib::XUngrabServer(self.display);
            }
        }
    }

    fn hide(&mut self) {
        if !self.visible {
            return;
        }

        unsafe {
            xlib::XGrabServer(self.display);
            // @TODO better handling for unwrap
            let fa = self.frame.x11_attributes(self.display).unwrap();
            let ra = self.root.x11_attributes(self.display).unwrap();
            // prevent UnmapNotify events
            xlib::XSelectInput(self.display, self.root, ra.your_event_mask & !xlib::SubstructureNotifyMask);
            xlib::XSelectInput(self.display, self.frame, fa.your_event_mask & !xlib::StructureNotifyMask & !xlib::SubstructureNotifyMask);
            xlib::XUnmapWindow(self.display, self.frame);
            xlib::XUnmapWindow(self.display, self.window);
            // @TODO set IconicState (see moonwm - window_set_state(dpy, win, IconicState))
            self.set_state(IconicState);
            xlib::XSelectInput(self.display, self.root, ra.your_event_mask);
            xlib::XSelectInput(self.display, self.frame, fa.your_event_mask);
            xlib::XUngrabServer(self.display);
        }

        self.visible = false;
    }

    fn move_resize(&mut self, x: i32, y: i32, width: u32, height: u32) {
        let min_size = 2*self.obw + WINDOW_MIN_SIZE;
        if width < min_size {
            self.move_resize(x, y, min_size, height);
            return;
        } else if height < min_size {
            self.move_resize(x, y, width, min_size);
            return;
        }

        self.x = x;
        self.y = y;
        self.w = width;
        self.h = height;

        unsafe {
            xlib::XMoveResizeWindow(self.display, self.frame, self.x, self.y,
                                    self.w - 2*self.obw, self.h - 2 * self.obw);
            xlib::XMoveResizeWindow(self.display, self.window,
                                    self.fw.try_into().unwrap(), self.fw.try_into().unwrap(),
                                    self.w - 2*self.ibw - 2*self.fw - 2 * self.obw,
                                    self.h - 2*self.ibw - 2*self.fw - 2 * self.obw);
        }
    }

    fn raise(&self) {
        unsafe {
            xlib::XRaiseWindow(self.display, self.frame);
        }
    }

    fn set_frame_color(&mut self, color: u64) {
        unsafe {
            xlib::XSetWindowBackground(self.display, self.frame, color);
            xlib::XClearWindow(self.display, self.frame);
            xlib::XSync(self.display, xlib::False);
        }
    }

    fn set_frame_width(&mut self, width: u32) {
        self.fw = width;
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn set_height(&mut self, height: u32) {
        self.h = height;
    }

    fn set_inner_bw(&mut self, bw: u32) {
        self.ibw = bw;
        unsafe {
            xlib::XSetWindowBorderWidth(self.display, self.window, self.ibw);
        }
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn set_inner_color(&mut self, color: u64) {
        unsafe {
            xlib::XSetWindowBorder(self.display, self.window, color);
        }
    }

    fn set_outer_bw(&mut self, bw: u32) {
        self.obw = bw;
        unsafe {
            xlib::XSetWindowBorderWidth(self.display, self.frame, self.obw);
        }
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn set_outer_color(&mut self, color: u64) {
        unsafe {
            xlib::XSetWindowBorder(self.display, self.frame, color);
        }
    }

    fn show(&mut self) {
        if self.visible {
            return;
        }

        unsafe {
            xlib::XMapWindow(self.display, self.window);
            xlib::XMapWindow(self.display, self.frame);
            self.set_state(NormalState);
            xlib::XSetInputFocus(self.display, self.frame, xlib::RevertToPointerRoot, xlib::CurrentTime);
        }

        self.visible = true;
    }
}

impl X11Window for X11Client {
    fn x11_attributes(&self, display: *mut xlib::Display) -> Result<xlib::XWindowAttributes, String> {
        return self.window.x11_attributes(display);
    }

    fn x11_replace_property_long(&self, display: *mut xlib::Display, property: X11Atom, prop_type: c_ulong, data: &[c_ulong]) {
        self.frame.x11_replace_property_long(display, property, prop_type, data);
    }

    fn x11_dimensions(&self, display: *mut xlib::Display) -> Result<Dimensions, String> {
        return self.frame.x11_dimensions(display);
    }

    fn x11_geometry(&self, display: *mut xlib::Display) -> Result<(u64, i32, i32, u32, u32, u32, u32), String>{
        return self.frame.x11_geometry(display);
    }

    fn x11_is_transient_for(&self, display: *mut xlib::Display) -> Option<xlib::Window> {
        return self.window.x11_is_transient_for(display);
    }

    fn x11_message(&self, display: *mut xlib::Display, msg_type: atoms::X11Atom, msg_format: c_int, msg_data: xlib::ClientMessageData) {
        return self.window.x11_message(display, msg_type, msg_format, msg_data);
    }

    fn x11_wm_protocols(&self, display: *mut xlib::Display) -> Vec<xlib::Atom> {
        return self.window.x11_wm_protocols(display);
    }

    fn x11_wm_normal_hints(&self, display: *mut xlib::Display) -> Result<(xlib::XSizeHints, c_long), String> {
        return self.window.x11_wm_normal_hints(display);
    }
}

impl Dimensioned for X11Client {
    fn x(&self) -> i32 { self.x }
    fn y(&self) -> i32 { self.y }
    fn w(&self) -> u32 { self.w }
    fn h(&self) -> u32 { self.h }
    fn pos(&self) -> (i32, i32) { (self.x, self.y) }
    fn size(&self) -> (u32, u32) { (self.w, self.h) }

    fn set_x(&mut self, x: i32) {
        self.x = x;
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn set_y(&mut self, y: i32) {
        self.y = y;
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn set_w(&mut self, w: u32) {
        self.w = w;
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn set_h(&mut self, h: u32) {
        self.h = h;
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn set_pos(&mut self, pos: (i32, i32)) {
        (self.x, self.y) = pos;
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn set_size(&mut self, size: (u32, u32)) {
        (self.w, self.h) = size;
        self.move_resize(self.x, self.y, self.w, self.h);
    }

    fn dimensions(&self) -> Dimensions { Dimensions::new(self.x, self.y, self.w, self.h) }
}

impl X11Window for xlib::Window {
    fn x11_attributes(&self, display: *mut xlib::Display) -> Result<xlib::XWindowAttributes, String> {
        unsafe {
            let mut attributes: xlib::XWindowAttributes = mem::MaybeUninit::uninit().assume_init();
            match xlib::XGetWindowAttributes(display, *self, &mut attributes) {
                0 => return Err(String::from("Unable to retrieve attributes")),
                _ => return Ok(attributes),
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

    fn x11_is_transient_for(&self, display: *mut xlib::Display) -> Option<xlib::Window> {
        unsafe {
            let mut window: xlib::Window = 0;
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
                display: display,
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
            let mut atoms: *mut xlib::Atom = mem::MaybeUninit::uninit().assume_init();
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
            let mut size_hints: xlib::XSizeHints = mem::MaybeUninit::uninit().assume_init();
            let mut supplied_hints: c_long = 0;
            match xlib::XGetWMNormalHints(display, *self, &mut size_hints, &mut supplied_hints) {
                0 => return Err(String::from("Unable to retrieve WMNormalHints")),
                _ => return Ok((size_hints, supplied_hints)),
            }
        }
    }
}

