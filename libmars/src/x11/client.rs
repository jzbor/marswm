extern crate x11;

use x11::xlib;
use std::cmp;

use crate::*;
use crate::x11::*;
use crate::x11::atoms::*;
use crate::x11::window::*;


#[derive(PartialEq,Eq)]
pub struct X11Client {
    name: String,
    display: *mut xlib::Display,
    root: u64,
    window: u64,
    frame: u64,

    x: i32, y: i32, // x, y position
    w: u32, h: u32, // width, height
    ibw: u32, obw: u32, // inner and outer border width
    fw: u32,        // frame width

    actively_reparenting: bool,
    dont_decorate: bool,
    fullscreen: bool,
    is_dialog: bool,
    visible: bool,

    saved_decorations: Option<(u32, u32, u32)>,
    saved_dimensions: Option<Dimensions>,
}

impl X11Client {
    pub fn new(display: *mut xlib::Display, root: u64, window: xlib::Window, is_dialog: bool) -> X11Client {
        let attributes = window.x11_attributes(display)
            .expect("Unable to retrieve attributes for new client");  // FIXME this should not be fatal
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

        unsafe {
            xlib::XSelectInput(display, window, xlib::EnterWindowMask | xlib::LeaveWindowMask);
        }

        let name = match window.x11_class_hint(display) {
            Ok((name, class)) => format!("{}::{} [0x{:x}]", name, class, window),
            Err(msg) => {
                println!("Unable to get class hint: {}", msg);
                format!("0x{:x}", window)
            },
        };

        return X11Client {
            name,
            display, root, window, frame,
            x, y, w, h,
            ibw: 0,
            obw: 0,
            fw: 0,

            actively_reparenting: false,
            dont_decorate: false,
            fullscreen: false,
            is_dialog,
            visible: false,

            saved_decorations: None,
            saved_dimensions: None,
        };
    }

    pub fn apply_motif_hints(&mut self) {
        let motif_atom = MotifWMHints.to_xlib_atom(self.display);
        if let Ok(hints) = self.x11_read_property_long(self.display, motif_atom, motif_atom) {
            if hints[MWM_HINTS_FLAGS_FIELD] & MWM_HINTS_DECORATIONS != 0 {
                if hints[MWM_HINTS_DECORATIONS_FIELD] & MWM_DECOR_ALL == 0
                        && hints[MWM_HINTS_DECORATIONS_FIELD] & MWM_DECOR_BORDER == 0
                        && hints[MWM_HINTS_DECORATIONS_FIELD] & MWM_DECOR_TITLE == 0 {
                    self.dont_decorate = true;
                }
            }
        }
    }

    pub fn apply_size_hints(&mut self) {
        // the handling of normal hints is *very* heavily inspired by dwm
        if let Ok((hints, _supplied)) = self.window.x11_wm_normal_hints(self.display) {
            let mut basew: u32 = 0;
            let mut baseh: u32 = 0;
            let mut incw: u32 = 0;
            let mut inch: u32 = 0;
            let mut minw: u32 = 0;
            let mut minh: u32 = 0;
            let mut maxw: u32 = 0;
            let mut maxh: u32 = 0;
            let mut mina = 0.0;
            let mut maxa = 0.0;

            // read hints
            if hints.flags & xlib::PSize != 0 {
                self.w = hints.width.try_into().unwrap();
                self.h = hints.width.try_into().unwrap();
            }
            if hints.flags & xlib::PBaseSize != 0 {
                basew = hints.base_width.try_into().unwrap();
                baseh = hints.base_height.try_into().unwrap();
            } else if hints.flags & xlib::PMinSize != 0 {
                basew = hints.min_width.try_into().unwrap();
                baseh = hints.min_height.try_into().unwrap();
            }

            if hints.flags & xlib::PResizeInc != 0 {
                incw = hints.width_inc.try_into().unwrap();
                inch = hints.height_inc.try_into().unwrap();
            }

            if hints.flags & xlib::PMaxSize != 0 {
                maxw = hints.max_width.try_into().unwrap();
                maxh = hints.max_height.try_into().unwrap();
            }

            if hints.flags & xlib::PMinSize != 0 {
                minw = hints.min_width.try_into().unwrap();
                minh = hints.min_height.try_into().unwrap();
            }

            if hints.flags & xlib::PAspect != 0 {
                mina = hints.min_aspect.y as f32 / hints.min_aspect.x as f32;
                maxa = hints.max_aspect.x as f32 / hints.max_aspect.y as f32;
            }

            // apply hints
            if basew >= minw && baseh > minh {
                self.w = basew;
                self.h = baseh;
            }

            // adjust for aspect limits
            if mina > 0.0 && maxa > 0.0 {
                if maxa < self.w as f32 / self.h as f32 {
                    self.w = (self.h as f32 * maxa + 0.5) as u32;
                } else if mina < self.h as f32 / self.w as f32 {
                    self.h = (self.w as f32 * mina + 0.5) as u32;
                }
            }
            if basew >= minw && baseh > minh { // required for increment calculation
                self.w -= basew;
                self.h -= baseh;
            }
            // adjust for increment value
            if incw != 0 {
                self.w -= self.w % incw;
            }
            if inch != 0 {
                self.h -= self.h % inch;
            }
            // restore base dimensions
            self.w = cmp::max(self.w + basew, minw);
            self.h = cmp::max(self.h + baseh, minh);
            if maxw != 0 {
                self.w = cmp::min(self.w, maxw);
            }
            if maxh != 0 {
                self.h = cmp::min(self.h, maxh);
            }
        }
    }

