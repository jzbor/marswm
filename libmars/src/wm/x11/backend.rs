extern crate x11;

use std::ffi::*;
use x11::xlib;
use x11::xrandr;
use std::ptr;
use std::slice;
use std::mem::MaybeUninit;

use crate::wm::*;
use crate::wm::x11::*;
use crate::common::x11::*;
use crate::common::x11::atoms::*;
use crate::common::x11::atoms::X11Atom::*;
use crate::wm::x11::client::*;
use crate::common::x11::window::*;


#[allow(unused_macros)]
macro_rules! print_event {
    ($wm:expr, $event:expr) => {
        #[cfg(debug_assertions)]
        if let Some(client) = $wm.clients().find(|c| c.borrow().window() == $event.window) {
            eprintln!("Received {} for window 0x{:x} (window of {})", event_type(&$event), $event.window, client.borrow().name());
        } else if let Some(client) = $wm.clients().find(|c| c.borrow().frame() == $event.window) {
            eprintln!("Received {} for window 0x{:x} (frame of {})", event_type(&$event), $event.window, client.borrow().name());
        } else {
            eprintln!("Received {} for window 0x{:x} (not a client)", event_type(&$event), $event.window);
        }
        #[cfg(debug_assertions)]
        eprintln!("\t{:?}", $event);
        #[cfg(not(debug_assertions))]
        let (_, _) = (&$wm, &$event);
    }
}

type WM<'a, A> = dyn WindowManager<X11Backend<A>, A> + 'a;

pub struct X11Backend<A: PartialEq> {
    display: *mut xlib::Display,
    root: u64,
    xrandr: XRandrInfo,
    monitors: Vec<MonitorConfig>,
    wmcheck_win: u64,
    dock_windows: Vec<xlib::Window>,
    last_active: Option<Rc<RefCell<X11Client<A>>>>,
}

struct XRandrInfo {
    supported: bool,
    event_base: i32,
    _error_base: i32,
}


const SUPPORTED_ATOMS: &[X11Atom; 21] = & [
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
    NetWMWindowTypeDesktop,
    NetWMWindowTypeDialog,
    NetWMWindowTypeDock,
    NetWMWindowTypeMenu,
    NetWMWindowTypeNotification,
    NetWorkarea,

    MarsWMStateTiled,
];


impl<A: PartialEq + Default> X11Backend<A> {
    /// Register window manager and initialize backend with new connection.
    pub fn init(name: &str) -> Result<X11Backend<A>> {
        // open new connection to x11 server
        let display = open_display()?;
        return Self::init_with_connection(display, name);
    }

    /// Register window manager and create backend from existing connection.
    pub fn init_with_connection(display: *mut xlib::Display, name: &str) -> Result<X11Backend<A>> {
        unsafe {
            let root = xlib::XDefaultRootWindow(display);

            let mut x11b = X11Backend {
                display,
                root,
                xrandr: XRandrInfo::query(display),
                monitors: Vec::new(),
                wmcheck_win: 0,
                dock_windows: Vec::new(),
                last_active: None,
            };

            // For debugging:
            // #[cfg(debug_assertions)]
            // xlib::XSynchronize(display, 1);

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

            if x11b.xrandr.supported {
                xrandr::XRRSelectInput(display, root, xrandr::RRCrtcChangeNotifyMask);
            }

            x11b.set_supported_atoms(SUPPORTED_ATOMS);
            x11b.monitors = query_monitor_config(display, true);

            return Ok(x11b);
        }
    }

    fn apply_dock_insets(&mut self) {
        self.monitors.iter_mut().for_each(|m| m.remove_insets());

        for dock in &self.dock_windows {
            let dimensions = match dock.x11_dimensions(self.display) {
                Ok(dimensions) => dimensions,
                Err(_) => continue,
            };

            if let Some(mon) = self.monitors.iter_mut().find(|m| m.contains_point(dimensions.center())) {
                // apply top indent
                if dimensions.center().1 < mon.dimensions().center().1 {
                    let inset = dimensions.bottom() - mon.dimensions().y();
                    mon.add_inset_top(inset as u32);
                } else {
                    let inset = mon.dimensions().bottom() - dimensions.y();
                    mon.add_inset_bottom(inset as u32);
                }
            }
        }
    }

