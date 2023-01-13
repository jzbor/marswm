use std::rc::Rc;
use std::cell::RefCell;
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

use libmars::*;
use crate::config::LayoutConfiguration;


enum_with_values! {
    #[derive(Serialize,Deserialize,Clone,Copy,Debug,PartialEq,Eq)]
    #[serde(rename_all = "kebab-case")]
    vis pub enum LayoutType {
        Floating,
        Stack,
        BottomStack,
        Monocle,
        Deck,
        Dynamic
    }
}

enum_with_values! {
    #[derive(Serialize,Deserialize,Clone,Copy,Debug,PartialEq,Eq)]
    #[serde(rename_all = "kebab-case")]
    vis pub enum StackPosition {
        Right,
        Left,
        Bottom,
        Top
    }
}

enum_with_values! {
    #[derive(Serialize,Deserialize,Clone,Copy,Debug,PartialEq,Eq)]
    #[serde(rename_all = "kebab-case")]
    vis pub enum StackMode {
        Deck,
        Split
    }
}

pub struct Layout<C: Client> {
    apply: fn(Dimensions, &VecDeque<Rc<RefCell<C>>>, &LayoutConfiguration),
}

impl<C: Client> Layout<C> {
    pub fn get(layout_type: LayoutType) -> Layout<C> {
        return match layout_type {
            LayoutType::Floating => Layout {
                apply: |_, _, _| {},
            },
            LayoutType::Stack => Layout {
                apply: apply_layout_stack,
            },
            LayoutType::BottomStack => Layout {
                apply: apply_layout_bottom_stack,
            },
            LayoutType::Monocle => Layout {
                apply: apply_layout_monocle,
            },
            LayoutType::Deck => Layout {
                apply: apply_layout_deck,
            },
            LayoutType::Dynamic => Layout {
                apply: apply_layout_dynamic,
            },
        }
    }

    pub fn apply_layout(&self, win_area: Dimensions, clients: &VecDeque<Rc<RefCell<C>>>, config: &LayoutConfiguration) {
        (self.apply)(win_area, clients, config);
    }
}

fn apply_layout_bottom_stack<C: Client>(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<C>>>, config: &LayoutConfiguration) {
    let mut config = config.clone();
    config.stack_position = StackPosition::Bottom;
    config.stack_mode = StackMode::Split;
    apply_layout_dynamic(win_area, clients, &config);
}

fn apply_layout_dynamic<C: Client>(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<C>>>, config: &LayoutConfiguration) {
    let nclients: u32 = clients.len().try_into().unwrap();
    let mut clients = clients.iter();
    let main_clients = (&mut clients).take(config.nmain.try_into().unwrap()).collect();
    let stack_clients = clients.collect();

    let (areas, swap) = match config.stack_position {
        StackPosition::Left => (layout_dimensions_horizontal(win_area, 1.0 - config.main_ratio, config.gap_width,
                                                             config.nmain, nclients), true),
        StackPosition::Top => (layout_dimensions_vertical(win_area, 1.0 - config.main_ratio, config.gap_width,
                                                          config.nmain, nclients), true),
        StackPosition::Right => (layout_dimensions_horizontal(win_area, config.main_ratio, config.gap_width,
                                                              config.nmain, nclients), false),
        StackPosition::Bottom => (layout_dimensions_vertical(win_area, config.main_ratio, config.gap_width,
                                                             config.nmain, nclients), false),
    };

    let (main_area, stack_area) = if swap {
        let (main, stack) = areas;
        (stack, main)
    } else {
        areas
    };

    match config.stack_position {
        StackPosition::Left | StackPosition::Right => {
            stack_clients_vertically(main_area, main_clients, config.gap_width);
        },
        StackPosition::Top | StackPosition::Bottom => {
            stack_clients_horizontally(main_area, main_clients, config.gap_width);
        }
    }

    match config.stack_mode {
        StackMode::Deck => stack_clients_ontop(stack_area, stack_clients),
        StackMode::Split => match config.stack_position {
            StackPosition::Left | StackPosition::Right => {
                stack_clients_vertically(stack_area, stack_clients, config.gap_width);
            },
            StackPosition::Top | StackPosition::Bottom => {
                stack_clients_horizontally(stack_area, stack_clients, config.gap_width);
            }
        }
    }
}

fn apply_layout_stack<C: Client>(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<C>>>, config: &LayoutConfiguration) {
    let mut config = config.clone();
    config.stack_position = StackPosition::Right;
    config.stack_mode = StackMode::Split;
    apply_layout_dynamic(win_area, clients, &config);
}

fn apply_layout_monocle(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<impl Client>>>, _config: &LayoutConfiguration) {
    let clients = clients.iter().collect();
    stack_clients_ontop(win_area, clients);
}

