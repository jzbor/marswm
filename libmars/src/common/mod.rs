use std::cmp;


pub mod error;
pub mod x11;


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

    fn center(&self) -> (i32, i32) {
        let center_x = self.x() + (self.w() / 2) as i32;
        let center_y = self.y() + (self.h() / 2) as i32;
        return (center_x, center_y);
    }

    fn bottom(&self) -> i32 {
        return self.y() + self.h() as i32;
    }

    fn right(&self) -> i32 {
        return self.x() + self.w() as i32;
    }
}


#[derive(Copy,Clone,PartialEq,Eq,Debug)]
pub struct Dimensions {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

#[derive(Clone,PartialEq)]
pub struct MonitorConfig {
    name: String,
    dims: Dimensions,
    win_area: Dimensions,
}


impl Dimensions {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Dimensions {
        return Dimensions {x, y, w, h};
    }

    pub fn as_tuple(&self) -> (i32, i32, u32, u32) {
        return (self.x, self.y, self.w, self.h);
    }
}

impl MonitorConfig {
    pub fn new(name: String, dimensions: Dimensions, win_area: Dimensions) -> MonitorConfig {
        return MonitorConfig { name, dims: dimensions, win_area };
    }

    pub fn add_inset_top(&mut self, inset: u32) {
        self.win_area.set_y(cmp::max(self.win_area.y(), self.win_area.y() + inset as i32));
        self.win_area.set_h(cmp::min(self.win_area.h(), self.win_area.h() - inset));
    }

    pub fn add_inset_bottom(&mut self, inset: u32) {
        self.win_area.set_h(cmp::min(self.win_area.h(), self.win_area.h() - inset));
    }

    pub fn contains_point(&self, point: (i32, i32)) -> bool {
        return point.0 >= self.dims.x()
            && point.0 < self.dims.x() + self.dims.w() as i32
            && point.1 >= self.dims.y()
            && point.1 < self.dims.y() + self.dims.h() as i32;
    }

    pub fn dimensions(&self) -> Dimensions {
        return self.dims;
    }

    pub fn name(&self) -> &str {
        return &self.name;
    }

    pub fn remove_insets(&mut self) {
        self.win_area = self.dims;
    }

    pub fn window_area(&self) -> Dimensions {
        return self.win_area;
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

