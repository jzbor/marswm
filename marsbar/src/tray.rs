extern crate x11;

use libmars::common::*;
use libmars::common::x11::atoms::X11Atom::*;
use libmars::common::x11::window::X11Window;
use libmars::draw::*;
use libmars::draw::x11::canvas::*;
use libmars::draw::x11::widget::*;
use std::cmp;
use std::mem::MaybeUninit;
use x11::xlib;

const SYSTEM_TRAY_REQUEST_DOCK: i64 = 0;
const _SYSTEM_TRAY_BEGIN_MESSAGE: i64 = 1;
const _SYSTEM_TRAY_CANCEL_MESSAGE: i64 = 2;
const _NET_SYSTEM_TRAY_ORIENTATION_HORZ: u64 = 0;
const _NET_SYSTEM_TRAY_ORIENTATION_VERT: u64 = 1;

/* XEMBED messages */
const XEMBED_EMBEDDED_NOTIFY: u64 = 0;
const _XEMBED_FOCUS_IN: u64 = 4;
const _XEMBED_MODALITY_ON: u64 = 10;

const _XEMBED_MAPPED: u64 = 1 << 0;
const _XEMBED_WINDOW_ACTIVATE: u64 = 1;
const _XEMBED_WINDOW_DEACTIVATE: u64 = 2;

const VERSION_MAJOR: u64 = 0;
const VERSION_MINOR: u64 = 0;
const XEMBED_EMBEDDED_VERSION: u64 = (VERSION_MAJOR << 16) | VERSION_MINOR;


pub struct SystemTrayWidget {
    display: *mut xlib::Display,
    tray_icons: Vec<xlib::Window>,
    window: xlib::Window,
    canvas: X11Canvas,
    event_handlers: Vec<Box<dyn WidgetEventHandler>>,
    width: u32,
    height: u32,
    min_size: (u32, u32),
    max_size: (u32, u32),
    icon_width: u32,
    ipad: u32,
    hpad: u32,
    vpad: u32,
    bg_color: u64,
}


impl SystemTrayWidget {
    pub fn new(display: *mut xlib::Display, parent: xlib::Window, x: i32, y: i32, height: u32,
               ipad: u32, hpad: u32, vpad: u32, bg_color: u64) -> Result<SystemTrayWidget, String> {
        if height < 2*vpad {
            return Err(format!("Padding bigger than height (h: {}, vpad: {})", height, vpad));
        }

        let outer_dimensions = Dimensions::new(x, y, MIN_SIZE.0, MIN_SIZE.1);
        let root = unsafe { xlib::XDefaultRootWindow(display) };
        let window = create_widget_window(display, parent, outer_dimensions)?;
        let mut canvas = X11Canvas::new_for_window(display, window)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;

        canvas.set_foreground(bg_color)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;
        canvas.set_background(bg_color)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;

        // get required input selections (the default doesn't fit very well)
        let mask = xlib::StructureNotifyMask | xlib::SubstructureNotifyMask | xlib::ExposureMask;
        unsafe { xlib::XSelectInput(display, window, mask); }

        // configure tray window
        let data = [_NET_SYSTEM_TRAY_ORIENTATION_HORZ];
        window.x11_replace_property_long(display, NetSystemTrayOrientation, xlib::XA_CARDINAL, &data);

        // get tray selection
        unsafe {
            xlib::XSetSelectionOwner(display, NetSystemTray.to_xlib_atom(display), window, xlib::CurrentTime);

            // check if we got the selection
            if xlib::XGetSelectionOwner(display, NetSystemTray.to_xlib_atom(display)) != window {
                xlib::XDestroyWindow(display, window);
                return Err("unable to get system tray selection".to_owned());
            }

            send_manager_message(display, root, NetSystemTray.to_xlib_atom(display), window, 0, 0);
        }

        let widget = SystemTrayWidget {
            display,
            tray_icons: Vec::new(),
            window, canvas,
            event_handlers: Vec::new(),
            width: height, height,
            min_size: MIN_SIZE,
            max_size: MAX_SIZE,
            icon_width: height - 2*vpad,
            ipad, hpad, vpad,
            bg_color,
        };

        Ok(widget)
    }

    fn dock(&mut self, tray_icon: xlib::Window) {
        let nicons = self.tray_icons.len();
        unsafe {
            // select events for window
            xlib::XSelectInput(self.display, tray_icon, xlib::ResizeRedirectMask);

            // reparent window
            let x = (self.hpad + nicons as u32 * (self.icon_width + self.ipad)) as i32;
            let y = self.vpad as i32;
            xlib::XAddToSaveSet(self.display, tray_icon);
            xlib::XReparentWindow(self.display, tray_icon, self.window, x, y);
            xlib::XResizeWindow(self.display, tray_icon, self.icon_width, self.icon_width);

            // set background
            let mut swa: xlib::XSetWindowAttributes = MaybeUninit::zeroed().assume_init();
            swa.background_pixel = self.bg_color;
            swa.backing_store = xlib::ParentRelative;
			xlib::XChangeWindowAttributes(self.display, tray_icon, xlib::CWBackPixel | xlib::CWBackPixmap, &mut swa);

            // notify window
            send_xembed_message(self.display, tray_icon, XEMBED_EMBEDDED_NOTIFY, 0, self.window, XEMBED_EMBEDDED_VERSION);

            // map window
            xlib::XMapRaised(self.display, tray_icon);
        }


        tray_icon.x11_set_state(self.display, libmars::common::x11::NORMAL_STATE);
        self.tray_icons.push(tray_icon);

        self.resize_to_content();
    }

