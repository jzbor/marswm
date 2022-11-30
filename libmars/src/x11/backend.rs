extern crate x11;

use x11::xlib;
use std::ptr;
use std::mem;

use crate::*;
use crate::x11::*;
use crate::x11::client::*;

type WM<'a> = dyn WindowManager<X11Backend, X11Client> + 'a;

pub struct X11Backend {
    display: *mut xlib::Display,
    screen: i32,
    root: u64,
}

impl X11Backend {
    /// Register window manager and initialize backend with new connection.
    pub fn init() -> Result<X11Backend, String> {
        // open new connection to x11 server
        let display = unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err("XOpenDisplay failed".to_owned());
            }
            display
        };

        return Self::init_with_connection(display);
    }

    fn handle_xevent(&mut self, wm: &mut WM, event: xlib::XEvent) {
        unsafe {  // unsafe because of access to union field
            match event.get_type() {
                xlib::ButtonPress => self.on_button_press(wm, event.button),
                xlib::EnterNotify => self.on_enter_notify(wm, event.crossing),
                xlib::KeyPress => self.on_key_press(wm, event.key),
                xlib::LeaveNotify => self.on_leave_notify(wm, event.crossing),
                xlib::MapRequest => self.on_map_request(wm, event.map_request),
                xlib::UnmapNotify => self.on_unmap_notify(wm, event.unmap),
                _ => (),
                // _ => { print!("."); stdout().flush().unwrap(); },
            }
        }
    }

    /// Register window manager and create backend from existing connection.
    pub fn init_with_connection(display: *mut xlib::Display) -> Result<X11Backend, String> {
        unsafe {
            let screen = xlib::XDefaultScreen(display);
            let root = xlib::XRootWindow(display, screen);

            let x11b = X11Backend {
                display,
                screen,
                root,
            };

            // register as window manager
            xlib::XSetErrorHandler(Some(on_wm_detected));
            // select events
            let mut attributes: xlib::XSetWindowAttributes = mem::MaybeUninit::uninit().assume_init();
            attributes.cursor = xlib::XCreateFontCursor(display, CURSOR_NORMAL);
            attributes.event_mask = xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask | xlib::KeyPressMask;
            xlib::XChangeWindowAttributes(display, root, xlib::CWEventMask | xlib::CWCursor, &mut attributes);
            xlib::XSync(display, xlib::False);
            xlib::XSetErrorHandler(Some(on_error));
            return Ok(x11b);
        }
    }

    /// Create a new client for the window and give it to the window manager
    fn manage(&mut self, wm: &mut WM, window: xlib::Window) {
        let attributes = match window.x11_attributes(self.display) {
            Ok(attr) => attr,
            Err(_) => return, // unable to get attributes for client (ignoring client)
        };

        // don't manage windows with the override_redirect flag set
        if attributes.override_redirect != 0 {
            return;
        }

        // set preferred dimensions to initial geometry dimensions
        let mut preferred_dims = match window.x11_dimensions(self.display) {
            Ok(dims) => dims,
            Err(_) => return,
        };

        // primitive way of accepting normal hints for preferred dimensions
        if let Ok((hints, _supplied)) = window.x11_wm_normal_hints(self.display) {
            // @TODO check supplied field first
            preferred_dims.x = hints.x;
            preferred_dims.y = hints.y;
            preferred_dims.w = hints.width as u32;
            preferred_dims.h = hints.height as u32;
        }

        // TODO
        // let transient_for = match window.is_transient_for(self.display) {
        //     Some(other_window) => match self.client(other_window) {
        //         Some(_ignored) => Some(other_window),
        //         None => None,
        //     },
        //     None => None,
        // };

        let client = X11Client::new(self.display, self.root, window);
        let boxed_client = Rc::new(RefCell::new(client));

        wm.manage(self, boxed_client);
    }

    fn on_button_press(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XButtonEvent) {
        let modifiers = sanitize_modifiers(event.state);
        let client = Self::client_by_frame(wm, event.window);
        wm.handle_button(self, modifiers, event.button, client);
    }

    fn on_enter_notify(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XCrossingEvent) {
        if let Some(client_rc) = Self::client_by_frame(wm, event.window) {
            wm.handle_focus(self, Some(client_rc.clone()));
            self.set_input_focus(client_rc);
        }
    }

    fn on_key_press(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, mut event: xlib::XKeyEvent) {
        let keysym = unsafe {
            xlib::XLookupKeysym(&mut event, 1)
        };

        let modifiers = sanitize_modifiers(event.state);
        let client_opt = Self::client_by_frame(wm, event.window);
        let key: u32 = keysym.try_into().unwrap();
        wm.handle_key(self, modifiers, key, client_opt)
    }

    fn on_leave_notify(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XCrossingEvent) {
        // let client_option = Self::client_by_frame(wm, event.window);
        // println!("LeaveNotify for client {}", event.window);
    }

    fn on_unmap_notify(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XUnmapEvent) {
        let root = self.root;
        let client_rc = match wm.clients_mut().find(|c| c.borrow().window() == event.window) {
            Some(client_rc) => client_rc.clone(),
            None => return,
        };

        // ignore unmap notifies generated from reparenting
        if event.event == root || client_rc.borrow().is_reparenting() {
            client_rc.borrow_mut().set_reparenting(false);
            return;
        }

        // tell window manager to drop client
        wm.unmanage(self, client_rc.clone());

        // remove client frame
        client_rc.borrow().destroy_frame();
    }

    fn on_map_request(&mut self, wm: &mut WM, event: xlib::XMapRequestEvent) {
        self.manage(wm, event.window);
    }

    fn client_by_frame<'a>(wm: &'a WM, frame: u64) -> Option<Rc<RefCell<X11Client>>> {
        return wm.clients().find(|c| c.borrow().frame() == frame).cloned();
    }
}

