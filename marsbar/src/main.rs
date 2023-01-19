extern crate x11;

use libmars::*;
use libmars::MonitorConfig;
use libmars::configuration::print_config;
use libmars::x11::atoms::X11Atom::{self, *};
use libmars::x11::draw::{self, canvas::Canvas, widget::*};
use libmars::x11::window::X11Window;
use std::env;
use std::ffi::*;
use std::iter;
use std::mem::MaybeUninit;
use x11::xlib;
use x11::xrandr;

use crate::config::*;
use crate::handlers::*;
use crate::tray::*;


mod config;
mod handlers;
mod status;
mod tray;


const HEIGHT: u32 = 31;
const CLASSNAME: &'static str = "bar";
const WINDOWNAME: &'static str = "Bar Window";
// const FONT: &'static str = "Noto Serif:size=12";
const FONT: &'static str = "serif";
// const TEXT_BASE_HPAD: u32 = 5;
// const TEXT_BASE_VPAD: u32 = 0;
// const TRAY_BASE_HPAD: u32 = 2;
// const TRAY_BASE_VPAD: u32 = 2;
// const ADDITIONAL_PAD: u32 = 4;
// const TITLE_HPAD: u32 = 20;


struct Bar {
    display: *mut xlib::Display,
    root: xlib::Window,
    window: xlib::Window,
    canvas: Canvas,
    dimensions: Dimensions,
    config: Configuration,
    workspace_widget: FlowLayoutWidget<TextWidget>,
    status_widget: FlowLayoutWidget<TextWidget>,
    systray: Option<SystemTrayWidget>,
    title_widget: TextWidget,
    active_window: Option<xlib::Window>,
    default_client_event_mask: i64,
}


impl Bar {
    fn create(display: *mut xlib::Display, dimensions: Dimensions, config: Configuration,
              default_client_event_mask: i64, create_tray: bool) -> Result<Bar, String> {
        let root = unsafe { xlib::XDefaultRootWindow(display) };
        let window_type = Some(NetWMWindowTypeDock);
        let window = draw::create_window(display, dimensions.x(), dimensions.y(), dimensions.w(), dimensions.h(), CLASSNAME, WINDOWNAME, window_type)?;
        let mut dimensions = dimensions;
        dimensions.set_h(HEIGHT);

        // request updates on property changes
        unsafe {
            xlib::XSelectInput(display, root, xlib::PropertyChangeMask);
        }

        // TODO destroy window on failure
        let mut canvas = Canvas::new_for_window(display, window)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;
        canvas.set_foreground(config.style.background)
            .and(canvas.set_background(config.style.background))
            .and(canvas.set_font(FONT))
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;

        let workspace_widget = config.style.workspaces.create_flow_layout_widget(display, window)?;
        let title_widget = config.style.title.create_text_widget(display, window)?;
        let status_widget = config.style.status.create_flow_layout_widget(display, window)?;
        let systray = if create_tray {
            config.style.status.create_systray_widget(display, window, dimensions.h()).ok()
        } else {
            None
        };


        let mut bar = Bar {
            display, root, window,
            canvas, dimensions,
            config,
            workspace_widget, status_widget, systray, title_widget,
            active_window: None,
            default_client_event_mask,
        };

        bar.arrange();

        return Ok(bar);
    }

    fn arrange(&mut self) {
        // arrange desktop segment
        let result = self.root.x11_get_text_list_property(self.display, NetDesktopNames);
        let desktop_names = match result {
            Ok(names) => names,
            Err(e) => {
                eprintln!("WARNING: Desktop names not available ({})", e);
                Vec::new()
            },
        };
        self.arrange_workspaces(desktop_names);
        if let Ok(idx) = self.get_active_workspace() {
            self.set_active_workspace(idx);
        }

        // arrange title segment
        self.active_window = self.get_active_window();
        self.arrange_title(self.get_active_window_title());

        // arrange tray segment
        self.arrange_tray();

        // arrange status segment
        if let Ok(wmname) = self.get_status() {
            self.arrange_status(wmname);
        }

        self.update_constraints();
    }