    pub fn handle_systray_event(&mut self, event: xlib::XClientMessageEvent) {
        if event.window != self.window {
            return;
        }

        match event.data.get_long(1) {
            SYSTEM_TRAY_REQUEST_DOCK => self.dock(event.data.get_long(2) as u64),
            _ => (),
        }
    }

    pub fn handle_icon_destroyed(&mut self, event: xlib::XDestroyWindowEvent) {
        if event.event != self.window {
            return;
        }

        // remove window from list
        self.tray_icons.retain(|w| *w != event.window);

        for (i, icon) in self.tray_icons.iter().enumerate() {
            unsafe {
                xlib::XMoveWindow(self.display, *icon,
                                  self.hpad as i32 + i as i32 * (self.icon_width + self.ipad) as i32,
                                  self.vpad as i32);
            }
        }

        self.resize_to_content();
    }

    fn resize_to_content(&mut self) {
        let nicons = self.tray_icons.len();
        self.width = if nicons > 0 {
            nicons as u32 * (self.icon_width + self.ipad) - self.ipad + 2*self.hpad
        } else {
            self.min_size.0
        };
        self.height = self.height;
        self.width = cmp::min(cmp::max(self.width, self.min_size.0), self.max_size.0);
        self.height = cmp::min(cmp::max(self.height, self.min_size.1), self.max_size.1);

        unsafe {
            xlib::XResizeWindow(self.display, self.window, self.width, self.height);
        }

        self.canvas.match_resize();
    }
}


impl Widget for SystemTrayWidget {
    fn move_to(&mut self, x: i32, y: i32) {
        unsafe {
            xlib::XMoveWindow(self.display, self.window, x, y);
        }
        self.redraw();
    }

    fn redraw(&mut self) {
        self.canvas.fill_rectangle(0, 0, self.width, self.height);
        self.canvas.flush();
    }

    fn register_event_handler(&mut self, event_handler: Box<dyn WidgetEventHandler>) {
        self.event_handlers.push(event_handler);
    }

    fn handle_xevent(&mut self, event: xlib::XEvent) -> bool {
        unsafe {
            if event.any.window == self.window {
                let widget_event = match event.get_type() {
                    xlib::ButtonPress => {
                        let button = event.button.button;
                        Some(WidgetEvent::ButtonPressed(button))
                    },
                    xlib::Expose => {
                        self.redraw();
                        None
                    }
                    _ => None,
                };

                if let Some(widget_event) = widget_event {
                    let _handled = self.event_handlers.iter()
                        .fold(false, {
                            |already_handled, handler| handler.handle_action_event(widget_event, already_handled)
                        });
                }
                true
            } else {
                false
            }
        }
    }

    fn set_max_size(&mut self, max_size: (u32, u32)) {
        self.max_size = max_size;
        self.resize_to_content();
    }

    fn set_min_size(&mut self, min_size: (u32, u32)) {
        self.min_size = min_size;
        self.resize_to_content();
    }
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn wid(&self) -> xlib::Window {
        self.window
    }
}


fn send_manager_message(display: *mut xlib::Display, root: xlib::Window, selection: u64, window: u64, data0: u64, data1: u64) {
    let mut data = xlib::ClientMessageData::new();
    data.set_long(0, xlib::CurrentTime as i64);
    data.set_long(1, selection as i64);
    data.set_long(2, window as i64);
    data.set_long(3, data0 as i64);
    data.set_long(4, data1 as i64);

    let ce = xlib::XClientMessageEvent {
        type_: xlib::ClientMessage,
        serial: 0,
        send_event: xlib::True,
        display, window,
        message_type: Manager.to_xlib_atom(display),
        format: 32,
        data,
    };

    let mut event = xlib::XEvent::from(ce);

    unsafe {
        xlib::XSendEvent(display, root, xlib::False, xlib::StructureNotifyMask, &mut event);
        xlib::XSync(display, xlib::False);
    }
}
fn send_xembed_message(display: *mut xlib::Display, window: xlib::Window, message: u64, detail: u64, data0: u64, data1: u64) {
    let mut data = xlib::ClientMessageData::new();
    data.set_long(0, xlib::CurrentTime as i64);
    data.set_long(1, message as i64);
    data.set_long(2, detail as i64);
    data.set_long(3, data0 as i64);
    data.set_long(4, data1 as i64);

    let ce = xlib::XClientMessageEvent {
        type_: xlib::ClientMessage,
        serial: 0,
        send_event: xlib::True,
        display, window,
        message_type: Xembed.to_xlib_atom(display),
        format: 32,
        data,
    };

    let mut event = xlib::XEvent::from(ce);

    unsafe {
        xlib::XSendEvent(display, window, xlib::False, xlib::NoEventMask, &mut event);
        xlib::XSync(display, xlib::False);
    }
}
