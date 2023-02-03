extern crate x11;

use clap::Parser;
use libmars::common::error::MarsError;
use libmars::control::*;
use libmars::control::x11::X11Controller;
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

    /// String parameter
    #[clap(short, long)]
    status: Option<String>
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ArgEnum)]
pub enum Command {
    Activate,
    Close,
    Menu,
    Pin,
    SendToDesktop,
    SetFullscreen,
    SetStatus,
    SetTiled,
    SwitchDesktop,
    SwitchDesktopNext,
    SwitchDesktopPrev,
    ToggleFullscreen,
    ToggleTiled,
    Unpin,
    UnsetFullscreen,
    UnsetTiled,
}


impl Command {
    fn execute(&self, controller: &impl WMController<xlib::Window>, window: xlib::Window, args: Args) -> Result<(), String> {
        if *self == Command::Menu {
            return Self::menu(controller, window, args);
        } else if *self == Command::SetStatus {
            if let Some(status) = args.status {
                return controller.set_status(status)
                    .map_err(|e| e.to_string());
            } else {
                return Err("Please supply a status string".to_owned());
            }
        } else if *self == Command::SwitchDesktop {
            if let Some(ws) = args.desktop {
                return controller.switch_workspace(ws as u32)
                    .map_err(|e| e.to_string());
            } else {
                return Err("Please supply a workspace".to_owned());
            }
        } else if *self == Command::SendToDesktop {
            if let Some(ws) = args.desktop {
                return controller.send_window_to_workspace(window, ws as u32)
                    .map_err(|e| e.to_string());
            } else {
                return Err("Please supply a workspace".to_owned());
            }
        } else {
            let result = match self {
                Command::Activate => controller.activate_window(window),
                Command::Close => controller.close_window(window),
                Command::Pin => controller.pin_window(window, SettingMode::Set),
                Command::SetFullscreen => controller.fullscreen_window(window, SettingMode::Set),
                Command::SetTiled => controller.tile_window(window, SettingMode::Set),
                Command::SwitchDesktopNext => Self::switch_workspace_relative(controller, 1),
                Command::SwitchDesktopPrev => Self::switch_workspace_relative(controller, -1),
                Command::ToggleFullscreen => controller.fullscreen_window(window, SettingMode::Toggle),
                Command::ToggleTiled => controller.tile_window(window, SettingMode::Toggle),
                Command::Unpin => controller.pin_window(window, SettingMode::Unset),
                Command::UnsetFullscreen => controller.fullscreen_window(window, SettingMode::Unset),
                Command::UnsetTiled => controller.tile_window(window, SettingMode::Unset),
                Command::Menu | Command::SendToDesktop | Command::SetStatus
                    | Command::SwitchDesktop => panic!("unhandled command"),
            };
            return result.map_err(|e| e.to_string());
        }
    }

    fn menu(controller: &impl WMController<xlib::Window>, window: xlib::Window, args: Args) -> Result<(), String> {
        let command = match display_menu() {
            Ok(cmd) => cmd,
            Err(e) => { eprintln!("Error: {}", e); return Err("unable to display menu".to_owned()); },
        };
        return command.execute(controller, window, args);
    }

    fn switch_workspace_relative(controller: &impl WMController<xlib::Window>, inc: i32) -> Result<(), MarsError> {
        let workspace = controller.current_workspace()?;
        let nworkspaces = controller.count_workspaces()?;
        let new_workspace = (workspace + (nworkspaces as i32 + inc) as u32) % nworkspaces;
        return controller.switch_workspace(new_workspace);
    }
}

fn main() {
    let args = Args::parse();

    let controller = match X11Controller::new() {
        Ok(ctrl) => ctrl,
        Err(e) => {
            eprintln!("Unable to get active window: {}", e);
            std::process::exit(1);
        },
    };

    let command = args.command;
    let window = match args.window {
        Some(window) => window,
        None => match controller.get_active_window() {
            Ok(window) => window,
            Err(e) => {
                eprintln!("Unable to get active window: {}", e);
                std::process::exit(1);
            },
        },
    };
    if let Err(msg) = command.execute(&controller, window, args) {
        eprintln!("Error: {}", msg);
        std::process::exit(1);
    }
}
