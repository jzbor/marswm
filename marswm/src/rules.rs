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
}

#[derive(Serialize,Deserialize,Default,PartialEq,Debug,Clone)]
#[serde(default)]
struct Identifiers {
    application: Option<String>,
    title: Option<String>,
}

impl Rule {
    pub fn matches(&self, client_rc: Rc<RefCell<impl Client>>) -> bool {
        return self.identifiers.matches(client_rc);
    }

    pub fn actions(&self) -> &Vec<BindingAction> {
        return &self.actions;
    }
}

impl Identifiers {
    pub fn matches(&self, client_rc: Rc<RefCell<impl Client>>) -> bool {
        return (self.application.is_none() || self.application == Some(client_rc.borrow().application().to_owned()))
            && (self.title.is_none() || self.title == Some(client_rc.borrow().title()));
    }
}