    fn arrange_status(&mut self, status: String) {
        let mut nblocks = 0;
        let mut divider = String::new();
        divider.push(char::from(0x1f));

        // update status blocks
        for (i, block) in status.split(&divider).enumerate() {
            if let Some(widget) = self.status_widget.child_mut(i) {
                widget.set_label(block.to_owned());
            } else {
                let mut widget = self.config.style.status
                    .create_text_widget(self.display, self.status_widget.wid())
                    .unwrap();
                if let Some(callback) = &self.config.callback {
                    let event_handler = StatusEventHandler::new(i, callback.clone());
                    widget.register_event_handler(Box::new(event_handler));
                }
                self.status_widget.push(widget);
            }
            nblocks += 1;
        }

        // remove unused blocks
        let nblock_widgets = self.status_widget.count_children();
        if nblocks < nblock_widgets {
            self.status_widget.truncate_children(nblocks);
        }

        self.status_widget.rearrange();

        // self.status_widget.set_label(status);
        let tray_width = if let Some(tray_widget) = &self.systray {
            tray_widget.size().0 + self.config.style.status.padding_horz
        } else {
            0
        };
        let height_diff = (self.dimensions.h() as i32 - self.status_widget.size().1 as i32) / 2;
        let x = self.dimensions.w() as i32 - tray_width as i32 - self.status_widget.size().0 as i32 - height_diff;
        let y = height_diff;
        self.status_widget.move_to(x, y);
    }

    fn arrange_title(&mut self, title: String) {
        self.title_widget.set_label(title);
        let height_diff = (self.dimensions.h() as i32 - self.title_widget.size().1 as i32) / 2;
        let x = (self.dimensions.w() as i32 - self.title_widget.size().0 as i32) / 2;
        let y = height_diff;
        self.title_widget.move_to(x, y);
    }

    fn arrange_tray(&mut self) {
        if let Some(tray_widget) = &mut self.systray {
            let height_diff = (self.dimensions.h() as i32 - tray_widget.size().1 as i32) / 2;
            let x = self.dimensions.w() as i32 - tray_widget.size().0 as i32 - height_diff;
            let y = height_diff;
            tray_widget.move_to(x, y);
        }
    }

    fn arrange_workspaces(&mut self, workspace_names: Vec<String>) {
        // update workspaces
        for (i, ws_name) in workspace_names.iter().enumerate() {
            if let Some(widget) = self.workspace_widget.child_mut(i) {
                widget.set_label(ws_name.to_owned());
            } else {
                let mut widget = self.config.style.workspaces
                    .create_text_widget(self.display, self.workspace_widget.wid())
                    .unwrap();
                let event_handler = WorkspaceEventHandler::new(self.display, i as u32);
                widget.register_event_handler(Box::new(event_handler));
                self.workspace_widget.push(widget);
            }
        }

        // remove extra widgets
        let nworkspaces = workspace_names.len();
        let nworkspace_widgets = self.workspace_widget.count_children();
        if nworkspaces < nworkspace_widgets {
            self.workspace_widget.truncate_children(nworkspaces);
        }

        self.workspace_widget.rearrange();
    }

    fn create_for_monitor(display: *mut xlib::Display, monitor_conf: &MonitorConfig, config: Configuration,
                          has_tray: bool) -> Result<Bar, String> {
        let mdims = monitor_conf.dimensions();
        let mut dimensions = mdims;
        dimensions.set_h(HEIGHT);
        return Self::create(display, dimensions, config.clone(), xlib::NoEventMask, has_tray);
    }

    fn draw(&mut self) {
        let (x, y) = self.dimensions.pos();
        let (w, h) = self.dimensions.size();
        self.canvas.fill_rectangle(x, y, w, h);

        // dirty workaround for the tray widget appearing as part of the status blocks
        if let Some(systray) = &self.systray {
            let width = systray.size().0 + self.config.style.status.padding_horz;
            let height = self.dimensions.h();
            let x = (self.dimensions.x() + self.dimensions.w() as i32) - width as i32;
            let y = self.dimensions.y();
            self.canvas.fill_rectangle_with(x, y, width, height, self.config.style.status.outer_background);
        }

        self.canvas.flush();
    }

