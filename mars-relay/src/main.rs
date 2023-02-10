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
    // #[clap(value_enum, value_parser)]
    #[clap(subcommand)]
    command: Command,

    /// Window to execute command on (defaults to currently active window)
    #[clap(short, long)]
    window: Option<xlib::Window>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::Args)]
pub struct Workspace { index: u32 }

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, clap::Args)]
pub struct Status { text: String }

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, clap::Subcommand)]
pub enum Command {
    /// Activate window
    Activate,

    /// Close window
    Close,

    /// Show quick menu
    Menu,

    /// Pin window so it is visible on all workspaces
    Pin,

    /// Send window to workspace
    SendToWorkspace(Workspace),

    /// Make window fullscreen
    SetFullscreen,

    /// Set status string
    SetStatus(Status),

    /// Tile window
    SetTiled,

    /// Switch current workspace
    SwitchWorkspace(Workspace),

    /// Switch to next workspace
    SwitchWorkspaceNext,

    /// Switch to previous workspace
    SwitchWorkspacePrev,

    /// Toggle fullscreen on window
    ToggleFullscreen,

    /// Toggle tiled state on window
    ToggleTiled,

    /// Unpin window
    Unpin,

    /// Unfullscreen window
    UnsetFullscreen,

    /// Untile window
    UnsetTiled,
}


impl Command {
    fn execute(&self, controller: &impl WMController<xlib::Window>, window: xlib::Window, args: Args) -> Result<(), String> {
        if *self == Command::Menu {
            return Self::menu(controller, window, args);
        } else {
            let result = match self {
                Command::Activate => controller.activate_window(window),
                Command::Close => controller.close_window(window),
                Command::Pin => controller.pin_window(window, SettingMode::Set),
                Command::SendToWorkspace(ws) => controller.send_window_to_workspace(window, ws.index),
                Command::SetFullscreen => controller.fullscreen_window(window, SettingMode::Set),
                Command::SetStatus(status) => controller.set_status(status.text.to_owned()),
                Command::SetTiled => controller.tile_window(window, SettingMode::Set),
                Command::SwitchWorkspace(ws) => controller.switch_workspace(ws.index),
                Command::SwitchWorkspaceNext => Self::switch_workspace_relative(controller, 1),
                Command::SwitchWorkspacePrev => Self::switch_workspace_relative(controller, -1),
                Command::ToggleFullscreen => controller.fullscreen_window(window, SettingMode::Toggle),
                Command::ToggleTiled => controller.tile_window(window, SettingMode::Toggle),
                Command::Unpin => controller.pin_window(window, SettingMode::Unset),
                Command::UnsetFullscreen => controller.fullscreen_window(window, SettingMode::Unset),
                Command::UnsetTiled => controller.tile_window(window, SettingMode::Unset),
                Command::Menu => panic!("unhandled command"),
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

    let command = args.command.clone();
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
