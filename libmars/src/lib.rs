extern crate x11 as x11_crate;

use std::rc::Rc;
use std::cell::RefCell;

pub mod x11;


pub trait WindowManager<B: Backend<C>, C: Client> {
    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<C>>> + '_>;
    fn clients_mut(&mut self) -> Box<dyn Iterator<Item = &mut Rc<RefCell<C>>> + '_>;
    fn handle_button(&mut self, backend: &mut B, modifiers: u32, button: u32, client_option: Option<Rc<RefCell<C>>>);
    fn handle_focus(&mut self, backend: &mut B, client_option: Option<Rc<RefCell<C>>>);
    fn handle_key(&mut self, backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<C>>>);
    fn init(&mut self, backend: &mut B);
    fn manage(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>);
    fn unmanage(&mut self, backend: &mut B, client_rc: Rc<RefCell<C>>);
}

pub trait Client: PartialEq {
    fn bind_button(&mut self, modifiers: u32, button: u32);
    fn bind_key(&mut self, modifiers: u32, key: u32);
    fn close(&self);
    fn hide(&mut self);
    fn move_resize(&mut self, x: i32, y: i32, width: u32, height: u32);
    fn raise(&self);
    fn set_frame_color(&mut self, color: u64);
    fn set_frame_width(&mut self, width: u32);
    fn set_height(&mut self, height: u32);
    fn set_inner_bw(&mut self, bw: u32);
    fn set_inner_color(&mut self, color: u64);
    fn set_outer_bw(&mut self, bw: u32);
    fn set_outer_color(&mut self, color: u64);
    fn show(&mut self);
}

pub trait Backend<C: Client> {
    /// Run window manager event loop
    fn run(self, wm: &mut (dyn WindowManager<Self, C>));

    /// Handle windows existing before initialization
    fn handle_existing_windows(&mut self, wm: &mut dyn WindowManager<Self, C>);

    /// Move client with mouse
    fn mouse_move(&mut self, wm: &mut dyn WindowManager<Self, C>, client_rc: Rc<RefCell<C>>);

    /// Resize client with mouse
    fn mouse_resize(&mut self, wm: &mut dyn WindowManager<Self, C>, client_rc: Rc<RefCell<C>>);

    /// Get position of pointer on screen
    fn pointer_pos(&self) -> (i32, i32);

    /// Set client that receives mouse and keyboard inputs
    fn set_input_focus(&self, client_rc: Rc<RefCell<C>>);
}

pub trait Dimensioned {
    /// Get x coordinate
    fn x(&self) -> i32;

    /// Get y coordinate
    fn y(&self) -> i32;

    /// Get width
    fn w(&self) -> u32;

    /// Get height
    fn h(&self) -> u32;

    /// Get position
    fn pos(&self) -> (i32, i32);

    /// Get size
    fn size(&self) -> (u32, u32);

    /// Set x coordinate
    fn set_x(&mut self, x: i32);

    /// Set y coordinate
    fn set_y(&mut self, y: i32);

    /// Set width
    fn set_w(&mut self, w: u32);

    /// Set height
    fn set_h(&mut self, h: u32);

    /// Set position
    fn set_pos(&mut self, pos: (i32, i32));

    /// Set size
    fn set_size(&mut self, size: (u32, u32));

    /// Get [Dimensions]
    fn dimensions(&self) -> Dimensions;
}

#[derive(Copy,Clone)]
pub struct Dimensions {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl Dimensions {
    fn new(x: i32, y: i32, w: u32, h: u32) -> Dimensions {
        return Dimensions {x, y, w, h};
    }
}

impl Dimensioned for Dimensions {
    fn x(&self) -> i32 { self.x }
    fn y(&self) -> i32 { self.y }
    fn w(&self) -> u32 { self.w }
    fn h(&self) -> u32 { self.h }
    fn pos(&self) -> (i32, i32) { (self.x, self.y) }
    fn size(&self) -> (u32, u32) { (self.w, self.h) }
    fn set_x(&mut self, x: i32) { self.x = x; }
    fn set_y(&mut self, y: i32) { self.y = y; }
    fn set_w(&mut self, w: u32) { self.w = w; }
    fn set_h(&mut self, h: u32) { self.h = h; }
    fn set_pos(&mut self, pos: (i32, i32)) { (self.x, self.y) = pos; }
    fn set_size(&mut self, size: (u32, u32)) { (self.w, self.h) = size; }
    fn dimensions(&self) -> Dimensions { self.clone() }
}

