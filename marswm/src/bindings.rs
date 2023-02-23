extern crate x11;

use libmars::common::x11::get_keysym;
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
    ($button:expr, $action:expr) => {
        ButtonBinding::new(vec![], $button, vec![Frame], $action)
    }
}



#[derive(Serialize,Deserialize,Clone,Debug,PartialEq)]
#[serde(rename_all = "kebab-case")]
// #[serde(tag = "action", content = "arg")]
// #[serde(tag = "type")]
pub enum BindingAction {
    CenterClient,
    ChangeMainRatio(f32),
    CloseClient,
    CycleClient(i32),
    CycleLayout,
    CycleWorkspace(i32),
    Execute(String),
    Exit,
    IncNMain(i32),
    MoveWorkspace(u32),
    PreviousWorkspace,
    MouseMove,
    MouseResize,
    MoveMain,
    Restart,
    SetLayout(LayoutType),
    SetStackMode(StackMode),
    SetStackPosition(StackPosition),
    StackMove(i32),
    SwitchWorkspace(u32),
    ToggleFloating,
    ToggleFullscreen,
}

#[derive(Serialize,Deserialize,Clone,Debug,PartialEq,Eq)]
pub enum Modifier {
    Mod1,
    Mod4,
    Shift,
    Control,
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
pub struct KeyBinding {
    /// list of modifiers that apply to this binding
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
                if let Some(mon) = wm.get_monitor(&client_rc) {
                    client_rc.borrow_mut().center_on_screen(mon.config().window_area());
                }
            },
            ChangeMainRatio(f) =>  wm.current_workspace_mut(backend).change_main_ratio(*f),
            CloseClient => if let Some(client_rc) = client_option {
                client_rc.borrow().close();
            },
            CycleClient(inc) => wm.cycle_client(backend, *inc),
            CycleLayout => wm.current_workspace_mut(backend).cycle_layout(),
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
            IncNMain(i) => wm.current_workspace_mut(backend).inc_nmain(*i),
            MouseMove => if let Some(client_rc) = client_option {
                backend.mouse_move(wm, client_rc);
                wm.current_monitor_mut(backend).restack_current();
            },
            MouseResize => if let Some(client_rc) = client_option {
                backend.mouse_resize(wm, client_rc);
                wm.current_monitor_mut(backend).restack_current();
            },
            MoveWorkspace(ws) => if let Some(client_rc) = client_option {
                if let Some(monitor) = wm.get_monitor_mut(&client_rc) {
                    monitor.move_to_workspace(client_rc, *ws);
                }
            },
            PreviousWorkspace => wm.current_monitor_mut(backend).switch_prev_workspace(backend),
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
            SwitchWorkspace(ws) => wm.current_monitor_mut(backend).switch_workspace(backend, *ws),
            ToggleFloating => if let Some(client_rc) = client_option {
                if let Some(ws) = wm.get_workspace_mut(&client_rc) {
                    ws.toggle_floating(client_rc);
                    ws.apply_layout();
                }
            },
            ToggleFullscreen => if let Some(client_rc) = client_option {
                wm.toggle_fullscreen_client(backend, client_rc);
            },
        }
    }
}

impl KeyBinding {
    pub fn new(modifiers: Vec<Modifier>, key: &str, action: BindingAction) -> Self {
        return KeyBinding { modifiers, key: key.to_owned(), action };
    }

    pub fn action(&self) -> BindingAction {
        return self.action.clone();
    }

    pub fn matches(&self, modifiers: u32, key: u32) -> bool {
        return modifiers == self.modifiers() && key == self.key();
    }

    pub fn modifiers(&self) -> u32 {
        return self.modifiers.iter().fold(0, |a, b| a | b.mask());
    }

    pub fn key(&self) -> u32 {
        return get_keysym(&self.key) as u32;
    }
}

impl ButtonBinding {
    pub fn new(modifiers: Vec<Modifier>, button: u32, targets: Vec<ButtonTarget>, action: BindingAction) -> Self {
        return ButtonBinding { modifiers, button, targets, action };
    }

    pub fn action(&self) -> BindingAction {
        return self.action.clone();
    }

    pub fn button(&self) -> u32 {
        return self.button;
    }

    pub fn matches(&self, modifiers: u32, button: u32, target: ButtonTarget) -> bool {
        return modifiers == self.modifiers() && button == self.button && self.targets.iter().any(|t| *t == target);
    }

    pub fn modifiers(&self) -> u32 {
        return self.modifiers.iter().fold(0, |a, b| a | b.mask());
    }

    pub fn targets(&self) -> &[ButtonTarget] {
        return &self.targets;
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
        KeyBinding::new(vec![DEFAULT_MODKEY], "j", CycleClient(1)),
        KeyBinding::new(vec![DEFAULT_MODKEY], "k", CycleClient(-1)),
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
    ];

    for i in 0..cmp::min(nworkspaces, 9) {
        let key_name = format!("{}", i + 1);
        bindings.push(KeyBinding::new(vec!(DEFAULT_MODKEY), &key_name, SwitchWorkspace(i)));
        bindings.push(KeyBinding::new(vec!(DEFAULT_MODKEY, Modifier::Shift), &key_name, MoveWorkspace(i)));
    }

    return bindings;
}

pub fn default_button_bindings() -> Vec<ButtonBinding> {
    use BindingAction::*;
    use ButtonTarget::*;
    use Modifier::*;
    let bindings = vec![
        frame_button_binding!(1, MouseMove),
        client_button_binding!(1, MouseMove),
        frame_button_binding!(2, Execute("mars-relay menu".to_owned())),
        client_button_binding!(2, Execute("mars-relay menu".to_owned())),
        client_button_binding!(2, CloseClient, (Shift)),
        frame_button_binding!(3, MouseResize),
        client_button_binding!(3, MouseResize),
    ];
    return bindings;
}

