use std::rc::Rc;
use std::cell::RefCell;
use serde::{Serialize, Deserialize};
use libmars::interfaces::wm::Client;

use crate::attributes::*;
use crate::bindings::*;
use crate::config::WindowPlacement;


#[derive(Serialize,Deserialize,Default,PartialEq,Debug,Clone)]
#[serde(default)]
pub struct Rule {
    identifiers: Identifiers,
    actions: Vec<BindingAction>,

    /// do not manage this window
    ignore_window: bool,

    /// where should windows be placed initially
    initial_placement: Option<WindowPlacement>,

    /// initially make this window floating
    floating: Option<bool>,

    /// preferred workspace for the application to get launched on
    workspace: Option<u32>,
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
    pub fn matches(&self, client_rc: Rc<RefCell<impl Client<Attributes>>>) -> bool {
        self.identifiers.matches(client_rc)
    }

    pub fn actions(&self) -> &Vec<BindingAction> {
        &self.actions
    }

    pub fn floating(&self) -> Option<bool> {
        self.floating
    }

    pub fn ignore_window(&self) -> bool {
        self.ignore_window
    }

    pub fn initial_placement(&self) -> Option<WindowPlacement> {
        self.initial_placement
    }

    pub fn workspace(&self) -> Option<u32> {
        self.workspace
    }
}

impl Identifiers {
    pub fn matches(&self, client_rc: Rc<RefCell<impl Client<Attributes>>>) -> bool {
        return (self.application.is_none() || self.application == Some(client_rc.borrow().application()))
            && (self.title.is_none() || self.title == Some(client_rc.borrow().title()));
    }
}
