use std::cmp;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

use libmars::*;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum LayoutType {
    Floating,
    Stack,
}

pub struct Layout<C: Client> {
    layout_type: LayoutType,
    symbol: &'static str,
    label: &'static str,
    apply: fn(MonitorConfig, &VecDeque<Rc<RefCell<C>>>),
}

pub const LAYOUT_TYPES: &'static [LayoutType; 2] = & [
    LayoutType::Floating,
    LayoutType::Stack,
];

impl<C: Client> Layout<C> {
    pub fn get(layout_type: LayoutType) -> Layout<C> {
        return match layout_type {
            LayoutType::Floating => Layout {
                layout_type,
                symbol: "<><",
                label: "floating",
                apply: |_, _| {},
            },
            LayoutType::Stack => Layout {
                layout_type,
                symbol: "[]=",
                label: "stacking",
                apply: apply_layout_stack,
            },
        }
    }

    pub fn apply_layout(&self, monitor_conf: MonitorConfig, clients: &VecDeque<Rc<RefCell<C>>>) {
        (self.apply)(monitor_conf, clients);
    }
}

fn apply_layout_stack(monitor_conf: MonitorConfig, clients: &VecDeque<Rc<RefCell<impl Client>>>) {
    let nclients: u32 = clients.len().try_into().unwrap();
    let nmain = 2;

    if nclients == 0 {
        return;
    }

    let main_width = if nclients > nmain {
        (monitor_conf.window_area().w() as f32 * 0.6) as u32
    } else {
        monitor_conf.window_area().w()
    };
    let main_height = monitor_conf.window_area().h() / cmp::min(nclients, nmain);
    let stacked_width = monitor_conf.window_area().w() - main_width;
    let stacked_height = monitor_conf.window_area().h()
        / if nclients > nmain { nclients - nmain } else { 1 };

    for (i, client_rc) in clients.iter().enumerate() {
        if i < nmain.try_into().unwrap() { // main window(s)
            let y_offset: i32 = (i as u32 * main_height).try_into().unwrap();
            client_rc.borrow_mut().move_resize(
                monitor_conf.window_area().x(),
                monitor_conf.window_area().y() + y_offset,
                main_width,
                main_height);
        } else { // stack windows
            let i_stack: u32 = i as u32 - nmain;
            let x_offset: i32 = main_width.try_into().unwrap();
            let y_offset: i32 = (i_stack * stacked_height).try_into().unwrap();
            client_rc.borrow_mut().move_resize(
                monitor_conf.window_area().x() + x_offset,
                monitor_conf.window_area().y() + y_offset,
                stacked_width,
                stacked_height);
        }
    }
}