impl Backend<X11Client> for X11Backend {
    fn handle_existing_windows(&mut self, wm: &mut WM) {
        unsafe {
            xlib::XGrabServer(self.display);
            let mut returned_root: xlib::Window = mem::MaybeUninit::uninit().assume_init();
            let mut returned_parent: xlib::Window = mem::MaybeUninit::uninit().assume_init();
            let mut top_level_windows: *mut xlib::Window = mem::MaybeUninit::uninit().assume_init();
            let mut num_top_level_windows: u32 = mem::MaybeUninit::uninit().assume_init();

            match xlib::XQueryTree(self.display, self.root,
                                   &mut returned_root, &mut returned_parent,
                                   &mut top_level_windows, &mut num_top_level_windows) {
                0 => panic!("Unable to query x window tree"),
                _ => for i in 0..num_top_level_windows {
                    // @TODO check for override redirect and viewable status on pre-existing windows
                    self.manage(wm, *top_level_windows.offset(i.try_into().unwrap()));
                },
            }
            println!("Initially managed {} windows", num_top_level_windows);

            xlib::XFree(top_level_windows as *mut c_void);
            xlib::XUngrabServer(self.display);
        }
    }

    fn mouse_move(&mut self, wm: &mut WM, client_rc: Rc<RefCell<X11Client>>) {
        unsafe {
            // grab pointer
            let cursor = xlib::XCreateFontCursor(self.display, CURSOR_MOVE);
            let success = xlib::XGrabPointer(self.display, self.root, xlib::False, MOUSEMASK.try_into().unwrap(),
                    xlib::GrabModeAsync, xlib::GrabModeAsync, XLIB_NONE, cursor, xlib::CurrentTime);
            if success != xlib::GrabSuccess {
                xlib::XFreeCursor(self.display, cursor);
                return;
            }

            let orig_client_pos = client_rc.borrow().pos();
            let orig_pointer_pos = self.pointer_pos();
            let mut event: xlib::XEvent = mem::MaybeUninit::uninit().assume_init();

            loop {
                xlib::XMaskEvent(self.display, MOUSEMASK | xlib::ExposureMask | xlib::SubstructureRedirectMask, &mut event);

                if event.get_type() == xlib::MotionNotify {
                    // cast event to XMotionEvent
                    let event = event.motion;

                    // @TODO add max framerate (see moonwm)
                    let dest_x = orig_client_pos.0 + (event.x_root - orig_pointer_pos.0);
                    let dest_y = orig_client_pos.1 + (event.y_root - orig_pointer_pos.1);
                    let size = client_rc.borrow().size();
                    client_rc.borrow_mut().move_resize(dest_x, dest_y, size.0, size.1);
                } else if event.get_type() == xlib::ButtonRelease {
                    break;
                } else {
                    self.handle_xevent(wm, event);
                }
            }

            // Ungrab pointer and clean up
            xlib::XUngrabPointer(self.display, xlib::CurrentTime);
            xlib::XFreeCursor(self.display, cursor);
        }
    }