    pub fn destroy_frame(&self) {
        println!("Destroying frame for client {}", self.name);
        unsafe {
            // These generate xlib errors if the window is already unmapped
            xlib::XReparentWindow(self.display, self.window, self.root, 0, 0);
            xlib::XRemoveFromSaveSet(self.display, self.window);
            xlib::XDestroyWindow(self.display, self.frame);
        }

    }

    pub fn frame(&self) -> u64 {
        return self.frame;
    }

    fn remove_decoration(&mut self) {
        if self.saved_decorations.is_none() {
            self.saved_decorations = Some((self.ibw, self.obw, self.fw));
            self.set_inner_bw(0);
            self.set_outer_bw(0);
            self.set_frame_width(0);
        }
    }

    fn restore_decoration(&mut self) {
        if let Some((ibw, obw, fw)) = self.saved_decorations {
            self.set_inner_bw(ibw);
            self.set_outer_bw(obw);
            self.set_frame_width(fw);
            self.saved_decorations = None;
        }
    }

    pub fn is_reparenting(&self) -> bool {
        return self.actively_reparenting;
    }

    pub fn set_reparenting(&mut self, status: bool) {
        self.actively_reparenting = status;
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

    fn center_on_screen(&mut self, monitor_conf: &MonitorConfig) {
        let (center_x, center_y) = monitor_conf.window_area().center();
        self.move_resize(center_x - (self.w as i32 / 2),
            center_y - (self.h as i32 / 2),
            self.w(), self.h())
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

    fn dont_decorate(&self) -> bool {
        return self.dont_decorate;
    }

    fn export_pinned(&self, state: bool, workspace_idx: Option<u32>) {
        let idx: u64 = if state { 0xffffffff } else { workspace_idx.expect("Need workspace index to unpin window").into() };
        let data = &[idx];
        self.window.x11_replace_property_long(self.display, NetWMDesktop.to_xlib_atom(self.display), xlib::XA_CARDINAL, data);
    }

    fn export_tiled(&self, state: bool) {
        let xatom = MarsWMStateTiled.to_xlib_atom(self.display);
        if state {
            self.x11_net_wm_state_add(self.display, xatom);
        } else {
            self.x11_net_wm_state_remove(self.display, xatom);
        }
    }

    fn export_workspace(&self, workspace_idx: u32) {
        let idx: u64 = workspace_idx.into();
        let data = &[idx];
        self.window.x11_replace_property_long(self.display, NetWMDesktop.to_xlib_atom(self.display), xlib::XA_CARDINAL, data);
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
            self.x11_set_state(self.display, ICONIC_STATE);
            xlib::XSelectInput(self.display, self.root, ra.your_event_mask);
            xlib::XSelectInput(self.display, self.frame, fa.your_event_mask);
            xlib::XUngrabServer(self.display);
        }

        self.visible = false;
    }

    fn is_dialog(&self) -> bool {
        return self.is_dialog;
    }

    fn is_fullscreen(&self) -> bool {
        return self.fullscreen;
    }

    fn is_visible(&self) -> bool {
        return self.visible;
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

    fn name(&self) -> &str {
        return &self.name;
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
        let diff = (width as i32) - (self.fw as i32);
        self.fw = width;
        self.move_resize(self.x - diff, self.y - diff, (self.w as i32 + 2 * diff) as u32, (self.h as i32 + 2 * diff) as u32);
    }

    fn set_fullscreen(&mut self, state: bool, monitor_conf: &MonitorConfig) {
        let dimensions = monitor_conf.dimensions();
        if state {
            self.saved_dimensions = Some(self.dimensions());
            self.fullscreen = true;
            let xatom = NetWMStateFullscreen.to_xlib_atom(self.display);
            self.x11_net_wm_state_add(self.display, xatom);
            self.remove_decoration();
            self.move_resize(dimensions.x(), dimensions.y(), dimensions.w(), dimensions.h());
            self.raise();
        } else {
            if let Some(dimensions) = self.saved_dimensions {
                self.fullscreen = false;
                let xatom = NetWMStateFullscreen.to_xlib_atom(self.display);
                self.x11_net_wm_state_remove(self.display, xatom);
                self.move_resize(dimensions.x(), dimensions.y(), dimensions.w(), dimensions.h());
                self.restore_decoration();
            }
        }
    }

    fn set_height(&mut self, height: u32) {
        self.h = height;
    }

    fn set_inner_bw(&mut self, bw: u32) {
        let diff = (bw as i32) - (self.ibw as i32);
        self.ibw = bw;
        unsafe {
            xlib::XSetWindowBorderWidth(self.display, self.window, self.ibw);
        }
        self.move_resize(self.x - diff, self.y - diff, (self.w as i32 + 2 * diff) as u32, (self.h as i32 + 2 * diff) as u32);
    }

    fn set_inner_color(&mut self, color: u64) {
        unsafe {
            xlib::XSetWindowBorder(self.display, self.window, color);
        }
    }

    fn set_outer_bw(&mut self, bw: u32) {
        let diff = (bw as i32) - (self.obw as i32);
        self.obw = bw;
        unsafe {
            xlib::XSetWindowBorderWidth(self.display, self.frame, self.obw);
        }
        self.move_resize(self.x - diff, self.y - diff, (self.w as i32 + 2 * diff) as u32, (self.h as i32 + 2 * diff) as u32);
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
            self.x11_set_state(self.display, NORMAL_STATE);
            xlib::XSetInputFocus(self.display, self.frame, xlib::RevertToPointerRoot, xlib::CurrentTime);
        }

        self.visible = true;
    }

    fn warp_pointer_to_center(&self) {
        unsafe {
            // let (x, y) = self.center();
            let (x, y) = ((self.w / 2) as i32, (self.h / 2) as i32);
            // FIXME handle return (return result)
            xlib::XWarpPointer(self.display, 0, self.frame, 0, 0, 0, 0, x, y);
        }
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

impl X11Window for X11Client {
    fn x11_net_wm_state_add(&self, display: *mut xlib::Display, state: xlib::Atom) {
        self.window.x11_net_wm_state_add(display, state);
    }

    fn x11_net_wm_state_remove(&self, display: *mut xlib::Display, state: xlib::Atom) {
        self.window.x11_net_wm_state_remove(display, state);
    }

    fn x11_attributes(&self, display: *mut xlib::Display) -> Result<xlib::XWindowAttributes, String> {
        return self.window.x11_attributes(display);
    }

    fn x11_class_hint(&self, display: *mut xlib::Display) -> Result<(String, String), String> {
        return self.window.x11_class_hint(display);
    }

    fn x11_get_state(&self, display: *mut xlib::Display) -> Result<u64, &'static str> {
        return self.window.x11_get_state(display);
    }

    fn x11_read_property_long(&self, display: *mut xlib::Display, property: xlib::Atom, prop_type: c_ulong) -> Result<Vec<u64>, &'static str> {
        return self.window.x11_read_property_long(display, property, prop_type);
    }

    fn x11_replace_property_long(&self, display: *mut xlib::Display, property: xlib::Atom, prop_type: c_ulong, data: &[c_ulong]) {
        self.window.x11_replace_property_long(display, property, prop_type, data);
    }

    fn x11_set_state(&self, display: *mut xlib::Display, state: i32) {
        self.window.x11_set_state(display, state);
    }

    fn x11_set_text_list_property(&self, display: *mut xlib::Display, property: xlib::Atom, list: Vec<CString>) {
        self.window.x11_set_text_list_property(display, property, list);
    }

    fn x11_dimensions(&self, display: *mut xlib::Display) -> Result<Dimensions, String> {
        return self.frame.x11_dimensions(display);
    }

    fn x11_geometry(&self, display: *mut xlib::Display) -> Result<(u64, i32, i32, u32, u32, u32, u32), String>{
        return self.frame.x11_geometry(display);
    }

    fn x11_get_window_types(&self, display: *mut xlib::Display) -> Vec<xlib::Atom> {
        return self.window.x11_get_window_types(display);
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
