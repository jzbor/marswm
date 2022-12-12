extern crate x11;

use std::rc::Rc;
use std::cell::RefCell;
use x11::keysym::*;
use x11::xlib::{Mod1Mask, ShiftMask};

use libmars::*;
use crate::*;

pub const MODKEY: u32 = Mod1Mask;

macro_rules! switch_workspace_binding {
    ($key:expr, $ws:expr) => {
        Keybinding::new(MODKEY, $key, |wm: &mut MarsWM<C>, backend, _client_option| {
            wm.switch_workspace(backend, $ws);
        })
    }
}

macro_rules! move_workspace_binding {
    ($key:expr, $ws:expr) => {
        Keybinding::new(MODKEY|ShiftMask, $key, |wm: &mut MarsWM<C>, backend, client_option| {
            if let Some(client_rc) = client_option {
                wm.move_to_workspace(backend, client_rc, $ws)
            }
        })
    }
}

#[macro_export]
macro_rules! execute_binding {
    ($mods:expr, $key:expr, $cmd:expr) => {
        Keybinding::new($mods, $key, |_wm, _backend, _client_option| {
            std::process::Command::new("sh").arg("-c").arg($cmd).spawn();
        })
    }
}

pub struct Keybinding<B: Backend<C>, C: Client> {
    modifiers: u32,
    key: u32,
    action: fn(&mut MarsWM<C>, &mut B, Option<Rc<RefCell<C>>>),
}

impl<B: Backend<C>, C: Client> Keybinding<B, C> {
    pub fn new(modifiers: u32, key: u32, action: fn(&mut MarsWM<C>, &mut B, Option<Rc<RefCell<C>>>))
            -> Keybinding<B, C> {
        return Keybinding { modifiers, key, action };
    }

    pub fn check(&self, modifiers: u32, key: u32, wm: &mut MarsWM<C>, backend: &mut B, client_option: Option<Rc<RefCell<C>>>) -> bool {
        if modifiers == self.modifiers && key == self.key {
            (self.action)(wm, backend, client_option);
            return true;
        }
        return false;
    }

    pub fn modifiers(&self) -> u32 {
        return self.modifiers;
    }

    pub fn key(&self) -> u32 {
        return self.key;
    }
}

pub fn keybindings<B: Backend<C>, C: Client>() -> Vec<Keybinding<B, C>> {
    vec![
        Keybinding::new(MODKEY, XK_Delete, |_wm, _backend, client_option: Option<Rc<RefCell<C>>>| {
            if let Some(client_rc) = client_option {
                client_rc.borrow().close();
            }
        }),
        Keybinding::new(MODKEY, XK_t, |wm: &mut MarsWM<C>, _backend, _client_option| {
            wm.cycle_current_layout();
        }),
        Keybinding::new(MODKEY, XK_BackSpace, |wm: &mut MarsWM<C>, _backend, client_option| {
            if let Some(client_rc) = client_option {
                wm.pull_current_front(client_rc);
            }
        }),

        switch_workspace_binding!(XK_1, 0),
        switch_workspace_binding!(XK_2, 1),
        switch_workspace_binding!(XK_3, 2),
        switch_workspace_binding!(XK_4, 3),
        move_workspace_binding!(XK_1, 0),
        move_workspace_binding!(XK_2, 1),
        move_workspace_binding!(XK_3, 2),
        move_workspace_binding!(XK_4, 3),
        execute_binding!(MODKEY, XK_Return, "xterm"),
    ]
}

