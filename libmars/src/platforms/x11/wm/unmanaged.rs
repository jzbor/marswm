use x11::xlib;
use crate::platforms::x11::misc::window::X11Window;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum UnmanagedType {
    Desktop, Dock, Notification,
}

pub struct UnmanagedClient {
    display: *mut xlib::Display,
    window: xlib::Window,
    client_type: UnmanagedType,
}

impl UnmanagedClient {
    pub fn new(display: *mut xlib::Display, window: xlib::Window, client_type: UnmanagedType) -> Self {
        let unmanaged = UnmanagedClient { display, window, client_type };
        window.x11_map(display);
        unmanaged.restack();

        unmanaged
    }

    pub fn get_type(&self) -> UnmanagedType {
        self.client_type
    }

    pub fn restack(&self) {
        unsafe {
            use UnmanagedType::*;
            match self.client_type {
                Desktop => { xlib::XLowerWindow(self.display, self.window); },
                Dock | Notification => { xlib::XRaiseWindow(self.display, self.window); },
            }
        }
    }

    pub fn window(&self) -> xlib::Window {
        self.window
    }
}


