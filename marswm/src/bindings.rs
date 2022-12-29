extern crate x11;

use serde::{Serialize, Deserialize};
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use x11::xlib::{Mod1Mask, Mod4Mask, ShiftMask, ControlMask};

use libmars::*;
use libmars::x11::get_keysym;
use crate::*;
use crate::layouts::*;

pub const MODKEY: Modifier = Modifier::Mod4;

#[derive(Serialize,Deserialize,Clone,Debug,PartialEq,Eq)]
#[serde(rename_all = "kebab-case")]
// #[serde(tag = "type", content = "arg")]
// #[serde(tag = "type")]
pub enum BindingAction {
    CenterClient,
    CloseClient,
    CycleClient(i32),
    CycleLayout,
    CycleWorkspace(i32),
    DecNMain,
    Execute(String),
    IncNMain,
    MoveWorkspace(u32),
    PreviousWorkspace,
    MoveMain,
    Restart,
    SetLayout(LayoutType),
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

#[derive(Serialize,Deserialize,PartialEq,Eq,Debug,Clone)]
pub struct Keybinding {
    modifiers: Vec<Modifier>,
    key_name: String,
    action: BindingAction,
}

impl BindingAction {
    pub fn execute<B: Backend<C>, C: Client>(&self, wm: &mut MarsWM<C>, backend: &mut B,
                                         client_option: Option<Rc<RefCell<C>>>) {
        use BindingAction::*;
        match self {
            CenterClient => if let Some(client_rc) = client_option {
                if let Some(mon) = wm.get_monitor(&client_rc) {
                    client_rc.borrow_mut().center_on_screen(mon.config());
                }
            },
            CloseClient => if let Some(client_rc) = client_option {
                client_rc.borrow().close();
            },
            CycleClient(inc) => wm.cycle_client(backend, *inc),
            CycleLayout => wm.current_workspace_mut(backend).cycle_layout(),
            CycleWorkspace(inc) => wm.cycle_workspace(backend, *inc),
            DecNMain => wm.current_workspace_mut(backend).dec_nmain(),
            Execute(cmd) => {
                if let Ok(mut handle) = std::process::Command::new("sh").arg("-c").arg(cmd).spawn() {
                    std::thread::spawn(move || {
                        let _ignored = handle.wait();
                    });
                }
            },
            IncNMain => wm.current_workspace_mut(backend).inc_nmain(),
            MoveWorkspace(ws) => if let Some(client_rc) = client_option {
                wm.move_to_workspace(backend, client_rc, *ws);
            },
            PreviousWorkspace => wm.current_monitor_mut(backend).switch_prev_workspace(backend),
            MoveMain => if let Some(client_rc) = client_option {
                wm.current_workspace_mut(backend).move_main(client_rc);
            },
            Restart => wm.restart(backend),
            SetLayout(layout) => wm.current_workspace_mut(backend).set_layout(*layout),
            SwitchWorkspace(ws) => wm.switch_workspace(backend, *ws),
            ToggleFloating => if let Some(client_rc) = client_option {
                if let Some(ws) = wm.get_workspace_mut(&client_rc) {
                    ws.toggle_floating(client_rc);
                    ws.apply_layout();
                }
            },
            ToggleFullscreen => if let Some(client_rc) = client_option {
                wm.handle_fullscreen_toggle(backend, client_rc);
            },
        }
    }
}

impl Keybinding {
    pub fn new(modifiers: Vec<Modifier>, key_name: &str, action: BindingAction) -> Keybinding {
        return Keybinding { modifiers, key_name: key_name.to_owned(), action };
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
        return get_keysym(&self.key_name) as u32;
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
        Keybinding::new(vec!(MODKEY), "Delete", CloseClient),
        Keybinding::new(vec!(MODKEY), "n", CycleLayout),
        Keybinding::new(vec!(MODKEY), "t", SetLayout(LayoutType::Stack)),
        Keybinding::new(vec!(MODKEY), "c", SetLayout(LayoutType::Deck)),
        Keybinding::new(vec!(MODKEY), "m", SetLayout(LayoutType::Monocle)),
        Keybinding::new(vec!(MODKEY), "BackSpace", MoveMain),
        Keybinding::new(vec!(MODKEY), "a", IncNMain),
        Keybinding::new(vec!(MODKEY), "x", DecNMain),
        Keybinding::new(vec!(MODKEY), "j", CycleClient(1)),
        Keybinding::new(vec!(MODKEY), "k", CycleClient(-1)),
        Keybinding::new(vec!(MODKEY), "period", CycleWorkspace(1)),
        Keybinding::new(vec!(MODKEY), "comma", CycleWorkspace(-1)),
        Keybinding::new(vec!(MODKEY), "f", ToggleFullscreen),
        Keybinding::new(vec!(MODKEY, Modifier::Shift), "f", ToggleFloating),
        Keybinding::new(vec!(MODKEY), "z", CenterClient),
        Keybinding::new(vec!(MODKEY), "Tab", PreviousWorkspace),
        Keybinding::new(vec!(MODKEY), "Return", Execute("buttermilk".to_owned())),
        Keybinding::new(vec!(MODKEY), "d", Execute("rofi -show drun".to_owned())),
        Keybinding::new(vec!(MODKEY, Modifier::Control), "BackSpace", Restart),
    ];

    for i in 0..cmp::min(nworkspaces, 9) {
        let key_name = format!("{}", i + 1);
        bindings.push(Keybinding::new(vec!(MODKEY), &key_name, SwitchWorkspace(i)));
        bindings.push(Keybinding::new(vec!(MODKEY, Modifier::Shift), &key_name, MoveWorkspace(i)));
    }

    return bindings;
}

