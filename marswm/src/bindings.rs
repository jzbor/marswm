extern crate x11;

use libmars::platforms::x11::misc::get_keysym;
use serde::{Serialize, Deserialize};
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use x11::xlib::{Mod1Mask, Mod4Mask, ShiftMask, ControlMask};

use crate::*;
use crate::layouts::*;


pub const DEFAULT_MODKEY: Modifier = Modifier::Mod4;


macro_rules! client_button_binding {
    ($button:expr, $action:expr $(, ($($add_mods:ident ),*))?) => {
        ButtonBinding::new(vec![DEFAULT_MODKEY $(, $($add_mods),*)?], $button, vec![Window, Frame], $action)
    }
}

macro_rules! frame_button_binding {
    ($button:expr, $action:expr $(, ($($add_mods:ident ),*))?) => {
        ButtonBinding::new(vec![$($($add_mods),*)?], $button, vec![Frame], $action)
    }
}


/// Actions for key bindings, button bindings and window rules.
///
/// ***Note that the configuration files use `kebab-case` convention for enum variants.***
#[derive(Serialize,Deserialize,Clone,Debug,PartialEq)]
#[serde(rename_all = "kebab-case")]
// #[serde(tag = "action", content = "arg")]
// #[serde(tag = "type")]
pub enum BindingAction {
    /// Center the current client on the screen
    CenterClient,
    /// Change the ration between main and stack area
    ChangeMainRatio(f32),
    /// Close the client
    CloseClient,
    /// Cycle through clients
    CycleClient(i32),
    /// Switch through the different layouts
    CycleLayout,
    /// Switch monitor (relative to the current monitor)
    CycleMonitor(i32),
    /// Cycle through workspaces
    CycleWorkspace(i32),
    /// Execute a command in the system shell
    Execute(String),
    /// Exit the window manager
    Exit,
    /// Focus next client in the specified direction
    FocusDirection(Direction),
    /// Switch between the last focused window of the main and stack area
    FocusMain,
    /// Increase or decrease the gap width of the current workspace
    IncGaps(i32),
    /// Increase or decrease the number of clients in the main area
    IncNMain(i32),
    /// Move the client with the mouse
    MouseMove,
    /// Place a window with the mouse
    MousePlace,
    /// Resize a window with the mouse
    MouseResize,
    /// Resize a window around its center
    MouseResizeCentered,
    /// Move client to/from the main area
    MoveMain,
    /// Move the client to a different monitor (relative to the current monitor)
    MoveMonitor(i32),
    /// Move the client to a different workspace
    MoveWorkspace(u32),
    /// Switch to the previously focused workspace
    PreviousWorkspace,
    /// Restart the window manager
    Restart,
    /// Set the layout
    SetLayout(LayoutType),
    /// Set the stack mode for the dynamic layout
    SetStackMode(StackMode),
    /// Set the stack position for the dynamic layout
    SetStackPosition(StackPosition),
    /// Move the client up or down the stack
    StackMove(i32),
    /// Switch to a different workspace
    SwitchWorkspace(u32),
    /// Toggle floating state on the window
    ToggleFloating,
    /// Toggle fullscreen state on the window
    ToggleFullscreen,
}

#[derive(Serialize,Deserialize,Clone,Debug,PartialEq,Eq)]
pub enum Modifier {
    Mod1,
    Mod4,
    Shift,
    Control,
}

#[derive(Serialize,Deserialize,PartialEq,Eq,Debug,Copy,Clone)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
pub struct KeyBinding {
    /// list of modifiers that apply to this binding
    #[serde(default)]
    modifiers: Vec<Modifier>,

    /// key name (as found in
    /// [keysymdef.h](https://cgit.freedesktop.org/xorg/proto/x11proto/tree/keysymdef.h) without
    /// the leading "XK_")
    key: String,

    /// action to execute on key press
    action: BindingAction,
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
pub struct ButtonBinding {
    /// list of modifiers that apply to this binding
    modifiers: Vec<Modifier>,

    /// button index (1, 2, 3 for left, middle, right)
    button: u32,

    /// button target
    targets: Vec<ButtonTarget>,

