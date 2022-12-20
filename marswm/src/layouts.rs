use std::cmp;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;

use libmars::*;
use crate::marswm::*;


enum_with_values! {
    #[derive(Clone,Copy,Debug,PartialEq,Eq)]
    vis pub enum LayoutType {
        Floating,
        Stack,
        Monocle,
        Deck
    }
}

pub struct Layout<C: Client> {
    _layout_type: LayoutType,
    _symbol: &'static str,
    _label: &'static str,
    apply: fn(Dimensions, &VecDeque<Rc<RefCell<C>>>, u32),
}

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
            LayoutType::Deck => Layout {
                _layout_type: layout_type,
                _symbol: "[]D",
                _label: "deck",
                apply: apply_layout_deck,
            },
        }
    }

    pub fn apply_layout(&self, win_area: Dimensions, clients: &VecDeque<Rc<RefCell<C>>>, nmain: u32) {
        (self.apply)(win_area, clients, nmain);
    }
}

fn apply_layout_stack<C: Client>(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<C>>>, nmain: u32) {
    let nclients: u32 = clients.len().try_into().unwrap();
    let mut clients = clients.iter();
    let main_clients = (&mut clients).take(nmain.try_into().unwrap()).collect();
    let stack_clients = clients.collect();
    let gap_width = GAP_WIDTH;

    let (main_area, stack_area) = layout_dimensions_horizontal(win_area, MAIN_RATIO, gap_width, nmain, nclients);

    stack_clients_vertically(main_area, main_clients, gap_width);
    stack_clients_vertically(stack_area, stack_clients, gap_width);
}

fn apply_layout_monocle(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<impl Client>>>, _nmain: u32) {
    let clients = clients.iter().collect();
    stack_clients_ontop(win_area, clients);
}

fn apply_layout_deck(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<impl Client>>>, nmain: u32) {
    let nclients: u32 = clients.len().try_into().unwrap();
    let mut clients = clients.iter();
    let main_clients = (&mut clients).take(nmain.try_into().unwrap()).collect();
    let stack_clients = clients.collect();
    let gap_width = GAP_WIDTH;

    let (main_area, stack_area) = layout_dimensions_horizontal(win_area, MAIN_RATIO, gap_width, nmain, nclients);

    stack_clients_vertically(main_area, main_clients, gap_width);
    stack_clients_ontop(stack_area, stack_clients);
}

fn layout_dimensions_horizontal(win_area: Dimensions, main_ratio: f32, gap_width: u32, nmain: u32, nclients: u32) -> (Dimensions, Dimensions) {
    let main_width: u32 = (win_area.w() as f32 * main_ratio) as u32;

    let (main_area, stack_area) = {
        if nmain == 0 {  // all windows in stack area
            let main_area = Dimensions::new(0, 0, 0, 0);
            let stack_x = win_area.x() + gap_width as i32;
            let stack_y = win_area.y() + gap_width as i32;
            let stack_w = win_area.w() - 2 * gap_width;
            let stack_h = win_area.h() - 2 * gap_width;
            let stack_area = Dimensions::new(stack_x, stack_y, stack_w, stack_h);
            (main_area, stack_area)
        } else if nclients <= nmain {  // no windows in stack area
            let stack_area = Dimensions::new(0, 0, 0, 0);
            let main_x = win_area.x() + gap_width as i32;
            let main_y = win_area.y() + gap_width as i32;
            let main_w = win_area.w() - 2 * gap_width;
            let main_h = win_area.h() - 2 * gap_width;
            let main_area = Dimensions::new(main_x, main_y, main_w, main_h);
            (main_area, stack_area)
        } else {
            let main_x = win_area.x() + gap_width as i32;
            let main_y = win_area.y() + gap_width as i32;
            let main_w = main_width - 2 * gap_width;
            let main_h = win_area.h() - 2 * gap_width;
            let main_area = Dimensions::new(main_x, main_y, main_w, main_h);

            let stack_x = win_area.x() + main_width as i32;
            let stack_y = win_area.y() + gap_width as i32;
            let stack_w = win_area.w() - main_width - gap_width;  // center gap already included
            let stack_h = win_area.h() - 2 * gap_width;
            let stack_area = Dimensions::new(stack_x, stack_y, stack_w, stack_h);
            (main_area, stack_area)
        }
    };
    return (main_area, stack_area);
}

fn stack_clients_vertically(area: Dimensions, clients: Vec<&Rc<RefCell<impl Client>>>, gap_width: u32) {
    let nclients: u32 = clients.len().try_into().unwrap();
    if nclients == 0 {
        return;
    }

    let width = area.w();
    let height = (area.h() - ((nclients - 1) * gap_width)) / nclients;
    for (i, client_rc) in clients.iter().enumerate() {
        if !client_rc.borrow().is_fullscreen() {
            let x: i32 = area.x();
            let y: i32 = area.y() + (i as i32 * (height + gap_width) as i32);
            client_rc.borrow_mut().move_resize(x, y, width, height);
        }
    }
}

fn stack_clients_ontop(area: Dimensions, clients: Vec<&Rc<RefCell<impl Client>>>) {
    for client_rc in clients {
        if !client_rc.borrow().is_fullscreen() {
            client_rc.borrow_mut().move_resize(area.x(), area.y(), area.w(), area.h());
        }
    }
}
