extern crate x11;

use std::cmp;
use std::ffi::*;
use x11::xlib;

use crate::common::error::*;
use crate::common::*;
use crate::interfaces::wm::*;
use crate::platforms::x11::{
    draw::widget::*,
    misc::*,
    misc::atoms::*,
    misc::atoms::X11Atom::*,
    misc::window::*,
    wm::*,
};


type Decoration = (u32, u32, (u32, u32, u32, u32));

pub struct X11Client<A: PartialEq> {
    name: String,
    display: *mut xlib::Display,
    root: u64,
    window: u64,
    frame: u64,
    title_widget: Option<X11TextWidget>,
    attributes: A,

    orig_pos: (i32, i32), // position prior to reparenting
    x: i32, y: i32, // x, y position
    w: u32, h: u32, // width, height
    ibw: u32, obw: u32, // inner and outer border width
    fw: (u32, u32, u32, u32),        // frame width

    actively_reparenting: bool,
    dont_decorate: bool,
    fullscreen: bool,
    is_dialog: bool,
    visible: bool,

    frame_color: u64,
    saved_decorations: Option<Decoration>,
    saved_dimensions: Option<Dimensions>,
}

impl<A: Default + PartialEq> X11Client<A> {
    pub fn new(display: *mut xlib::Display, root: u64, window: xlib::Window, is_dialog: bool) -> Result<X11Client<A>> {
        let attributes = window.x11_attributes(display)?;
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
            xlib::XSelectInput(display, window, xlib::EnterWindowMask | xlib::LeaveWindowMask | xlib::PropertyChangeMask);
        }

        let name = match window.x11_class_hint(display) {
            Ok((name, class)) => format!("{}::{} [0x{:x}]", name, class, window),
            Err(msg) => {
                println!("Unable to get class hint: {}", msg);
                format!("0x{:x}", window)
            },
        };

        Ok( X11Client {
            name,
            display, root, window, frame,
            title_widget: None,
            attributes: A::default(),

            orig_pos: (x, y),
            x, y, w, h,
            ibw: 0,
            obw: 0,
            fw: (0, 0, 0, 0),

            actively_reparenting: false,
            dont_decorate: false,
            fullscreen: false,
            is_dialog,
            visible: false,

            frame_color: 0x000000,
            saved_decorations: None,
            saved_dimensions: None,
        } )
    }
}