    /// action to execute on key press
    action: BindingAction,
}

impl BindingAction {
    pub fn execute<B: Backend<Attributes>>(&self, wm: &mut MarsWM<B>, backend: &mut B,
                                         client_option: Option<Rc<RefCell<B::Client>>>) {
        use BindingAction::*;
        match self {
            CenterClient => if let Some(client_rc) = client_option {
                wm.center_client(backend, client_rc);
            },
            ChangeMainRatio(f) =>  wm.current_workspace_mut(backend).change_main_ratio(*f),
            CloseClient => if let Some(client_rc) = client_option {
                client_rc.borrow().close();
            },
            CycleClient(inc) => wm.cycle_client(backend, *inc),
            CycleLayout => wm.current_workspace_mut(backend).cycle_layout(),
            CycleMonitor(inc) => wm.cycle_monitor(backend, *inc),
            CycleWorkspace(inc) => wm.cycle_workspace(backend, *inc),
            Execute(cmd) => {
                if let Ok(mut handle) = std::process::Command::new("sh").arg("-c").arg(cmd).spawn() {
                    std::thread::spawn(move || {
                        let _ignored = handle.wait();
                    });
                }
            },
            Exit => {
                wm.exit(backend);
            },
            FocusDirection(d) => wm.focus_direction(backend, *d),
            FocusMain => wm.switch_to_main(backend),
            IncGaps(i) => wm.current_workspace_mut(backend).inc_gaps(*i),
            IncNMain(i) => wm.current_workspace_mut(backend).inc_nmain(*i),
            MouseMove => if let Some(client_rc) = client_option {
                backend.mouse_move(wm, client_rc);
                wm.current_monitor_mut(backend).restack_current();
            },
            MousePlace => if let Some(client_rc) = client_option {
                wm.mouse_place(backend, client_rc);
            },
            MouseResize => if let Some(client_rc) = client_option {
                if is_floating!(wm, &client_rc) {
                    backend.mouse_resize(wm, client_rc);
                }
            },
            MouseResizeCentered => if let Some(client_rc) = client_option {
                if is_floating!(wm, &client_rc) {
                    wm.mouse_resize_centered(backend, client_rc);
                }
            },
            MoveMonitor(inc) => if let Some(client_rc) = client_option {
                wm.move_client_to_monitor(client_rc, *inc);
            },
            MoveWorkspace(ws) => if let Some(client_rc) = client_option {
                let ws_index_option = wm.get_monitor_mut(&client_rc)
                    .and_then(|m| m.workspace(*ws))
                    .map(|ws| ws.global_index());
                if let Some(ws_index) = ws_index_option {
                    wm.move_to_workspace(backend, client_rc, ws_index);
                }
            },
            PreviousWorkspace => wm.switch_prev_workspace(backend),
            MoveMain => if let Some(client_rc) = client_option {
                wm.current_workspace_mut(backend).move_main(client_rc);
            },
            Restart => wm.restart(backend),
            SetLayout(layout) => wm.current_workspace_mut(backend).set_layout(*layout),
            SetStackMode(mode) => wm.current_workspace_mut(backend).set_stack_mode(*mode),
            SetStackPosition(position) => wm.current_workspace_mut(backend).set_stack_position(*position),
            StackMove(i) => if let Some(client_rc) = client_option {
                wm.current_workspace_mut(backend).stack_move(client_rc, *i);
            },
            SwitchWorkspace(ws) => {
                let ws_index_option = wm.current_monitor(backend).workspace(*ws)
                    .map(|ws| ws.global_index());
                if let Some(ws_index) = ws_index_option {
                    wm.switch_workspace(backend, ws_index);
                }
            },
            ToggleFloating => if let Some(client_rc) = client_option {
                wm.toggle_tile_client(backend, client_rc);
            },
            ToggleFullscreen => if let Some(client_rc) = client_option {
                wm.toggle_fullscreen_client(backend, client_rc);
            },
        }
    }
}

impl KeyBinding {
    pub fn new(modifiers: Vec<Modifier>, key: &str, action: BindingAction) -> Self {
        KeyBinding { modifiers, key: key.to_owned(), action }
    }

    pub fn action(&self) -> BindingAction {
        self.action.clone()
    }

    pub fn matches(&self, modifiers: u32, key: u32) -> bool {
        modifiers == self.modifiers() && key == self.key()
    }

    pub fn modifiers(&self) -> u32 {
        return self.modifiers.iter().fold(0, |a, b| a | b.mask());
    }

    pub fn key(&self) -> u32 {
        get_keysym(&self.key) as u32
    }
}

impl ButtonBinding {
    pub fn new(modifiers: Vec<Modifier>, button: u32, targets: Vec<ButtonTarget>, action: BindingAction) -> Self {
        ButtonBinding { modifiers, button, targets, action }
    }

    pub fn action(&self) -> BindingAction {
        self.action.clone()
    }

    pub fn button(&self) -> u32 {
        self.button
    }

    pub fn matches(&self, modifiers: u32, button: u32, target: ButtonTarget) -> bool {
        return modifiers == self.modifiers() && button == self.button && self.targets.iter().any(|t| *t == target);
    }

    pub fn modifiers(&self) -> u32 {
        return self.modifiers.iter().fold(0, |a, b| a | b.mask());
    }