    fn mouse_resize(&mut self, wm: &mut WM, client_rc: Rc<RefCell<X11Client>>) {
        unsafe {
            // grab pointer
            let cursor = xlib::XCreateFontCursor(self.display, CURSOR_RESIZE);
            let success = xlib::XGrabPointer(self.display, self.root, xlib::False, MOUSEMASK.try_into().unwrap(),
                    xlib::GrabModeAsync, xlib::GrabModeAsync, XLIB_NONE, cursor, xlib::CurrentTime);
            if success != xlib::GrabSuccess {
                xlib::XFreeCursor(self.display, cursor);
                return;
            }

            let orig_client_size = client_rc.borrow().size();
            let orig_pointer_pos = self.pointer_pos();
            let mut event: xlib::XEvent = mem::MaybeUninit::uninit().assume_init();

            loop {
                xlib::XMaskEvent(self.display, MOUSEMASK | xlib::ExposureMask | xlib::SubstructureRedirectMask, &mut event);

                if event.get_type() == xlib::MotionNotify {
                    // cast event to XMotionEvent
                    let event = event.motion;

                    // @TODO add max framerate (see moonwm)
                    let dest_w = orig_client_size.0 as i32 + (event.x_root - orig_pointer_pos.0);
                    let dest_h = orig_client_size.1 as i32 + (event.y_root - orig_pointer_pos.1);

                    let pos = client_rc.borrow().pos();
                    let dest_w: u32 = if dest_w < WINDOW_MIN_SIZE.try_into().unwrap() { WINDOW_MIN_SIZE } else { dest_w.try_into().unwrap() };
                    let dest_h: u32 = if dest_h < WINDOW_MIN_SIZE.try_into().unwrap() { WINDOW_MIN_SIZE } else { dest_h.try_into().unwrap() };
                    client_rc.borrow_mut().move_resize(pos.0, pos.1, dest_w, dest_h);
                } else if event.get_type() == xlib::ButtonRelease {
                    break;
                } else {
                    self.handle_xevent(wm, event);
                }
            }

            // Ungrab pointer and clean up
            xlib::XUngrabPointer(self.display, xlib::CurrentTime);
            xlib::XFreeCursor(self.display, cursor);
        }
    }

    fn pointer_pos(&self) -> (i32, i32) {
        unsafe {
            let mut x: i32 = 0;
            let mut y: i32 = 0;
            let mut di: i32 = 0;
            let mut dui: u32 = 0;
            let mut dummy: xlib::Window = 0;

            if xlib::XQueryPointer(self.display, self.root, &mut dummy, &mut dummy, &mut x, &mut y,
                                   &mut di, &mut di, &mut dui) == xlib::True {
                return (x, y);
            } else {
                panic!("Cannot find pointer");
            }
        }
    }

    fn set_input_focus(&self, client_rc: Rc<RefCell<X11Client>>) {
        let client = (*client_rc).borrow();
        unsafe {
            xlib::XSetInputFocus(self.display, client.frame(), xlib::RevertToPointerRoot, xlib::CurrentTime);
        }
    }

    fn run(mut self, wm: &mut WM) {
        loop {
            unsafe {
                let mut event: xlib::XEvent = mem::MaybeUninit::uninit().assume_init();
                xlib::XNextEvent(self.display, &mut event);
                self.handle_xevent(wm, event);
            };
        }
    }
}


