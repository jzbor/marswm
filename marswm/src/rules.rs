use std::rc::Rc;
use std::cell::RefCell;
use serde::{Serialize, Deserialize};
use libmars::wm::Client;

use crate::bindings::*;


#[derive(Serialize,Deserialize,Default,PartialEq,Debug,Clone)]
#[serde(default)]
pub struct Rule {
    identifiers: Identifiers,
    actions: Vec<BindingAction>,

    /// do not manage this window
    ignore_window: bool,
}

#[derive(Serialize,Deserialize,Default,PartialEq,Debug,Clone)]
#[serde(default)]
struct Identifiers {
    /// application name - WM class for X11 (exact match)
    application: Option<String>,

    /// title (exact match)
    title: Option<String>,
}

impl Rule {
    pub fn matches(&self, client_rc: Rc<RefCell<impl Client>>) -> bool {
        return self.identifiers.matches(client_rc);
    }

    pub fn actions(&self) -> &Vec<BindingAction> {
        return &self.actions;
    }

    pub fn ignore_window(&self) -> bool {
        return self.ignore_window;
    }
}

impl Identifiers {
    pub fn matches(&self, client_rc: Rc<RefCell<impl Client>>) -> bool {
        return (self.application.is_none() || self.application == Some(client_rc.borrow().application()))
            && (self.title.is_none() || self.title == Some(client_rc.borrow().title()));
    }
}
