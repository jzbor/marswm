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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::Args)]
pub struct ModifierShim {
    #[clap(value_enum)]
    modifier: Option<Modifier>
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, clap::Subcommand)]
pub enum Command {
    /// Activate window
    Activate,

    /// Close window
    Close,

    /// Fullscreen setting of a window
    Fullscreen(ModifierShim),

    /// Show quick menu
    Menu,

    /// Pin window so it is visible on all workspaces
    Pinned(ModifierShim),

    /// Send window to workspace
    SendToWorkspace(Workspace),

    /// Set status string
    SetStatus(Status),

    /// Switch current workspace
    SwitchWorkspace(Workspace),

    /// Switch to next workspace
    SwitchWorkspaceNext,

    /// Switch to previous workspace
    SwitchWorkspacePrev,

    /// Tiled setting of a window
    Tiled(ModifierShim),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Modifier { Set, Unset, Toggle }


impl From<Modifier> for ModifierShim {
    fn from(value: Modifier) -> ModifierShim {
        return ModifierShim { modifier: Some(value) };
    }
}

impl From<Modifier> for SettingMode {
    fn from(value: Modifier) -> SettingMode {
        return match value {
            Modifier::Set => SettingMode::Set,
            Modifier::Unset => SettingMode::Unset,
            Modifier::Toggle => SettingMode::Toggle,
        };
    }
}

fn handle_window_setting<C, G, S>(getter: G, setter: S, controller: &C, window: xlib::Window, opt: ModifierShim)
    -> libmars::common::error::Result<()>
where
    C: WMController<xlib::Window>,
    S: Fn(&C, xlib::Window, SettingMode) -> libmars::common::error::Result<()>,
    G: Fn(&C, xlib::Window) -> libmars::common::error::Result<bool>,
{
    if let Some(modifier) = opt.modifier {
        setter(controller, window, SettingMode::from(modifier))
    } else {
        let state = getter(controller, window)?;
        println!("{}", state);
        if state {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    }
}


impl Command {
    fn execute<C: WMController<xlib::Window>>(&self, controller: &C, window: xlib::Window, args: Args) -> Result<(), String> {
        if *self == Command::Menu {
            return Self::menu(controller, window, args);
        } else {
            let result = match self {
                Command::Activate => controller.activate_window(window),
                Command::Close => controller.close_window(window),
                Command::Fullscreen(mode) => handle_window_setting(C::window_is_fullscreen, C::fullscreen_window, controller,
                                                                   window, *mode),
                Command::Pinned(mode) => handle_window_setting(C::window_is_pinned, C::pin_window, controller, window, *mode),
                Command::SendToWorkspace(ws) => controller.send_window_to_workspace(window, ws.index),
                Command::SetStatus(status) => controller.set_status(status.text.to_owned()),
                Command::SwitchWorkspace(ws) => controller.switch_workspace(ws.index),
                Command::SwitchWorkspaceNext => Self::switch_workspace_relative(controller, 1),
                Command::SwitchWorkspacePrev => Self::switch_workspace_relative(controller, -1),
                Command::Tiled(mode) => handle_window_setting(C::window_is_tiled, C::tile_window, controller, window, *mode),
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
