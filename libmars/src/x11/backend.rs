extern crate x11;

use x11::xlib;
use x11::xinerama;
use std::ptr;
use std::slice;
use std::mem::MaybeUninit;

use crate::*;
use crate::Dimensioned;
use crate::x11::*;
use crate::x11::atoms::*;
use crate::x11::client::*;
use crate::x11::window::*;


macro_rules! print_event {
    ($wm:expr, $event:expr) => {
        #[cfg(debug_assertions)]
        if let Some(client) = $wm.clients().find(|c| c.borrow().window() == $event.window) {
            println!("Received {} for window 0x{:x} (window of {})", event_type(&$event), $event.window, client.borrow().name());
        } else if let Some(client) = $wm.clients().find(|c| c.borrow().frame() == $event.window) {
            println!("Received {} for window 0x{:x} (frame of {})", event_type(&$event), $event.window, client.borrow().name());
        } else {
            println!("Received {} for window 0x{:x} (not a client)", event_type(&$event), $event.window);
        }
        #[cfg(debug_assertions)]
        println!("\t{:?}", $event);
        #[cfg(not(debug_assertions))]
        let (_, _) = (&$wm, &$event);
    }
}

#[allow(unused_macros)]
macro_rules! debug_occurences {
    ($wm:expr, $client:expr) => {
        let window = $client.borrow().window();
        let frame = $client.borrow().frame();
        println!("Number of occurences in client list: {} / {} / {}",
                 $wm.clients().filter(|c| *c == &$client).count(),
                 $wm.clients().filter(|c| c.borrow().window() == window).count(),
                 $wm.clients().filter(|c| c.borrow().frame() == frame).count());
    }
}



type WM<'a> = dyn WindowManager<X11Backend, X11Client> + 'a;

pub struct X11Backend {
    display: *mut xlib::Display,
    screen: *mut xlib::Screen,
    root: u64,
    wmcheck_win: u64,
    last_active: Option<Rc<RefCell<X11Client>>>,
}


const SUPPORTED_ATOMS: &'static [X11Atom; 18] = & [
    NetActiveWindow,
    NetClientList,
    NetClientListStacking,
    NetCloseWindow,
    NetCurrentDesktop,
    NetDesktopNames,
    NetNumberOfDesktops,
    NetSupported,
    NetSupportingWMCheck,
    NetWMDesktop,
    NetWMName,
    NetWMState,
    NetWMStateFullscreen,
    NetWMWindowType,
    NetWMWindowTypeDock,
    NetWMWindowTypeDesktop,
    NetWMWindowTypeDialog,
    NetWMWindowTypeMenu,
];


