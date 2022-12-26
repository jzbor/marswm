use clap::Parser;
use libmars::x11::atoms::*;
use libmars::x11::atoms::X11Atom::*;
use libmars::x11::window::X11Window;
use std::ptr;
use x11::xlib;

use crate::menu::*;


mod menu;


/// Sending commands to marswm and other window managers
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Command to send to window manager
    #[clap(value_enum, value_parser)]
    command: Command,

    /// Window to execute command on (defaults to currently active window)
    #[clap(short, long)]
    window: Option<xlib::Window>,

    /// Desktop index for switching desktop or changing desktop of clients
    #[clap(short, long)]
    desktop: Option<usize>,

}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ArgEnum)]
pub enum Command {
    Activate,
    Close,
    Menu,
    Pin,
    Unpin,
    SendToDesktop,
    SetFullscreen,
    ToggleFullscreen,
    UnsetFullscreen,
    SwitchDesktop,
}


const MODE_SET: u64 = 1;
const MODE_UNSET: u64 = 0;
const MODE_TOGGLE: u64 = 2;


impl Command {
    fn execute(&self, display: *mut xlib::Display, window: xlib::Window, desktop: Option<usize>) -> Result<(), &'static str> {
        match self {
            Command::Activate => Self::simple_window_message(display, window, NetActiveWindow),
            Command::Close => Self::simple_window_message(display, window, NetCloseWindow),
            Command::Menu => Self::menu(display, window),
            Command::Pin => Self::pin_window(display, window),
            Command::Unpin => Self::unpin_window(display, window),
            Command::SendToDesktop => Self::send_window_to_desktop(display, window, desktop),
            Command::SetFullscreen => Self::fullscreen_window(display, window, MODE_SET),
            Command::ToggleFullscreen => Self::fullscreen_window(display, window, MODE_TOGGLE),
            Command::UnsetFullscreen => Self::fullscreen_window(display, window, MODE_UNSET),
            Command::SwitchDesktop => Self::switch_desktop(display, desktop),
        }
    }

    fn fullscreen_window(display: *mut xlib::Display, window: xlib::Window, mode: u64) -> Result<(), &'static str> {
        require_ewmh_atom(display, NetWMStateFullscreen)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, mode as i64);
        data.set_long(1, NetWMStateFullscreen.to_xlib_atom(display) as i64);
        send_client_message(display, NetWMState, window, data);
        return Ok(());
    }

    fn menu(display: *mut xlib::Display, window: xlib::Window) -> Result<(), &'static str> {
        let command = match display_menu() {
            Ok(cmd) => cmd,
            Err(e) => { println!("Error: {}", e); return Err("unable to display menu"); },
        };
        return command.execute(display, window, None);
    }

    fn pin_window(display: *mut xlib::Display, window: xlib::Window) -> Result<(), &'static str> {
        require_ewmh_atom(display, NetWMDesktop)?;
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, -1);
        send_client_message(display, NetWMDesktop, window, data);
        return Ok(());
    }

    fn send_window_to_desktop(display: *mut xlib::Display, window: xlib::Window, desktop: Option<usize>)
            -> Result<(), &'static str> {
        require_ewmh_atom(display, NetWMDesktop)?;
        let desktop = match desktop {
            Some(desktop) => match desktop.try_into() {
                Ok(desktop) => desktop,
                Err(_) => return Err("Invalid desktop index"),
            },
            None => return Err("Please supply a desktop index"),
        };
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, desktop);
        send_client_message(display, NetWMDesktop, window, data);
        return Ok(());
    }

    fn simple_window_message(display: *mut xlib::Display, window: xlib::Window, atom: X11Atom)
            -> Result<(), &'static str> {
        require_ewmh_atom(display, atom)?;
        let data = xlib::ClientMessageData::new();
        send_client_message(display, atom, window, data);
        return Ok(());
    }

    fn switch_desktop(display: *mut xlib::Display, desktop: Option<usize>)
            -> Result<(), &'static str> {
        require_ewmh_atom(display, NetCurrentDesktop)?;
        let desktop = match desktop {
            Some(desktop) => match desktop.try_into() {
                Ok(desktop) => desktop,
                Err(_) => return Err("Invalid desktop index"),
            },
            None => return Err("Please supply a desktop index"),
        };
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, desktop);
        send_client_message(display, NetCurrentDesktop, 0, data);
        return Ok(());
    }

    fn unpin_window(display: *mut xlib::Display, window: xlib::Window) -> Result<(), &'static str> {
        require_ewmh_atom(display, NetWMDesktop)?;
        require_ewmh_atom(display, NetCurrentDesktop)?;
        let root = unsafe { xlib::XDefaultRootWindow(display) };
        let data = root.x11_read_property_long(display, NetCurrentDesktop.to_xlib_atom(display), xlib::XA_CARDINAL)?;
        return Self::send_window_to_desktop(display, window, Some(data[0].try_into().unwrap()));
    }
}

fn active_window(display: *mut xlib::Display) -> Result<xlib::Window, &'static str> {
    require_ewmh_atom(display, NetActiveWindow)?;

    let root = unsafe {
        xlib::XDefaultRootWindow(display)
    };

    let data = root.x11_read_property_long(display, NetActiveWindow.to_xlib_atom(display), xlib::XA_WINDOW)?;
    if data.len() != 1 {
        return Err("Query for _NET_ACTIVE_WINDOW returned invalid data");
    } else {
        return Ok(data[0]);
    }
}

fn require_ewmh_atom(display: *mut xlib::Display, atom: X11Atom) -> Result<(), &'static str> {
    let root = unsafe {
        xlib::XDefaultRootWindow(display)
    };
    let xatom = atom.to_xlib_atom(display);
    let supported = root.x11_read_property_long(display, NetSupported.to_xlib_atom(display), xlib::XA_ATOM)?;

    if supported.contains(&xatom) {
        return Ok(());
    } else {
        println!("Required atom: {} / {}", atom, xatom);
        return Err("Required atom not supported");
    }
}

fn send_client_message(display: *mut xlib::Display, atom: X11Atom, window: xlib::Window, data: xlib::ClientMessageData) {
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

fn main() {
    let args = Args::parse();

    let display = unsafe {
        let display = xlib::XOpenDisplay(ptr::null());
        if display.is_null() {
            panic!("XOpenDisplay failed");
        }
        display
    };

    let command = args.command;
    let window = match args.window {
        Some(window) => window,
        None => match active_window(display) {
            Ok(window) => window,
            Err(msg) => {
                println!("Unable to get active window: {}", msg);
                std::process::exit(1);
            },
        },
    };
    let desktop = args.desktop;
    if let Err(msg) = command.execute(display, window, desktop) {
        println!("Error: {}", msg);
        std::process::exit(1);
    }
}