    fn get_status(&self) -> Result<String, &'static str> {
        return self.root.x11_read_property_string(self.display, MarsStatus)
            .or_else(|_| self.root.x11_wm_name(self.display));
    }

    fn get_active_workspace(&self) -> Result<u32, &'static str> {
        let data = self.root.x11_read_property_long(self.display, NetCurrentDesktop, xlib::XA_CARDINAL)?;
        match data.get(0) {
            Some(idx) => return Ok(*idx as u32),
            None => return Err("unable to convert desktop index to u32"),
        };
    }

    fn get_active_window(&self) -> Option<xlib::Window> {
        let result = self.root.x11_read_property_long(self.display, NetActiveWindow, xlib::XA_WINDOW);
        let data = match result {
            Ok(data) => data,
            Err(_) => return None,
        };
        match data.get(0) {
            Some(0) | None => return None,
            Some(window) => return Some(*window),
        };
    }

    fn get_active_window_title(&self) -> String {
        let default = "".to_owned();
        let window = match self.get_active_window() {
            Some(window) => window,
            None => return default,
        };
        match window.x11_wm_name(self.display) {
            Ok(title) => return title,
            Err(_) => return default,
        }
    }

    fn handle_xevent(&mut self, event: xlib::XEvent) {
        unsafe {
            if event.any.window == self.root {
                self.handle_root_event(event);
            } else if event.any.window == self.window {
                self.handle_bar_event(event);
            } else if Some(event.any.window) == self.active_window
                    && event.get_type() == xlib::PropertyNotify
                    && event.property.atom == WMName.to_xlib_atom(self.display) {
                let title = self.get_active_window_title();
                self.arrange_title(title);
            } else if event.get_type() == xlib::ClientMessage
                    && Some(event.any.window) == self.systray.as_ref().map(|w| w.wid())
                    && event.client_message.message_type == NetSystemTrayOpcode.to_xlib_atom(self.display) {
                if let Some(systray) = &mut self.systray {
                    systray.handle_systray_event(event.client_message);
                    self.arrange();
                }
            } else if event.get_type() == xlib::DestroyNotify
                    && Some(event.destroy_window.event) == self.systray.as_ref().map(|w| w.wid()) {
                if let Some(systray) = &mut self.systray {
                    systray.handle_icon_destroyed(event.destroy_window);
                    self.arrange();
                }
            } else {
                let systray_iter = if let Some(systray) = &mut self.systray {
                    Box::new(iter::once(systray as &mut dyn Widget)) as Box<dyn Iterator<Item = &mut dyn Widget>>
                } else {
                    Box::new(iter::empty())
                };
                // let mut widgets = self.workspace_widget.children_mut().map(|w| w as &mut dyn Widget)
                let mut widgets = iter::empty()
                    .chain(iter::once(&mut self.workspace_widget as &mut dyn Widget))
                    .chain(iter::once(&mut self.status_widget as &mut dyn Widget))
                    // .chain(self.status_widget.children_mut().map(|w| w as &mut dyn Widget))
                    .chain(iter::once(&mut self.title_widget as &mut dyn Widget))
                    .chain(systray_iter);
                distribute_widget_event(&mut widgets, event);
            }
        }
    }

    fn handle_bar_event(&mut self, event: xlib::XEvent) {
        match event.get_type() {
            xlib::Expose => self.draw(),
            _ => (),
        }
    }

    fn handle_root_event(&mut self, event: xlib::XEvent) {
        if event.get_type() == xlib::PropertyNotify {
            let event = unsafe { event.property };
            if let Some(property) = X11Atom::from_xlib_atom(self.display, event.atom) {
                match property {
                    NetCurrentDesktop => {
                        let new_idx = match self.get_active_workspace() {
                            Ok(idx) => idx,
                            Err(_) => return,
                        };
                        self.set_active_workspace(new_idx);
                    },
                    NetActiveWindow => {
                        // watch updates for WM_NAME on active window
                        unsafe {
                            if let Some(window) = self.active_window {
                                // TODO this does NOT work when called from a window manager
                                // directly
                                xlib::XSelectInput(self.display, window, self.default_client_event_mask);
                            }
                            self.active_window = self.get_active_window();
                            if let Some(window) = self.active_window {
                                xlib::XSelectInput(self.display, window, self.default_client_event_mask | xlib::PropertyChangeMask);
                            }
                        }

                        let title = self.get_active_window_title();
                        self.arrange_title(title);
                    }
                    MarsStatus | WMName => {
                        if let Ok(status) = self.get_status() {
                            self.arrange_status(status)
                        }
                    }
                    _ => (),
                }
            };
        }
    }

    fn reconfigure(&mut self, monitor_config: MonitorConfig) {
        self.dimensions.set_pos(monitor_config.dimensions().pos());
        self.dimensions.set_w(monitor_config.dimensions().w());
        unsafe {
            xlib::XMoveResizeWindow(self.display, self.window, self.dimensions.x(), self.dimensions.y(), self.dimensions.w(), self.dimensions.h());
        }
        self.arrange();
    }

    fn update_constraints(&mut self) {
        let height = self.dimensions.h();
        let mut width = self.dimensions.w() - self.workspace_widget.size().0;

        // constrain systray
        if let Some(systray) = &mut self.systray {
            systray.set_max_size((width, height));
            width -= systray.size().0 + self.config.style.status.padding_horz;
        }

        // constrain status
        self.status_widget.set_max_size((width, height));
        width -= self.status_widget.size().0;

        // constrain title
        self.title_widget.set_max_size((width, height));
    }

    fn set_active_workspace(&mut self, new_idx: u32) {
        for (i, widget) in &mut self.workspace_widget.children_mut().enumerate() {
            if i as u32 == new_idx {
                widget.set_foreground(self.config.style.workspaces.inner_background).unwrap();
                widget.set_background(self.config.style.workspaces.foreground).unwrap();
            } else {
                widget.set_foreground(self.config.style.workspaces.foreground).unwrap();
                widget.set_background(self.config.style.workspaces.inner_background).unwrap();
            }
        }
    }

    fn await_map_notify(&mut self) {
        draw::await_map_notify(self.display, self.window);
        self.draw();
        println!("Window mapped: 0x{:x}, {:?}", self.window, self.window.x11_dimensions(self.display));
    }

}