impl X11Backend {
    /// Register window manager and initialize backend with new connection.
    pub fn init(name: &str) -> Result<X11Backend, String> {
        // open new connection to x11 server
        let display = unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err("XOpenDisplay failed".to_owned());
            }
            display
        };

        return Self::init_with_connection(display, name);
    }

    /// Register window manager and create backend from existing connection.
    pub fn init_with_connection(display: *mut xlib::Display, name: &str) -> Result<X11Backend, String> {
        unsafe {
            let screen = xlib::XDefaultScreenOfDisplay(display);
            let root = xlib::XDefaultRootWindow(display);

            let mut x11b = X11Backend {
                display,
                screen,
                root,
                wmcheck_win: 0,
                last_active: None,
            };

            // For debugging:
            #[cfg(debug_assertions)]
            xlib::XSynchronize(display, 1);

            // export wm name
            x11b.wmcheck_win = xlib::XCreateSimpleWindow(display, root, 0, 0, 1, 1, 0, 0, 0);
            xlib::XChangeProperty(display, x11b.wmcheck_win, NetSupportingWMCheck.to_xlib_atom(display), xlib::XA_WINDOW,
                32, xlib::PropModeReplace, &x11b.wmcheck_win as *const u64 as *const u8, 1);
            xlib::XChangeProperty(display, x11b.wmcheck_win, NetWMName.to_xlib_atom(display),
                UTF8String.to_xlib_atom(display), 8, xlib::PropModeReplace, name.as_ptr(), name.len() as i32);
            xlib::XChangeProperty(display, root, NetSupportingWMCheck.to_xlib_atom(display), xlib::XA_WINDOW,
                32, xlib::PropModeReplace, &x11b.wmcheck_win as *const u64 as *const u8, 1);

            // register as window manager
            xlib::XSetErrorHandler(Some(on_wm_detected));
            // select events
            let mut attributes: MaybeUninit<xlib::XSetWindowAttributes> = MaybeUninit::uninit();
            (*attributes.as_mut_ptr()).cursor = xlib::XCreateFontCursor(display, CURSOR_NORMAL);
            (*attributes.as_mut_ptr()).event_mask = xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask | xlib::StructureNotifyMask | xlib::KeyPressMask;
            xlib::XChangeWindowAttributes(display, root, xlib::CWEventMask | xlib::CWCursor, attributes.as_mut_ptr());
            xlib::XSync(display, xlib::False);
            xlib::XSetErrorHandler(Some(on_error));

            x11b.set_supported_atoms(SUPPORTED_ATOMS);

            return Ok(x11b);
        }
    }

    fn handle_xevent(&mut self, wm: &mut WM, event: xlib::XEvent) {
        unsafe {  // unsafe because of access to union field
            match event.get_type() {
                xlib::ButtonPress => self.on_button_press(wm, event.button),
                xlib::ClientMessage => self.on_client_message(wm, event.client_message),
                xlib::ConfigureNotify => self.on_configure_notify(wm, event.configure),
                xlib::DestroyNotify => self.on_destroy_notify(wm, event.destroy_window),
                xlib::EnterNotify => self.on_enter_notify(wm, event.crossing),
                xlib::KeyPress => self.on_key_press(wm, event.key),
                xlib::LeaveNotify => self.on_leave_notify(wm, event.crossing),
                xlib::MapRequest => self.on_map_request(wm, event.map_request),
                xlib::MapNotify => self.on_map_notify(wm, event.map),
                xlib::UnmapNotify => self.on_unmap_notify(wm, event.unmap),
                _ => (),
                // _ => { print!("."); stdout().flush().unwrap(); },
            }
        }
    }

    /// Create a new client for the window and give it to the window manager
    fn manage(&mut self, wm: &mut WM, window: xlib::Window) {
        if window == self.wmcheck_win {
            return;
        }

        let attributes = match window.x11_attributes(self.display) {
            Ok(attr) => attr,
            Err(_) => return, // unable to get attributes for client (ignoring client)
        };

        // don't manage windows with the override_redirect flag set
        if attributes.override_redirect != 0 {
            return;
        }

        let mut is_dialog = false;
        let window_types: Vec<X11Atom> = window.x11_get_window_types(self.display).iter()
            .map(|a| X11Atom::from_xlib_atom(self.display, *a)).flatten().collect();
        for win_type in &window_types {
            match win_type {
                NetWMWindowTypeDesktop => unsafe {
                    xlib::XMapWindow(self.display, window);
                    xlib::XLowerWindow(self.display, window);
                    return;
                },
                NetWMWindowTypeDialog => {
                    is_dialog = true;
                },
                NetWMWindowTypeDock => unsafe {
                    xlib::XMapRaised(self.display, window);
                    return;
                },
                NetWMWindowTypeMenu => unsafe {
                    xlib::XSelectInput(self.display, window, xlib::EnterWindowMask | xlib::LeaveWindowMask);
                    xlib::XMapRaised(self.display, window);
                    return;
                },
                _ => (),
            }
        }

        // TODO
        // let transient_for = match window.is_transient_for(self.display) {
        //     Some(other_window) => match self.client(other_window) {
        //         Some(_ignored) => Some(other_window),
        //         None => None,
        //     },
        //     None => None,
        // };

        let mut client = X11Client::new(self.display, self.root, window, is_dialog);
        client.apply_size_hints();
        client.apply_motif_hints();

        println!("New client: {} (frame: {}) with types {:?}", client.name(), client.frame(), window_types);

        let boxed_client = Rc::new(RefCell::new(client));
        wm.manage(self, boxed_client.clone());

        // Setting workspace as specified by _NET_WM_DESKTOP
        let workspace_req = {
            match boxed_client.clone().borrow().x11_read_property_long(self.display, NetWMDesktop.to_xlib_atom(self.display), xlib::XA_CARDINAL) {
                Ok(data) => Some(data[0]),
                Err(_msg) => None,
            }
        };
        if let Some(workspace) = workspace_req {
            wm.move_to_workspace(self, boxed_client, workspace.try_into().unwrap())
        }
    }

    fn mouse_action(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>,
                    client_rc: Rc<RefCell<X11Client>>, cursor_type: u32,
                    action: fn(&mut Self, &Rc<RefCell<X11Client>>, (i32, i32), (u32, u32), (i32, i32))) {
        unsafe {
            // grab pointer
            let cursor = xlib::XCreateFontCursor(self.display, cursor_type);
            let success = xlib::XGrabPointer(self.display, self.root, xlib::False, MOUSEMASK.try_into().unwrap(),
                    xlib::GrabModeAsync, xlib::GrabModeAsync, XLIB_NONE, cursor, xlib::CurrentTime);
            if success != xlib::GrabSuccess {
                xlib::XFreeCursor(self.display, cursor);
                return;
            }

            let orig_client_pos = client_rc.borrow().pos();
            let orig_client_size = client_rc.borrow().size();
            let orig_pointer_pos = self.pointer_pos();
            let mut event: xlib::XEvent = MaybeUninit::uninit().assume_init();

            loop {
                xlib::XMaskEvent(self.display, MOUSEMASK | xlib::ExposureMask | xlib::SubstructureRedirectMask, &mut event);

                if event.get_type() == xlib::MotionNotify {
                    // @TODO add max framerate (see moonwm)
                    // cast event to XMotionEvent
                    let event = event.motion;
                    let delta = (event.x_root - orig_pointer_pos.0,
                                 event.y_root - orig_pointer_pos.1);

                    let old_center = client_rc.borrow().center();
                    let old_mon = self.point_to_monitor(old_center);
                    action(self, &client_rc, orig_client_pos, orig_client_size, delta);
                    if let Some(old_mon) = old_mon {
                        let new_center = client_rc.borrow().center();
                        if let Some(new_mon) = self.point_to_monitor(new_center).clone() {
                            if old_mon != new_mon {
                                wm.handle_client_switches_monitor(client_rc.clone(), new_mon);
                            }
                        }
                    }
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

    fn mouse_action_move(&mut self, client_rc: &Rc<RefCell<X11Client>>, orig_client_pos: (i32, i32),
                         _orig_client_size: (u32, u32), delta: (i32, i32)) {
        let dest_x = orig_client_pos.0 + delta.0;
        let dest_y = orig_client_pos.1 + delta.1;
        let size = client_rc.borrow().size();
        client_rc.borrow_mut().move_resize(dest_x, dest_y, size.0, size.1);
    }

    fn mouse_action_resize(&mut self, client_rc: &Rc<RefCell<X11Client>>, _orig_client_pos: (i32, i32),
                         orig_client_size: (u32, u32), delta: (i32, i32)) {
        let dest_w = orig_client_size.0 as i32 + delta.0;
        let dest_h = orig_client_size.1 as i32 + delta.1;
        let pos = client_rc.borrow().pos();
        let dest_w: u32 = if dest_w < WINDOW_MIN_SIZE.try_into().unwrap() { WINDOW_MIN_SIZE } else { dest_w.try_into().unwrap() };
        let dest_h: u32 = if dest_h < WINDOW_MIN_SIZE.try_into().unwrap() { WINDOW_MIN_SIZE } else { dest_h.try_into().unwrap() };
        client_rc.borrow_mut().move_resize(pos.0, pos.1, dest_w, dest_h);
    }


    fn on_button_press(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XButtonEvent) {
        //print_event!(wm, event);
        let modifiers = sanitize_modifiers(event.state);
        let client = Self::client_by_frame(wm, event.window);
        wm.handle_button(self, modifiers, event.button, client);
    }

    fn on_client_message(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XClientMessageEvent) {
        print_event!(wm, event);
        if let Some(atom) = X11Atom::from_xlib_atom(self.display, event.message_type) {
            match atom {
                NetActiveWindow => {
                    if let Some(client_rc) = Self::client_by_window(wm, event.window){
                        wm.activate_client(self, client_rc);
                    }
                },
                NetCloseWindow => {
                    if let Some(client_rc) = Self::client_by_window(wm, event.window) {
                        client_rc.borrow().close();
                    }
                },
                NetCurrentDesktop => {
                    let workspace = event.data.get_long(0).try_into().unwrap();
                    wm.switch_workspace(self, workspace);
                },
                NetWMDesktop => {
                    if let Some(client_rc) = Self::client_by_window(wm, event.window) {
                        if let Ok(workspace) = event.data.get_long(0).try_into() {
                            wm.move_to_workspace(self, client_rc, workspace);
                        }
                    }
                },
                NetWMState => {
                    if let Some(client_rc) = Self::client_by_window(wm, event.window) {
                        if event.data.get_long(1) as u64 == NetWMStateFullscreen.to_xlib_atom(self.display)
                                || event.data.get_long(2) as u64 == NetWMStateFullscreen.to_xlib_atom(self.display) {
                            let mode = event.data.get_long(0) as u64;
                            if mode == 1 {
                                wm.handle_fullscreen(self, client_rc, true);
                            } else if mode == 0 {
                                wm.handle_fullscreen(self, client_rc, false);
                            } else if mode == 2 {
                                wm.handle_fullscreen_toggle(self, client_rc);
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn on_configure_notify(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XConfigureEvent) {
        //print_event!(wm, event);
        if event.window == self.root {
            let monitor_configs = self.get_monitor_config();
            wm.update_monitor_config(monitor_configs);
        }
    }

    fn on_destroy_notify(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XDestroyWindowEvent) {
        print_event!(wm, event);
        let client_rc = match wm.clients().find(|c| c.borrow().window() == event.window) {
            Some(client_rc) => client_rc.clone(),
            None => return,
        };

        self.unmanage(wm, client_rc);
    }

    fn on_enter_notify(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XCrossingEvent) {
        //print_event!(wm, event);
        // if let Some(client_rc) = Self::client_by_frame(wm, event.window) {
        //     println!("EnterNotify on frame for client {}", client_rc.borrow().window());
        // }
        // if let Some(client_rc) = Self::client_by_window(wm, event.window) {
        //     println!("EnterNotify on window for client {}", client_rc.borrow().window());
        // }

        let client_option = if let Some(client_rc) = Self::client_by_frame(wm, event.window) {
            Some(client_rc)
        } else if let Some(client_rc) = Self::client_by_window(wm, event.window) {
            Some(client_rc)
        } else {
            unsafe {
                xlib::XSetInputFocus(self.display, event.window, xlib::RevertToPointerRoot, xlib::CurrentTime);
            }
            None
        };

        if let Some(client_rc) = client_option {
            if let Some(last_active_client) = &self.last_active {
                wm.handle_unfocus(self, last_active_client.clone());
            }
            wm.handle_focus(self, Some(client_rc.clone()));
            self.last_active = Some(client_rc);
        }
    }

    fn on_key_press(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XKeyEvent) {
        //print_event!(wm, event);

        let keysym = unsafe {
            xlib::XKeycodeToKeysym(self.display, event.keycode.try_into().unwrap(), 0)
        };

        let modifiers = sanitize_modifiers(event.state);
        let client_opt = Self::client_by_frame(wm, event.window);
        let key: u32 = keysym.try_into().unwrap();
        wm.handle_key(self, modifiers, key, client_opt)
    }

    fn on_leave_notify(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XCrossingEvent) {
        //print_event!(wm, event);
        // if let Some(client_rc) = Self::client_by_frame(wm, event.window) {
        //     println!("LeaveNotify on frame for client {}", client_rc.borrow().window());
        // }
        // if let Some(client_rc) = Self::client_by_window(wm, event.window) {
        //     println!("LeaveNotify on window for client {}", client_rc.borrow().window());
        // }

        // if let Some(client_rc) = Self::client_by_frame(wm, event.window) {
        //     wm.handle_unfocus(self, client_rc.clone());
        // } else if let Some(client_rc) = Self::client_by_window(wm, event.window) {
        //     wm.handle_unfocus(self, client_rc.clone());
        // }
    }

    fn on_unmap_notify(&mut self, wm: &mut dyn WindowManager<X11Backend,X11Client>, event: xlib::XUnmapEvent) {
        print_event!(wm, event);
        let root = self.root;
        let client_option = if let Some(client_rc) = Self::client_by_frame(wm, event.window) {
            Some(client_rc)
        } else if let Some(client_rc) = Self::client_by_window(wm, event.window) {
            Some(client_rc)
        } else {
            None
        };
        let client_rc = match client_option {
            Some(client_rc) => client_rc.clone(),
            None => return,
        };

        // ignore unmap notifies generated from reparenting
        if event.event == root || client_rc.borrow().is_reparenting() {
            client_rc.borrow_mut().set_reparenting(false);
            return;
        } else if event.send_event == xlib::True {
            client_rc.borrow_mut().x11_set_state(self.display, WITHDRAWN_STATE);
        } else {
            self.unmanage(wm, client_rc);
        }
    }

    fn on_map_request(&mut self, wm: &mut WM, event: xlib::XMapRequestEvent) {
        print_event!(wm, event);
        let already_managed = wm.clients().find(|c| c.borrow().window() == event.window).is_some();
        if !already_managed {
            self.manage(wm, event.window);
        }
    }

    fn on_map_notify(&mut self, wm: &mut WM, event: xlib::XMapEvent) {
        print_event!(wm, event);
    }

    fn set_supported_atoms(&mut self, supported_atoms: &[X11Atom]) {
        let atom_vec: Vec<xlib::Atom> = (*supported_atoms).iter().map(|a| a.to_xlib_atom(self.display)).collect();
        let data = atom_vec.as_slice();
        self.root.x11_replace_property_long(self.display, X11Atom::NetSupported.to_xlib_atom(self.display), xlib::XA_ATOM, data)
    }

    fn unmanage(&mut self, wm: &mut WM, client_rc: Rc<RefCell<X11Client>>) {
        println!("Closing client: {}", client_rc.borrow().name());

        // tell window manager to drop client
        wm.unmanage(self, client_rc.clone());

        // drop reference from last active field
        if let Some(last_active) = &self.last_active {
            if &client_rc == last_active {
                self.last_active = None;
            }
        }

        let client = client_rc.borrow();

        // remove client frame
        client.destroy_frame();

        // set WM_STATE to Withdrawn according to ICCCM
        let data = [WITHDRAWN_STATE as u64, 0];
        let wm_state_atom = WMState.to_xlib_atom(self.display);
        client.window().x11_replace_property_long(self.display, wm_state_atom, wm_state_atom, &data);
        debug_assert!(Rc::strong_count(&client_rc) == 1);
    }

    fn client_by_frame<'a>(wm: &'a WM, frame: u64) -> Option<Rc<RefCell<X11Client>>> {
        return wm.clients().find(|c| c.borrow().frame() == frame).cloned();
    }

    fn client_by_window<'a>(wm: &'a WM, window: u64) -> Option<Rc<RefCell<X11Client>>> {
        return wm.clients().find(|c| c.borrow().window() == window).cloned();
    }
}

impl Backend<X11Client> for X11Backend {
    fn export_active_window(&self, client_option: &Option<Rc<RefCell<X11Client>>>) {
        let window = match client_option {
            Some(client_rc) => client_rc.borrow().window(),
            None => XLIB_NONE,
        };
        let data = &[window];
        self.root.x11_replace_property_long(self.display, NetActiveWindow.to_xlib_atom(self.display), xlib::XA_WINDOW, data);
    }

    fn export_client_list(&self, clients: &Vec<Rc<RefCell<X11Client>>>) {
        // TODO ensure correct sorting as defined by EWMH
        let data_vec: Vec<u64> = clients.iter().map(|c| c.borrow().window()).collect();
        let data = data_vec.as_slice();
        self.root.x11_replace_property_long(self.display, X11Atom::NetClientList.to_xlib_atom(self.display), xlib::XA_WINDOW, data);
        self.root.x11_replace_property_long(self.display, X11Atom::NetClientListStacking.to_xlib_atom(self.display), xlib::XA_WINDOW, data);
    }

    fn export_current_workspace(&self, workspace_idx: usize) {
        let idx: u64 = workspace_idx.try_into().unwrap();
        let data = &[idx];
        self.root.x11_replace_property_long(self.display, NetCurrentDesktop.to_xlib_atom(self.display), xlib::XA_CARDINAL, data);
    }

    fn export_workspaces(&self, workspaces: Vec<String>) {
        // export number of workspaces
        let nworkspaces: u64 = workspaces.len().try_into().unwrap();
        let data = &[nworkspaces];
        self.root.x11_replace_property_long(self.display, NetNumberOfDesktops.to_xlib_atom(self.display), xlib::XA_CARDINAL, data);

        // export workspace names
        let cstrings: Vec<CString> = workspaces.iter().map(|s| CString::new(s.as_str()).unwrap()).collect();
        self.root.x11_set_text_list_property(self.display, NetDesktopNames.to_xlib_atom(self.display), cstrings);

    }

    fn get_monitor_config(&self) -> Vec<MonitorConfig> {
        unsafe {
            if xinerama::XineramaIsActive(self.display) != 0 {
                let mut screen_count = 0;
                let screens_raw = xinerama::XineramaQueryScreens(self.display, &mut screen_count);
                let screens_slice = slice::from_raw_parts_mut(screens_raw, screen_count.try_into().unwrap());
                let configs =  screens_slice.iter().map(|x| MonitorConfig::from(*x)).collect();
                xlib::XFree(screens_slice.as_mut_ptr() as *mut c_void);
                return configs;
            } else {
                let w = xlib::XWidthOfScreen(self.screen).try_into().unwrap();
                let h = xlib::XHeightOfScreen(self.screen).try_into().unwrap();
                let dims = Dimensions { x: 0, y: 0, w, h };

                return vec![
                    MonitorConfig { num: 0, dims, win_area: dims }
                ];
            }
        }
    }

    fn handle_existing_windows(&mut self, wm: &mut WM) {
        unsafe {
            xlib::XGrabServer(self.display);
            let mut returned_root: xlib::Window = 0;
            let mut returned_parent: xlib::Window = 0;
            let mut top_level_windows: *mut xlib::Window = ptr::null_mut();
            let mut num_top_level_windows: u32 = 0;

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

    fn mouse_move(&mut self, wm: &mut WM, client_rc: Rc<RefCell<X11Client>>, _button: u32) {
        self.mouse_action(wm, client_rc, CURSOR_MOVE, Self::mouse_action_move);
    }

    fn mouse_resize(&mut self, wm: &mut WM, client_rc: Rc<RefCell<X11Client>>, _button: u32) {
        self.mouse_action(wm, client_rc, CURSOR_RESIZE, Self::mouse_action_resize);
    }

    fn point_to_monitor(&self, point: (i32, i32)) -> Option<u32> {
        for mon in self.get_monitor_config() {
            if point.0 >= mon.dimensions().x()
                && point.0 < mon.dimensions().x() + mon.dimensions().w() as i32
                && point.1 >= mon.dimensions().y()
                && point.1 < mon.dimensions().y() + mon.dimensions().h() as i32 {
                    return Some(mon.num());
            }
        }
        return None;
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
        if !client.is_visible() {
            return;
        }

        unsafe {
            xlib::XSetInputFocus(self.display, client.window(), xlib::RevertToPointerRoot, xlib::CurrentTime);
        }
    }

    fn run(mut self, wm: &mut WM) {
        loop {
            unsafe {
                let mut event: xlib::XEvent = MaybeUninit::uninit().assume_init();
                xlib::XNextEvent(self.display, &mut event);
                self.handle_xevent(wm, event);
            };
        }
    }
}

fn event_type<T>(_: &T) -> &str {
    return std::any::type_name::<T>();
}