impl<A: PartialEq> X11Client<A> {
    pub fn apply_motif_hints(&mut self) {
        let motif_atom = MotifWMHints.to_xlib_atom(self.display);
        if let Ok(hints) = self.x11_read_property_long(self.display, MotifWMHints, motif_atom) {
            if hints[MWM_HINTS_FLAGS_FIELD] & MWM_HINTS_DECORATIONS != 0
                    && hints[MWM_HINTS_DECORATIONS_FIELD] & MWM_DECOR_ALL == 0
                    && hints[MWM_HINTS_DECORATIONS_FIELD] & MWM_DECOR_BORDER == 0
                    && hints[MWM_HINTS_DECORATIONS_FIELD] & MWM_DECOR_TITLE == 0 {
                self.dont_decorate = true;
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
        self.x11_destroy(self.display);
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    fn remove_decoration(&mut self) {
        if self.saved_decorations.is_none() {
            self.saved_decorations = Some((self.ibw, self.obw, self.fw));
            self.set_inner_bw(0);
            self.set_outer_bw(0);
            self.set_frame_width((0, 0, 0, 0));
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

    pub fn update_title(&mut self) {
        let title = self.title();
        if let Some(title_widget) = &mut self.title_widget {
            title_widget.set_label(title);
        }
    }

    pub fn is_reparenting(&self) -> bool {
        self.actively_reparenting
    }

    pub fn set_reparenting(&mut self, status: bool) {
        self.actively_reparenting = status;
    }

    pub fn title_window(&self) -> Option<u64> {
        return self.title_widget.as_ref().map(|w| w.wid());
    }

    pub fn window(&self) -> u64 {
        self.window
    }
}

impl<A: PartialEq> Client<A> for X11Client<A> {
    fn add_title(&mut self, font: &str, hpad: u32, vpad: u32, color: u64) -> Result<()> {
        let title = self.title();
        let params = X11WidgetParams::new(0, 0, hpad, vpad);
        let mut widget = X11TextWidget::new(self.display, self.frame, params,
                                        title.clone(), font, color, self.frame_color)?;
        unsafe {
            xlib::XLowerWindow(self.display, widget.wid());
            widget.set_label(title);
        }

        self.title_widget = Some(widget);
        Ok(())
    }

    fn application(&self) -> String {
        match self.x11_class_hint(self.display) {
            Ok((_name, class)) => class,
            Err(_) => String::default(),
        }
    }

    fn attributes(&self) -> &A {
        &self.attributes
    }

    fn attributes_mut(&mut self) -> &mut A {
        &mut self.attributes
    }

    fn bind_button(&mut self, modifiers: u32, button: u32, target: ButtonTarget) {
        let mask: u32 = (xlib::ButtonPressMask | xlib::ButtonReleaseMask | xlib::ButtonMotionMask)
            .try_into().unwrap();

        let window = match target {
            ButtonTarget::Window => self.window,
            // ButtonTarget::Frame => self.frame,
            ButtonTarget::Frame => return,  // already grabbed as we own the window
            ButtonTarget::Root => panic!("You can't bind actions to the root window through a client window"),
        };

        for modifiers in alternative_modifiers(modifiers) {
            unsafe {
                xlib::XGrabButton(self.display, button, modifiers, window, xlib::False, mask,
                                  xlib::GrabModeAsync, xlib::GrabModeAsync, window, 0);
            }
        }
    }

    fn bind_key(&mut self, modifiers: u32, key: u32) {
        unsafe {
            let keycode = xlib::XKeysymToKeycode(self.display, key.into());
            for modifiers in alternative_modifiers(modifiers) {
                xlib::XGrabKey(self.display, keycode.into(), modifiers, self.frame, xlib::False,
                                xlib::GrabModeAsync, xlib::GrabModeAsync);
            }
        }
    }

    fn center_on_screen(&mut self, dimensions: Dimensions) {
        let (center_x, center_y) = dimensions.center();
        self.move_resize(center_x - (self.w as i32 / 2),
            center_y - (self.h as i32 / 2),
            self.w(), self.h())
    }

    fn close(&self) {
        self.x11_close(self.display, Some(on_error));
    }

    fn dont_decorate(&self) -> bool {
        self.dont_decorate
    }

    fn export_pinned(&self, state: bool, workspace_idx: Option<u32>) {
        let idx: u64 = if state { 0xffffffff } else { workspace_idx.expect("Need workspace index to unpin window").into() };
        let data = &[idx];
        self.window.x11_replace_property_long(self.display, NetWMDesktop, xlib::XA_CARDINAL, data);
    }

    fn export_tiled(&self, state: bool) {
        if state {
            self.x11_net_wm_state_add(self.display, MarsWMStateTiled);
        } else {
            self.x11_net_wm_state_remove(self.display, MarsWMStateTiled);
        }
    }

    fn export_workspace(&self, workspace_idx: u32) {
        let idx: u64 = workspace_idx.into();
        let data = &[idx];
        self.window.x11_replace_property_long(self.display, NetWMDesktop, xlib::XA_CARDINAL, data);
    }

    fn frame_width(&self) -> (u32, u32, u32, u32) {
        self.fw
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

    fn inner_bw(&self) -> u32 {
        self.ibw
    }

    fn inner_dimensions(&self) -> Dimensions {
        let (fw_north, fw_east, _, _) = self.fw;
        let (bw_north, bw_east, bw_south, bw_west) = self.total_bw();
        Dimensions::new(fw_east.try_into().unwrap(),
                               fw_north.try_into().unwrap(),
                               self.w - bw_east - bw_west,
                               self.h - bw_north - bw_south)
    }

    fn is_centered_on_screen(&self, dimensions: Dimensions) -> bool {
        let (center_x, center_y) = dimensions.center();

        self.x == center_x - (self.w as i32 / 2)
            && self.y == center_y - (self.h as i32 / 2)
    }

    fn is_dialog(&self) -> bool {
        self.is_dialog
    }

    fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    fn is_visible(&self) -> bool {
        self.visible
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

        let inner_dimensions = self.inner_dimensions();
        unsafe {
            xlib::XMoveResizeWindow(self.display, self.frame, self.x, self.y,
                                    self.w - 2*self.obw, self.h - 2 * self.obw);
            xlib::XMoveResizeWindow(self.display, self.window,
                                    inner_dimensions.x(), inner_dimensions.y(),
                                    inner_dimensions.w(), inner_dimensions.h());
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn outer_bw(&self) -> u32 {
        self.obw
    }

    fn raise(&self) {
        unsafe {
            xlib::XRaiseWindow(self.display, self.frame);
        }
    }

    fn set_dimensions(&mut self, dimensions: Dimensions) {
        self.move_resize(dimensions.x(), dimensions.y(), dimensions.w(), dimensions.h())
    }

    fn set_frame_color(&mut self, color: u64) {
        unsafe {
            xlib::XSetWindowBackground(self.display, self.frame, color);
            xlib::XClearWindow(self.display, self.frame);
            xlib::XSync(self.display, xlib::False);
        }
        self.frame_color = color;

        if let Some(title_widget) = &mut self.title_widget {
            let _ignore_result = title_widget.set_background(color);
        }
    }

    fn set_frame_width(&mut self, width: (u32, u32, u32, u32)) {
        let diff_north = (width.0 as i32) - (self.fw.0 as i32);
        let diff_east = (width.1 as i32) - (self.fw.1 as i32);
        let diff_south = (width.2 as i32) - (self.fw.2 as i32);
        let diff_west = (width.3 as i32) - (self.fw.3 as i32);
        self.fw = width;
        self.move_resize(self.x - diff_west, self.y - diff_north, (self.w as i32 + diff_east + diff_west) as u32, (self.h as i32 + diff_north + diff_south) as u32);
    }

    fn set_fullscreen(&mut self, monitor_conf: &MonitorConfig) {
        let dimensions = monitor_conf.dimensions();
        self.saved_dimensions = Some(self.dimensions());
        self.fullscreen = true;
        self.x11_net_wm_state_add(self.display, NetWMStateFullscreen);
        self.remove_decoration();
        self.move_resize(dimensions.x(), dimensions.y(), dimensions.w(), dimensions.h());
        self.raise();
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

    fn set_title_color(&mut self, color: u64) {
        if let Some(title_widget) = &mut self.title_widget {
            let _ignore_result = title_widget.set_foreground(color);
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

    fn title(&self) -> String {
        self.window.x11_wm_name(self.display).unwrap_or_default()
    }

    fn total_bw(&self) -> (u32, u32, u32, u32) {
        (self.ibw + self.fw.0 + self.obw,
                self.ibw + self.fw.1 + self.obw,
                self.ibw + self.fw.2 + self.obw,
                self.ibw + self.fw.3 + self.obw)
    }

    fn unset_fullscreen(&mut self) {
        if let Some(dimensions) = self.saved_dimensions {
            self.fullscreen = false;
            self.x11_net_wm_state_remove(self.display, NetWMStateFullscreen);
            self.restore_decoration();
            self.move_resize(dimensions.x(), dimensions.y(), dimensions.w(), dimensions.h());
            self.saved_dimensions = None;
        }
    }

    fn warp_pointer_to_center(&self) {
        unsafe {
            // let (x, y) = self.center();
            let (x, y) = ((self.w / 2) as i32, (self.h / 2) as i32);
            // FIXME handle return (return result)
            xlib::XWarpPointer(self.display, 0, self.frame, 0, 0, 0, 0, x, y);
        }
    }

    fn warp_pointer_to_corner(&self) {
        unsafe {
            // let (x, y) = self.center();
            let (x, y) = (self.w as i32, self.h as i32);
            // FIXME handle return (return result)
            xlib::XWarpPointer(self.display, 0, self.frame, 0, 0, 0, 0, x, y);
        }
    }
}

impl<A: PartialEq> Dimensioned for X11Client<A> {
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

impl<A: PartialEq> Drop for X11Client<A> {
    fn drop(&mut self) {
        unsafe {
            // These generate xlib errors if the window is already unmapped
            xlib::XSetErrorHandler(Some(on_error_dummy));
            xlib::XReparentWindow(self.display, self.window, self.root, self.orig_pos.0, self.orig_pos.1);
            if self.visible {
                xlib::XMapWindow(self.display, self.window);
            }
            xlib::XRemoveFromSaveSet(self.display, self.window);
            xlib::XSetErrorHandler(Some(on_error));
            xlib::XDestroyWindow(self.display, self.frame);
        }
    }
}

impl<A: PartialEq> Eq for X11Client<A> {}

impl<A: PartialEq> PartialEq for X11Client<A> {
    fn eq(&self, other: &Self) -> bool {
        self.frame == other.frame
    }
}

impl<A: PartialEq> X11Window for X11Client<A> {
    fn x11_attributes(&self, display: *mut xlib::Display) -> Result<xlib::XWindowAttributes> {
        self.window.x11_attributes(display)
    }

    fn x11_class_hint(&self, display: *mut xlib::Display) -> Result<(String, String)> {
        self.window.x11_class_hint(display)
    }

    fn x11_close(&self, display: *mut xlib::Display,
                 error_handler: Option<unsafe extern "C" fn(_: *mut xlib::Display, _: *mut xlib::XErrorEvent) -> c_int>) {
        self.window.x11_close(display, error_handler);
    }

    fn x11_destroy(&self, display: *mut xlib::Display) {
        println!("Destroying frame for client {}", self.name);
        unsafe {
            // These generate xlib errors if the window is already unmapped
            xlib::XReparentWindow(display, self.window, self.root, 0, 0);
            xlib::XRemoveFromSaveSet(display, self.window);
            xlib::XDestroyWindow(display, self.frame);
        }
    }

    fn x11_get_state(&self, display: *mut xlib::Display) -> Result<u64> {
        self.window.x11_get_state(display)
    }

    fn x11_get_text_list_property(&self, display: *mut xlib::Display, property: X11Atom) -> Result<Vec<String>> {
        self.window.x11_get_text_list_property(display, property)
    }

    fn x11_net_wm_state(&self, display: *mut xlib::Display) -> Result<Vec<X11Atom>> {
        self.window.x11_net_wm_state(display)
    }

    fn x11_net_wm_state_add(&self, display: *mut xlib::Display, state: X11Atom) {
        self.window.x11_net_wm_state_add(display, state);
    }

    fn x11_net_wm_state_remove(&self, display: *mut xlib::Display, state: X11Atom) {
        self.window.x11_net_wm_state_remove(display, state);
    }

    fn x11_read_property_long(&self, display: *mut xlib::Display, property: X11Atom, prop_type: c_ulong) -> Result<Vec<u64>> {
        self.window.x11_read_property_long(display, property, prop_type)
    }

    fn x11_read_property_string(&self, display: *mut xlib::Display, property: X11Atom) -> Result<String> {
        self.window.x11_read_property_string(display, property)
    }

    fn x11_replace_property_long(&self, display: *mut xlib::Display, property: X11Atom, prop_type: c_ulong, data: &[c_ulong]) {
        self.window.x11_replace_property_long(display, property, prop_type, data);
    }

    fn x11_set_state(&self, display: *mut xlib::Display, state: i32) {
        self.window.x11_set_state(display, state);
    }

    fn x11_set_text_list_property(&self, display: *mut xlib::Display, property: X11Atom, list: &[String]) {
        self.window.x11_set_text_list_property(display, property, list);
    }

    fn x11_dimensions(&self, display: *mut xlib::Display) -> Result<Dimensions> {
        self.frame.x11_dimensions(display)
    }

    fn x11_geometry(&self, display: *mut xlib::Display) -> Result<(u64, i32, i32, u32, u32, u32, u32)> {
        self.frame.x11_geometry(display)
    }

    fn x11_get_window_types(&self, display: *mut xlib::Display) -> Vec<X11Atom> {
        self.window.x11_get_window_types(display)
    }

    fn x11_is_transient_for(&self, display: *mut xlib::Display) -> Option<xlib::Window> {
        self.window.x11_is_transient_for(display)
    }

    fn x11_map(&self, display: *mut xlib::Display) {
        self.window.x11_map(display);
    }

    fn x11_message(&self, display: *mut xlib::Display, msg_type: atoms::X11Atom, msg_format: c_int, msg_data: xlib::ClientMessageData) {
        self.window.x11_message(display, msg_type, msg_format, msg_data)
    }

    fn x11_supports_protocol(&self, display: *mut xlib::Display, protocol: X11Atom) -> bool {
        self.window.x11_supports_protocol(display, protocol)
    }

    fn x11_unmap(&self, display: *mut xlib::Display) {
        self.window.x11_unmap(display);
    }

    fn x11_wm_protocols(&self, display: *mut xlib::Display) -> Vec<X11Atom> {
        self.window.x11_wm_protocols(display)
    }

    fn x11_wm_name(&self, display: *mut xlib::Display) -> Result<String> {
        self.window.x11_wm_name(display)
    }

    fn x11_wm_normal_hints(&self, display: *mut xlib::Display) -> Result<(xlib::XSizeHints, c_long)> {
        self.window.x11_wm_normal_hints(display)
    }
}
