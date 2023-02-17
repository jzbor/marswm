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
pub struct Keybinding {
    /// list of modifiers that apply to this binding
    modifiers: Vec<Modifier>,

    /// key name (as found in
    /// [keysymdef.h](https://cgit.freedesktop.org/xorg/proto/x11proto/tree/keysymdef.h) without
    /// the leading "XK_")
    key: String,

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
                    client_rc.borrow_mut().center_on_screen(mon.config());
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
            MoveWorkspace(ws) => if let Some(client_rc) = client_option {
                wm.move_to_workspace(backend, client_rc, *ws);
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
            SwitchWorkspace(ws) => wm.switch_workspace(backend, *ws),
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

impl Keybinding {
    pub fn new(modifiers: Vec<Modifier>, key: &str, action: BindingAction) -> Keybinding {
        return Keybinding { modifiers, key: key.to_owned(), action };
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

pub fn default_keybindings(nworkspaces: u32) -> Vec<Keybinding> {
    use BindingAction::*;
    let mut bindings = vec![
        Keybinding::new(vec!(DEFAULT_MODKEY), "Delete", CloseClient),
        Keybinding::new(vec!(DEFAULT_MODKEY), "n", CycleLayout),
        Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Shift), "t", SetLayout(LayoutType::Stack)),
        Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Control), "t", SetLayout(LayoutType::BottomStack)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "c", SetLayout(LayoutType::Deck)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "m", SetLayout(LayoutType::Monocle)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "t", SetLayout(LayoutType::Dynamic)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "BackSpace", MoveMain),
        Keybinding::new(vec!(DEFAULT_MODKEY), "a", IncNMain(1)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "x", IncNMain(-1)),
        Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Control), "a", ChangeMainRatio(0.10)),
        Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Control), "x", ChangeMainRatio(-0.10)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "j", CycleClient(1)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "k", CycleClient(-1)),
        Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Shift), "j", StackMove(1)),
        Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Shift), "k", StackMove(-1)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "period", CycleWorkspace(1)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "comma", CycleWorkspace(-1)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "f", ToggleFullscreen),
        Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Shift), "f", ToggleFloating),
        Keybinding::new(vec!(DEFAULT_MODKEY), "z", CenterClient),
        Keybinding::new(vec!(DEFAULT_MODKEY), "Tab", PreviousWorkspace),
        Keybinding::new(vec!(DEFAULT_MODKEY), "Return", Execute("$TERMINAL".to_owned())),
        Keybinding::new(vec!(DEFAULT_MODKEY), "d", Execute("rofi -show drun".to_owned())),
        Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Control), "BackSpace", Restart),
        Keybinding::new(vec!(DEFAULT_MODKEY), "Up", SetStackPosition(StackPosition::Top)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "Right", SetStackPosition(StackPosition::Right)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "Down", SetStackPosition(StackPosition::Bottom)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "Left", SetStackPosition(StackPosition::Left)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "semicolon", SetStackMode(StackMode::Split)),
        Keybinding::new(vec!(DEFAULT_MODKEY), "apostrophe", SetStackMode(StackMode::Deck)),
    ];

    for i in 0..cmp::min(nworkspaces, 9) {
        let key_name = format!("{}", i + 1);
        bindings.push(Keybinding::new(vec!(DEFAULT_MODKEY), &key_name, SwitchWorkspace(i)));
        bindings.push(Keybinding::new(vec!(DEFAULT_MODKEY, Modifier::Shift), &key_name, MoveWorkspace(i)));
    }

    return bindings;
}