    pub fn targets(&self) -> &[ButtonTarget] {
        &self.targets
    }
}

impl Modifier {
    pub fn mask(&self) -> u32 {
        match self {
            Modifier::Mod1 => Mod1Mask,
            Modifier::Mod4 => Mod4Mask,
            Modifier::Shift => ShiftMask,
            Modifier::Control => ControlMask,
        }
    }
}

pub fn default_key_bindings(nworkspaces: u32) -> Vec<KeyBinding> {
    use BindingAction::*;
    use Modifier::*;
    use Direction::*;
    let mut bindings = vec![
        KeyBinding::new(vec![DEFAULT_MODKEY], "Delete", CloseClient),
        KeyBinding::new(vec![DEFAULT_MODKEY], "n", CycleLayout),
        KeyBinding::new(vec![DEFAULT_MODKEY, Shift], "t", SetLayout(LayoutType::Stack)),
        KeyBinding::new(vec![DEFAULT_MODKEY, Control], "t", SetLayout(LayoutType::BottomStack)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "c", SetLayout(LayoutType::Deck)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "m", SetLayout(LayoutType::Monocle)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "t", SetLayout(LayoutType::Dynamic)),
        KeyBinding::new(vec![DEFAULT_MODKEY, Shift], "f", SetLayout(LayoutType::Floating)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "BackSpace", MoveMain),
        KeyBinding::new(vec![DEFAULT_MODKEY], "a", IncNMain(1)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "x", IncNMain(-1)),
        KeyBinding::new(vec![DEFAULT_MODKEY, Control], "a", ChangeMainRatio(0.10)),
        KeyBinding::new(vec![DEFAULT_MODKEY, Control], "x", ChangeMainRatio(-0.10)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "j", FocusDirection(Down)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "k", FocusDirection(Up)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "h", FocusDirection(Left)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "l", FocusDirection(Right)),
        KeyBinding::new(vec![DEFAULT_MODKEY, Shift], "j", StackMove(1)),
        KeyBinding::new(vec![DEFAULT_MODKEY, Shift], "k", StackMove(-1)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "period", CycleWorkspace(1)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "comma", CycleWorkspace(-1)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "f", ToggleFullscreen),
        KeyBinding::new(vec![DEFAULT_MODKEY, Shift], "space", ToggleFloating),
        KeyBinding::new(vec![DEFAULT_MODKEY], "z", CenterClient),
        KeyBinding::new(vec![DEFAULT_MODKEY], "Tab", PreviousWorkspace),
        KeyBinding::new(vec![DEFAULT_MODKEY], "Return", Execute("$TERMINAL".to_owned())),
        KeyBinding::new(vec![DEFAULT_MODKEY], "d", Execute("rofi -show drun".to_owned())),
        KeyBinding::new(vec![DEFAULT_MODKEY, Control], "BackSpace", Restart),
        KeyBinding::new(vec![DEFAULT_MODKEY], "Up", SetStackPosition(StackPosition::Top)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "Right", SetStackPosition(StackPosition::Right)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "Down", SetStackPosition(StackPosition::Bottom)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "Left", SetStackPosition(StackPosition::Left)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "semicolon", SetStackMode(StackMode::Split)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "apostrophe", SetStackMode(StackMode::Deck)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "Next", CycleMonitor(1)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "Prior", CycleMonitor(-1)),
        KeyBinding::new(vec![DEFAULT_MODKEY, Shift], "Next", MoveMonitor(1)),
        KeyBinding::new(vec![DEFAULT_MODKEY, Shift], "Prior", MoveMonitor(-1)),
    ];

    for i in 0..cmp::min(nworkspaces, 9) {
        let key_name = format!("{}", i + 1);
        bindings.push(KeyBinding::new(vec!(DEFAULT_MODKEY), &key_name, SwitchWorkspace(i)));
        bindings.push(KeyBinding::new(vec!(DEFAULT_MODKEY, Modifier::Shift), &key_name, MoveWorkspace(i)));
    }

    bindings
}

pub fn default_button_bindings() -> Vec<ButtonBinding> {
    use BindingAction::*;
    use ButtonTarget::*;
    use Modifier::*;
    let bindings = vec![
        frame_button_binding!(1, MousePlace),
        client_button_binding!(1, MousePlace),
        frame_button_binding!(2, Execute("mars-relay menu".to_owned())),
        client_button_binding!(2, Execute("mars-relay menu".to_owned())),
        client_button_binding!(2, CloseClient, (Shift)),
        frame_button_binding!(3, MouseResize),
        client_button_binding!(3, MouseResize),
        frame_button_binding!(3, MouseResizeCentered, (Control)),
        client_button_binding!(3, MouseResizeCentered, (Control)),
        client_button_binding!(4, CycleClient(-1)),
        client_button_binding!(4, StackMove(-1), (Shift)),
        client_button_binding!(5, CycleClient(1)),
        client_button_binding!(5, StackMove(1), (Shift)),
    ];
    bindings
}