fn eventloop(display: *mut xlib::Display, mut bar: Bar, have_xrandr: bool, xrr_event_base: i32) {
    loop {
        let mut event: MaybeUninit<xlib::XEvent> = MaybeUninit::uninit();
        unsafe {
            xlib::XNextEvent(bar.display, event.as_mut_ptr());
            let event = event.assume_init();
            if have_xrandr && event.get_type() == xrr_event_base + xrandr::RRNotify {
                let monitors = libmars::x11::query_monitor_config(display);
                bar.reconfigure(*monitors.get(0).unwrap());
            } else {
                bar.handle_xevent(event);
            }
        }
    }
}

fn main() {
    if env::args().find(|a| a == "print-default-config").is_some() {
        print_config(&Configuration::default());
        return;
    }

    let config = read_config();

    unsafe {
        xlib::XSetErrorHandler(Some(on_error));
    }

    // test();
    let display = draw::open_display().unwrap();

    // unsafe {
    //     #[cfg(debug_assertions)]
    //     xlib::XSynchronize(display, 1);
    // }

    let (have_xrandr, xrr_event_base, _xrr_error_base) = unsafe {
        let mut xrr_event_base = 0;
        let mut xrr_error_base = 0;
        let have_xrandr = xrandr::XRRQueryExtension(display, &mut xrr_event_base, &mut xrr_error_base) != 0;

        if have_xrandr {
            xrandr::XRRSelectInput(display, xlib::XDefaultRootWindow(display), xrandr::RRCrtcChangeNotifyMask);
        }

        (have_xrandr, xrr_event_base, xrr_error_base)
    };

    let monitors = libmars::x11::query_monitor_config(display);
    let mut bar = Bar::create_for_monitor(display, monitors.get(0).unwrap(), config, true).unwrap();
    bar.await_map_notify();

    eventloop(display, bar, have_xrandr, xrr_event_base);

    draw::close_display(display);
}

extern "C" fn on_error(display: *mut xlib::Display, error: *mut xlib::XErrorEvent) -> c_int {
    let msg = unsafe {
        let bufsize = 1024;
        let mut buf = vec![0u8; bufsize];
        xlib::XGetErrorText(display, (*error).error_code.into(), buf.as_mut_ptr() as *mut i8,
                            (bufsize - 1) as c_int);
        let msg_cstring = CStr::from_ptr(buf.as_mut_ptr() as *mut i8);
        msg_cstring.to_str().unwrap().to_owned()
        // println!("{}", msg);
    };

    unsafe {
        match (*error).error_code {
            xlib::BadWindow => println!("X11 error: {} (request code: {})", msg, (*error).request_code),
            xlib::BadMatch => println!("X11 error: {} (request code: {})", msg, (*error).request_code),
            _ => panic!("Fatal X11 error: {} (request code: {})", msg, (*error).request_code),
        }
    }
    return 0;
}