fn apply_layout_deck(win_area: Dimensions, clients: &VecDeque<Rc<RefCell<impl Client>>>, config: &LayoutConfiguration) {
    let mut config = config.clone();
    config.stack_position = StackPosition::Right;
    config.stack_mode = StackMode::Deck;
    apply_layout_dynamic(win_area, clients, &config);
}

fn layout_dimensions_horizontal(win_area: Dimensions, ratio: f32, gap_width: u32, nmain: u32, nclients: u32) -> (Dimensions, Dimensions) {
    let first_width: u32 = (win_area.w() as f32 * ratio) as u32;
    let gap_share = (gap_width / 2, gap_width - (gap_width / 2));

    return {
        if nmain == 0 {  // all windows in stack area
            let first_area = Dimensions::new(0, 0, 0, 0);
            let second_x = win_area.x() + gap_width as i32;
            let second_y = win_area.y() + gap_width as i32;
            let second_w = win_area.w() - 2 * gap_width;
            let second_h = win_area.h() - 2 * gap_width;
            let second_area = Dimensions::new(second_x, second_y, second_w, second_h);
            (first_area, second_area)
        } else if nclients <= nmain {  // no windows in stack area
            let second_area = Dimensions::new(0, 0, 0, 0);
            let first_x = win_area.x() + gap_width as i32;
            let first_y = win_area.y() + gap_width as i32;
            let first_w = win_area.w() - 2 * gap_width;
            let first_h = win_area.h() - 2 * gap_width;
            let first_area = Dimensions::new(first_x, first_y, first_w, first_h);
            (first_area, second_area)
        } else {
            let first_x = win_area.x() + gap_width as i32;
            let first_y = win_area.y() + gap_width as i32;
            let first_w = first_width - gap_width - gap_share.0;
            let first_h = win_area.h() - 2 * gap_width;
            let first_area = Dimensions::new(first_x, first_y, first_w, first_h);

            let second_x = win_area.x() + first_width as i32 + gap_share.0 as i32;
            let second_y = win_area.y() + gap_width as i32;
            let second_w = win_area.w() - first_width - gap_width - gap_share.1;
            let second_h = win_area.h() - 2 * gap_width;
            let second_area = Dimensions::new(second_x, second_y, second_w, second_h);
            (first_area, second_area)
        }
    };
}

fn layout_dimensions_vertical(win_area: Dimensions, ratio: f32, gap_width: u32, nmain: u32, nclients: u32) -> (Dimensions, Dimensions) {
    let first_height: u32 = (win_area.h() as f32 * ratio) as u32;
    let gap_share = (gap_width / 2, gap_width - (gap_width / 2));

    return {
        if nmain == 0 {  // all windows in stack area
            let first_area = Dimensions::new(0, 0, 0, 0);
            let second_x = win_area.x() + gap_width as i32;
            let second_y = win_area.y() + gap_width as i32;
            let second_w = win_area.w() - 2 * gap_width;
            let second_h = win_area.h() - 2 * gap_width;
            let second_area = Dimensions::new(second_x, second_y, second_w, second_h);
            (first_area, second_area)
        } else if nclients <= nmain {  // no windows in stack area
            let second_area = Dimensions::new(0, 0, 0, 0);
            let first_x = win_area.x() + gap_width as i32;
            let first_y = win_area.y() + gap_width as i32;
            let first_w = win_area.w() - 2 * gap_width;
            let first_h = win_area.h() - 2 * gap_width;
            let first_area = Dimensions::new(first_x, first_y, first_w, first_h);
            (first_area, second_area)
        } else {
            let first_x = win_area.x() + gap_width as i32;
            let first_y = win_area.y() + gap_width as i32;
            let first_w = win_area.w() - 2 * gap_width;
            let first_h = first_height - gap_width - gap_share.0;
            let first_area = Dimensions::new(first_x, first_y, first_w, first_h);

            let second_x = win_area.x() + gap_width as i32;
            let second_y = win_area.y() + first_height as i32 + gap_share.0 as i32;
            let second_w = win_area.w() - 2 * gap_width;
            let second_h = win_area.h() - first_height - gap_width - gap_share.0;
            let second_area = Dimensions::new(second_x, second_y, second_w, second_h);
            (first_area, second_area)
        }
    };
}

fn stack_clients_horizontally(area: Dimensions, clients: Vec<&Rc<RefCell<impl Client>>>, gap_width: u32) {
    let nclients: u32 = clients.len().try_into().unwrap();
    if nclients == 0 {
        return;
    }

    let width = (area.w() - ((nclients - 1) * gap_width)) / nclients;
    let height = area.h();
    for (i, client_rc) in clients.iter().enumerate() {
        if !client_rc.borrow().is_fullscreen() {
            let x: i32 = area.x() + (i as i32 * (width + gap_width) as i32);
            let y: i32 = area.y();
            client_rc.borrow_mut().move_resize(x, y, width, height);
        }
    }
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