    fn send_configure_notify(&self, client_rc: Rc<RefCell<X11Client<A>>>) {
        let client = client_rc.borrow();
        let inner_dimensions = client.inner_dimensions();
        let ce = xlib::XConfigureEvent {
            type_: xlib::ConfigureNotify,
            serial: 0,
            send_event: xlib::True,
            display: self.display,
            event: client.window(),
            window: client.window(),
            x: inner_dimensions.x(),
            y: inner_dimensions.y(),
            width: inner_dimensions.w() as i32,
            height: inner_dimensions.h() as i32,
            border_width: client.inner_bw() as i32,
            above: XLIB_NONE,
            override_redirect: xlib::False,
        };
        let mut xevent = xlib::XEvent::from(ce);
        unsafe {
            xlib::XSendEvent(self.display, client.window(), xlib::False, xlib::StructureNotifyMask, &mut xevent);
        }
    }

    fn handle_xevent(&mut self, wm: &mut WM<A>, event: xlib::XEvent) {
        unsafe {  // unsafe because of access to union field
            if self.xrandr.supported && event.get_type() == self.xrandr.event_base + xrandr::RRNotify {
                self.monitors = query_monitor_config(self.display, true);
                self.apply_dock_insets();
                wm.update_monitor_config(self, self.monitors.clone());
                return;
            }

            match event.get_type() {
                xlib::ButtonPress => self.on_button_press(wm, event.button),
                xlib::ClientMessage => self.on_client_message(wm, event.client_message),
                xlib::ConfigureNotify => self.on_configure_notify(wm, event.configure),
                xlib::ConfigureRequest => self.on_configure_request(wm, event.configure_request),
                xlib::DestroyNotify => self.on_destroy_notify(wm, event.destroy_window),
                xlib::EnterNotify => self.on_enter_notify(wm, event.crossing),
                xlib::KeyPress => self.on_key_press(wm, event.key),
                xlib::LeaveNotify => self.on_leave_notify(wm, event.crossing),
                xlib::MapRequest => self.on_map_request(wm, event.map_request),
                xlib::MapNotify => self.on_map_notify(wm, event.map),
                xlib::UnmapNotify => self.on_unmap_notify(wm, event.unmap),
                xlib::PropertyNotify => self.on_property_notify(wm, event.property),
                _ => (),
                // _ => { print!("."); stdout().flush().unwrap(); },
            }
        }
    }

    /// Create a new client for the window and give it to the window manager
    fn manage(&mut self, wm: &mut WM<A>, window: xlib::Window) {
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
        let window_types: Vec<X11Atom> = window.x11_get_window_types(self.display);
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
                    self.dock_windows.push(window);
                    self.apply_dock_insets();
                    wm.update_monitor_config(self, self.monitors.clone());
                    return;
                },
                NetWMWindowTypeMenu => unsafe {
                    xlib::XSelectInput(self.display, window, xlib::EnterWindowMask | xlib::LeaveWindowMask);
                    xlib::XMapRaised(self.display, window);
                    return;
                },
                NetWMWindowTypeNotification => unsafe {
                    xlib::XMapRaised(self.display, window);
                    return;
                }
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

        if let Ok(mut client) = X11Client::new(self.display, self.root, window, is_dialog) {
            // client.apply_size_hints();
            client.apply_motif_hints();

            // println!("New client: {} (frame: {}) with types {:?}", client.name(), client.frame(), window_types);

            // Setting workspace as specified by _NET_WM_DESKTOP
            let workspace_req = {
                match client.x11_read_property_long(self.display, NetWMDesktop, xlib::XA_CARDINAL) {
                    // FIXME handle -1 as value for all desktops
                    Ok(data) => {
                        if data[0] == u64::MAX {
                            // TODO pin client
                            None
                        } else if data[0] >= u32::MAX.into() {
                            // value to big
                            None
                        } else {
                            Some(data[0] as u32)
                        }
                    },
                    Err(_msg) => None,
                }
            };

            let boxed_client = Rc::new(RefCell::new(client));
            wm.manage(self, boxed_client, workspace_req);
        } else {
            // Unable to manage window
            unsafe {
                xlib::XMapRaised(self.display, window);
            }
        }

