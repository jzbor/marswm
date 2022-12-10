use std::rc::Rc;
use std::cell::RefCell;

use libmars::*;

pub struct Keybinding<W: WindowManager<B, C>, B: Backend<C>, C: Client> {
    modifiers: u32,
    key: u32,
    action: fn(&mut W, &mut B, Option<Rc<RefCell<C>>>),
}

impl<W: WindowManager<B, C>, B: Backend<C>, C: Client> Keybinding<W, B, C> {
    pub fn new(modifiers: u32, key: u32, action: fn(&mut W, &mut B, Option<Rc<RefCell<C>>>))
            -> Keybinding<W, B, C> {
        return Keybinding { modifiers, key, action };
    }

    pub fn check(&self, modifiers: u32, key: u32, wm: &mut W, backend: &mut B, client_option: Option<Rc<RefCell<C>>>) -> bool {
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

