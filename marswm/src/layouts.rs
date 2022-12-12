use std::cmp;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

use libmars::*;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum LayoutType {
    Floating,
    Stack,
    Monocle,
}

pub struct Layout<C: Client> {
    _layout_type: LayoutType,
    _symbol: &'static str,
    _label: &'static str,
    apply: fn(Dimensions, &VecDeque<Rc<RefCell<C>>>, u32),
}

pub const LAYOUT_TYPES: &'static [LayoutType; 3] = & [
    LayoutType::Floating,
    LayoutType::Stack,
    LayoutType::Monocle,
];

impl<C: Client> Layout<C> {
    pub fn get(layout_type: LayoutType) -> Layout<C> {
        return match layout_type {
            LayoutType::Floating => Layout {
                _layout_type: layout_type,
                _symbol: "<><",
                _label: "floating",
                apply: |_, _, _| {},
            },
            LayoutType::Stack => Layout {
                _layout_type: layout_type,
                _symbol: "[]=",
                _label: "stacking",
                apply: apply_layout_stack,
            },
            LayoutType::Monocle => Layout {
                _layout_type: layout_type,
                _symbol: "[M]",
                _label: "monocle",
                apply: apply_layout_monocle,
            },
        }
    }

    pub fn apply_layout(&self, win_area: Dimensions, clients: &VecDeque<Rc<RefCell<C>>>, nmain: u32) {
        (self.apply)(win_area, clients, nmain);
    }
}

fn apply_layout_stack(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<impl Client>>>, nmain: u32) {
    let nclients: u32 = clients.len().try_into().unwrap();

    if nclients == 0 {
        return;
    }

    let (main_width, main_height, stacked_width, stacked_height) = {
        if nmain == 0 { // no windows in main area
            (0, 0, win_area.w(), win_area.h() / nclients)
        } else if nclients <= nmain { // no windows in stack area
            (win_area.w(), win_area.h() / nclients, 0, 0)
        } else {
            let main_width = (win_area.w() as f32 * 0.6) as u32;
            let main_height = win_area.h() / cmp::min(nclients, nmain);
            let stacked_width = win_area.w() - main_width;
            let stacked_height = win_area.h() / (nclients - nmain);
            (main_width, main_height, stacked_width, stacked_height)
        }
    };

    for (i, client_rc) in clients.iter().enumerate() {
        if i < nmain.try_into().unwrap() { // main window(s)
            let y_offset: i32 = (i as u32 * main_height).try_into().unwrap();
            client_rc.borrow_mut().move_resize(
                win_area.x(),
                win_area.y() + y_offset,
                main_width,
                main_height);
        } else { // stack windows
            let i_stack: u32 = i as u32 - nmain;
            let x_offset: i32 = main_width.try_into().unwrap();
            let y_offset: i32 = (i_stack * stacked_height).try_into().unwrap();
            client_rc.borrow_mut().move_resize(
                win_area.x() + x_offset,
                win_area.y() + y_offset,
                stacked_width,
                stacked_height);
        }
    }
}

fn apply_layout_monocle(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<impl Client>>>, nmain: u32) {
    for client_rc in clients {
        client_rc.borrow_mut().move_resize(
            win_area.x(),
            win_area.y(),
            win_area.w(),
            win_area.h());
    }
}