        // TODO move transient clients to workspace and monitor of their counterpart
    }

    fn mouse_action(&mut self, wm: &mut dyn WindowManager<X11Backend<A>, A>,
                    client_rc: Rc<RefCell<X11Client<A>>>, cursor_type: u32,
                    action: fn(&mut Self, &Rc<RefCell<X11Client<A>>>, (i32, i32), (u32, u32), (i32, i32))) {
        unsafe {
            // grab pointer
            let cursor = xlib::XCreateFontCursor(self.display, cursor_type);
            let success = xlib::XGrabPointer(self.display, self.root, xlib::False, MOUSEMASK as u32,
                    xlib::GrabModeAsync, xlib::GrabModeAsync, XLIB_NONE, cursor, xlib::CurrentTime);
            if success != xlib::GrabSuccess {
                xlib::XFreeCursor(self.display, cursor);
                return;
            }

            let orig_client_pos = client_rc.borrow().pos();
            let orig_client_size = client_rc.borrow().size();
            let orig_pointer_pos = self.pointer_pos();
            let mut event: MaybeUninit<xlib::XEvent> = MaybeUninit::uninit();

            loop {
                xlib::XMaskEvent(self.display, MOUSEMASK | xlib::ExposureMask | xlib::SubstructureRedirectMask, event.as_mut_ptr());
                let event = event.assume_init();

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
                        if let Some(new_mon) = self.point_to_monitor(new_center) {
                            if old_mon != new_mon {
                                wm.client_switches_monitor(client_rc.clone(), new_mon);
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

    fn mouse_action_move(&mut self, client_rc: &Rc<RefCell<X11Client<A>>>, orig_client_pos: (i32, i32),
                         _orig_client_size: (u32, u32), delta: (i32, i32)) {
        let dest_x = orig_client_pos.0 + delta.0;
        let dest_y = orig_client_pos.1 + delta.1;
        let size = client_rc.borrow().size();
        client_rc.borrow_mut().move_resize(dest_x, dest_y, size.0, size.1);
    }

    fn mouse_action_resize(&mut self, client_rc: &Rc<RefCell<X11Client<A>>>, _orig_client_pos: (i32, i32),
                         orig_client_size: (u32, u32), delta: (i32, i32)) {
        let dest_w = orig_client_size.0 as i32 + delta.0;
        let dest_h = orig_client_size.1 as i32 + delta.1;
        let pos = client_rc.borrow().pos();
        let dest_w: u32 = if dest_w < WINDOW_MIN_SIZE as i32 { WINDOW_MIN_SIZE } else { dest_w as u32 };
        let dest_h: u32 = if dest_h < WINDOW_MIN_SIZE as i32 { WINDOW_MIN_SIZE } else { dest_h as u32 };
        client_rc.borrow_mut().move_resize(pos.0, pos.1, dest_w, dest_h);
    }

    fn on_button_press(&mut self, wm: &mut dyn WindowManager<Self, A>, event: xlib::XButtonEvent) {
        //print_event!(wm, event);
        let modifiers = sanitize_modifiers(event.state);
        let client = Self::client_by_frame(wm, event.window);
        wm.handle_button(self, modifiers, event.button, client);
    }

    fn on_client_message(&mut self, wm: &mut dyn WindowManager<Self, A>, event: xlib::XClientMessageEvent) {
        //print_event!(wm, event);
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
                    } else {
                        event.window.x11_close(self.display, Some(on_error));
                    }
                },
                NetCurrentDesktop => {
                    if let Ok(workspace) = event.data.get_long(0).try_into() {
                        wm.switch_workspace(self, workspace);
                    }
                },
                NetWMDesktop => {
                    if let Some(client_rc) = Self::client_by_window(wm, event.window) {
                        let workspace = event.data.get_long(0);
                        wm.set_client_pinned(self, client_rc.clone(), workspace == -1);
                        if workspace != -1 {
                            wm.move_to_workspace(self, client_rc, workspace as u32);
                        }
                    }
                },
                NetWMState => {
                    if let Some(client_rc) = Self::client_by_window(wm, event.window) {
                        if event.data.get_long(1) as u64 == NetWMStateFullscreen.to_xlib_atom(self.display)
                                || event.data.get_long(2) as u64 == NetWMStateFullscreen.to_xlib_atom(self.display) {
                            let mode = event.data.get_long(0) as u64;
                            if mode == 1 {
                                wm.fullscreen_client(self, client_rc, true);
                            } else if mode == 0 {
                                wm.fullscreen_client(self, client_rc, false);
                            } else if mode == 2 {
                                wm.toggle_fullscreen_client(self, client_rc);
                            }
                        } else if event.data.get_long(1) as u64 == MarsWMStateTiled.to_xlib_atom(self.display)
                                || event.data.get_long(2) as u64 == MarsWMStateTiled.to_xlib_atom(self.display) {
                            let mode = event.data.get_long(0) as u64;
                            if mode == 1 {
                                wm.tile_client(self, client_rc, true);
                            } else if mode == 0 {
                                wm.tile_client(self, client_rc, false);
                            } else if mode == 2 {
                                wm.toggle_tile_client(self, client_rc);
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    fn on_configure_notify(&mut self, wm: &mut dyn WindowManager<Self, A>, event: xlib::XConfigureEvent) {
        //print_event!(wm, event);
        if event.window == self.root && !self.xrandr.supported {
            self.monitors = query_monitor_config(self.display, true);
            self.apply_dock_insets();
            wm.update_monitor_config(self, self.monitors.clone());
        }
    }

    fn on_configure_request(&mut self, wm: &mut dyn WindowManager<Self, A>, event: xlib::XConfigureRequestEvent) {
        let client = wm.clients().find(|c| c.borrow().window() == event.window).cloned();
        let inner = wm.clients().find(|c| c.borrow().window() == event.window).map(|c| c.borrow().inner_dimensions());
        if let Some(client_rc) = client {
            if let Some(inner) = inner {
                // get dimensions from event
                let x = if event.value_mask & (xlib::CWX as u64) != 0 {
                    event.x
                } else {
                    inner.x()
                };
                let y = if event.value_mask & (xlib::CWY as u64) != 0 {
                    event.y
                } else {
                    inner.y()
                };
                let width = if event.value_mask & (xlib::CWWidth as u64) != 0 {
                    event.width as u32
                } else {
                    inner.w()
                };
                let height = if event.value_mask & (xlib::CWHeight as u64) != 0 {
                    event.height as u32
                } else {
                    inner.h()
                };

                let mut window_changed = false;
                let (bw_north, bw_east, bw_south, bw_west) = client_rc.borrow().total_bw();

                // issue move request if size is different
                if x != inner.x() || y != inner.y() {
                    let client = client_rc.borrow();
                    // subtract border to size
                    let x = x - bw_east as i32;
                    let y = y - bw_north as i32;
                    drop(client);
                    // note that only moving might not generate a real ConfigureNotify
                    // therefore we ignore the result of a move_request
                    // window_changed |= wm.move_request(self, client_rc, x, y);
                    wm.move_request(self, client_rc.clone(), x, y);
                }

                // issue resize request if size is different
                if width != inner.w() || height != inner.h() {
                    let client = client_rc.borrow();
                    // add border to size
                    let width = width + bw_east + bw_west;
                    let height = height + bw_north + bw_south;
                    drop(client);
                    window_changed |= wm.resize_request(self, client_rc.clone(), width, height);
                }

                // send synthetic ConfigureNotify if the dimensions were not changed
                if !window_changed {
                    self.send_configure_notify(client_rc);
                }
            }
        } else {
            let mut wc = xlib::XWindowChanges {
                x: event.x,
                y: event.y,
                width: event.width,
                height: event.height,
                border_width: event.border_width,
                sibling: event.above,
                stack_mode: event.detail,
            };
            unsafe {
                xlib::XConfigureWindow(self.display, event.window, event.value_mask as u32, &mut wc);
            }
        }
    }

    fn on_destroy_notify(&mut self, wm: &mut dyn WindowManager<Self, A>, event: xlib::XDestroyWindowEvent) {
        //print_event!(wm, event);

        // unmanage dock window
        if let Some(index) = self.dock_windows.iter().position(|w| *w == event.window) {
            self.dock_windows.swap_remove(index);
            self.apply_dock_insets();
            wm.update_monitor_config(self, self.monitors.clone());
        }

        let client_rc = match wm.clients().find(|c| c.borrow().window() == event.window) {
            Some(client_rc) => client_rc.clone(),
            None => return,
        };

        // unmanage clients
        self.unmanage(wm, client_rc);
    }

    fn on_enter_notify(&mut self, wm: &mut dyn WindowManager<Self, A>, event: xlib::XCrossingEvent) {
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
                wm.unfocus_client(self, last_active_client.clone());
            }
            wm.focus_client(self, Some(client_rc.clone()));
            self.last_active = Some(client_rc);
        }
    }

    fn on_key_press(&mut self, wm: &mut dyn WindowManager<Self, A>, event: xlib::XKeyEvent) {
        //print_event!(wm, event);

        let keysym = unsafe {
            xlib::XKeycodeToKeysym(self.display, event.keycode.try_into().unwrap(), 0)
        };

        let modifiers = sanitize_modifiers(event.state);
        let client_opt = Self::client_by_frame(wm, event.window);
        let key: u32 = keysym.try_into().unwrap();
        wm.handle_key(self, modifiers, key, client_opt)
    }

    fn on_leave_notify(&mut self, _wm: &mut dyn WindowManager<Self, A>, _event: xlib::XCrossingEvent) {
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

    fn on_unmap_notify(&mut self, wm: &mut dyn WindowManager<Self, A>, event: xlib::XUnmapEvent) {
        //print_event!(wm, event);
        // unmanage dock window
        if let Some(index) = self.dock_windows.iter().position(|w| *w == event.window) {
            self.dock_windows.swap_remove(index);
            self.apply_dock_insets();
            wm.update_monitor_config(self, self.monitors.clone());
        }

        let root = self.root;
        let client_option = if let Some(client_rc) = Self::client_by_frame(wm, event.window) {
            Some(client_rc)
        } else {
            Self::client_by_window(wm, event.window)
        };
        let client_rc = match client_option {
            Some(client_rc) => client_rc,
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

    fn on_map_request(&mut self, wm: &mut WM<A>, event: xlib::XMapRequestEvent) {
        //print_event!(wm, event);
        let already_managed = wm.clients().any(|c| c.borrow().window() == event.window);
        if !already_managed {
            self.manage(wm, event.window);
        }
    }

    fn on_map_notify(&mut self, _wm: &mut WM<A>, _event: xlib::XMapEvent) {
        //print_event!(wm, event);
    }

    pub fn on_property_notify(&mut self, wm: &mut WM<A>, event: xlib::XPropertyEvent) {
        if let Some(client_rc) = Self::client_by_window(wm, event.window) {
            if let Some(atom) = X11Atom::from_xlib_atom(self.display, event.atom) {
                match atom {
                    WMName => client_rc.borrow_mut().update_title(),
                    _ => (),
                }
            }
        }
    }

    fn set_supported_atoms(&mut self, supported_atoms: &[X11Atom]) {
        let atom_vec: Vec<xlib::Atom> = (*supported_atoms).iter().map(|a| a.to_xlib_atom(self.display)).collect();
        let data = atom_vec.as_slice();
        self.root.x11_replace_property_long(self.display, NetSupported, xlib::XA_ATOM, data)
    }

    fn unmanage(&mut self, wm: &mut WM<A>, client_rc: Rc<RefCell<X11Client<A>>>) {
        // eprintln!("Closing client: {}", client_rc.borrow().name());

        // tell window manager to drop client
        wm.unmanage(self, client_rc.clone());

        // drop reference from last active field
        if let Some(last_active) = &self.last_active {
            if &client_rc == last_active {
                self.last_active = None;
            }
        }

        let window = client_rc.borrow().window();

        // destroy window frame by dropping the reference
        debug_assert!(Rc::strong_count(&client_rc) == 1);
        drop(client_rc);

        // set WM_STATE to Withdrawn according to ICCCM
        let data = [WITHDRAWN_STATE as u64, 0];
        let wm_state_atom = WMState.to_xlib_atom(self.display);
        window.x11_replace_property_long(self.display, WMState, wm_state_atom, &data);
    }


    fn client_by_frame(wm: &WM<A>, frame: u64) -> Option<Rc<RefCell<X11Client<A>>>> {
        return wm.clients().find(|c| c.borrow().frame() == frame || c.borrow().title_window() == Some(frame)).cloned();
    }

    fn client_by_window(wm: &WM<A>, window: u64) -> Option<Rc<RefCell<X11Client<A>>>> {
        return wm.clients().find(|c| c.borrow().window() == window).cloned();
    }
}

impl XRandrInfo {
    pub fn query(display: *mut xlib::Display) -> XRandrInfo {
        let mut event_base = 0;
        let mut error_base = 0;
        let supported = unsafe {
            xrandr::XRRQueryExtension(display, &mut event_base, &mut error_base) != 0
        };

        return XRandrInfo { supported, event_base, _error_base: error_base };
    }
}

impl<A: PartialEq + Default> Backend<A> for X11Backend<A> {
    type Client = X11Client<A>;

    fn export_active_window(&self, client_option: &Option<Rc<RefCell<Self::Client>>>) {
        let window = match client_option {
            Some(client_rc) => client_rc.borrow().window(),
            None => XLIB_NONE,
        };
        let data = &[window];
        self.root.x11_replace_property_long(self.display, NetActiveWindow, xlib::XA_WINDOW, data);
    }

    fn export_client_list(&self, clients: Vec<&Rc<RefCell<Self::Client>>>,
                          clients_stacked: Vec<&Rc<RefCell<Self::Client>>>) {
        let data_vec: Vec<u64> = clients.iter().map(|c| c.borrow().window()).collect();
        let data_stacked_vec: Vec<u64> = clients_stacked.iter().map(|c| c.borrow().window()).collect();
        let data = data_vec.as_slice();
        let data_stacked = data_stacked_vec.as_slice();
        self.root.x11_replace_property_long(self.display, NetClientList, xlib::XA_WINDOW, data);
        self.root.x11_replace_property_long(self.display, NetClientListStacking, xlib::XA_WINDOW, data_stacked);

    }

    fn export_current_workspace(&self, workspace_idx: u32) {
        let idx: u64 = workspace_idx.into();
        let data = &[idx];
        self.root.x11_replace_property_long(self.display, NetCurrentDesktop, xlib::XA_CARDINAL, data);
    }

    fn export_workspaces(&self, mut workspaces: Vec<(String, Dimensions, Dimensions)>) {
        let nworkspaces: u64 = workspaces.len().try_into().unwrap();
        let mut names = Vec::new();
        let mut workareas = Vec::new();

        for (name, _dimensions, workarea) in workspaces.drain(..) {
            names.push(name);

            workareas.push(workarea.x() as i64 as u64);
            workareas.push(workarea.y() as i64 as u64);
            workareas.push(workarea.w() as u64);
            workareas.push(workarea.h() as u64);
        }

        // export number of workspaces
        let data = &[nworkspaces];
        self.root.x11_replace_property_long(self.display, NetNumberOfDesktops, xlib::XA_CARDINAL, data);

        // export workspace names
        self.root.x11_set_text_list_property(self.display, NetDesktopNames, &names);

        // export workareas
        let data = workareas.as_slice();
        self.root.x11_replace_property_long(self.display, NetWorkarea, xlib::XA_CARDINAL, data);
    }

    fn get_monitor_config(&self) -> Vec<MonitorConfig> {
        return self.monitors.clone();
    }

    fn handle_existing_windows(&mut self, wm: &mut WM<A>) {
        unsafe {
            xlib::XGrabServer(self.display);
            let mut returned_root: xlib::Window = 0;
            let mut returned_parent: xlib::Window = 0;
            let mut top_level_windows: *mut xlib::Window = ptr::null_mut();
            let mut num_top_level_windows: u32 = 0;

            let windows = match xlib::XQueryTree(self.display, self.root,
                                   &mut returned_root, &mut returned_parent,
                                   &mut top_level_windows, &mut num_top_level_windows) {
                0 => Err(()),
                _ => Ok(slice::from_raw_parts(top_level_windows, num_top_level_windows.try_into().unwrap())),
            }.expect("Unable to query x window tree");

            // closure to check and manage windows
            let display = self.display;
            let check_manage_window = |window: &&xlib::Window| {
                let attributes = match window.x11_attributes(display) {
                    Ok(attr) => attr,
                    Err(_) => return false, // unable to get attributes for client (ignoring client)
                };

                // FIXME also manage windows where state == IconicState
                return attributes.map_state == xlib::IsViewable;
            };

            // manage non-transient windows first
            windows.iter().filter(|w| w.x11_is_transient_for(display).is_none())
                .filter(check_manage_window).for_each(|w| self.manage(wm, *w));
            windows.iter().filter(|w| w.x11_is_transient_for(display).is_some())
                .filter(check_manage_window).for_each(|w| self.manage(wm, *w));

            println!("Initially managed {} windows", windows.len());

            xlib::XFree(top_level_windows as *mut c_void);
            xlib::XUngrabServer(self.display);
        }
    }

    fn mouse_move(&mut self, wm: &mut WM<A>, client_rc: Rc<RefCell<Self::Client>>, _button: u32) {
        // ignore fullscreen windows
        if client_rc.borrow().is_fullscreen() {
            return;
        }

        self.mouse_action(wm, client_rc, CURSOR_MOVE, Self::mouse_action_move);
    }

    fn mouse_resize(&mut self, wm: &mut WM<A>, client_rc: Rc<RefCell<Self::Client>>, _button: u32) {
        // ignore fullscreen windows
        if client_rc.borrow().is_fullscreen() {
            return;
        }

        client_rc.borrow_mut().warp_pointer_to_corner();
        self.mouse_action(wm, client_rc, CURSOR_RESIZE, Self::mouse_action_resize);
    }

    fn point_to_monitor(&self, point: (i32, i32)) -> Option<u32> {
        for (i, mon) in query_monitor_config(self.display, true).iter().enumerate() {
            if point.0 >= mon.dimensions().x()
                && point.0 < mon.dimensions().x() + mon.dimensions().w() as i32
                && point.1 >= mon.dimensions().y()
                && point.1 < mon.dimensions().y() + mon.dimensions().h() as i32 {
                    return Some(i as u32);
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

    fn set_input_focus(&self, client_rc: Rc<RefCell<Self::Client>>) {
        let client = (*client_rc).borrow();
        if !client.is_visible() {
            return;
        }

        unsafe {
            xlib::XSetInputFocus(self.display, client.window(), xlib::RevertToPointerRoot, xlib::CurrentTime);
        }
    }

    fn shutdown(&mut self) {
        unsafe {
            xlib::XCloseDisplay(self.display);
        }
    }

    fn warp_pointer(&self, x: i32, y: i32) {
        unsafe {
            // this might fail (best effort)
            xlib::XWarpPointer(self.display, 0, self.root, 0, 0, 0, 0, x, y);
        }
    }

    fn run(mut self, wm: &mut WM<A>) {
        loop {
            unsafe {
                let mut event: MaybeUninit<xlib::XEvent> = MaybeUninit::uninit();
                xlib::XNextEvent(self.display, event.as_mut_ptr());
                self.handle_xevent(wm, event.assume_init());
            };
        }
    }
}

#[allow(dead_code)]
fn event_type<T>(_: &T) -> &str {
    return std::any::type_name::<T>();
}
