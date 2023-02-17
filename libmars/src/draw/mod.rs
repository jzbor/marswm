//! A very basic toolkit for creating simple applications and drawing on windows.
//!
//! Currently only X11 is supported.
use crate::common::error::*;

pub mod x11;

pub trait Canvas {
    fn draw_line(&mut self, pt1: (i32, i32), pt2: (i32, i32));
    fn draw_line_with(&mut self, pt1: (i32, i32), pt2: (i32, i32), color: u64, line_width: u32);
    fn draw_rectangle(&mut self, x: i32, y: i32, width: u32, height: u32);
    fn draw_rectangle_with(&mut self, x: i32, y: i32, width: u32, height: u32, color: u64, line_width: u32);
    fn draw_text(&mut self, x: i32, y: i32, height: u32, text: &str) -> Result<(u32, u32)>;
    fn fill_rectangle(&mut self, x: i32, y: i32, width: u32, height: u32);
    fn fill_rectangle_with(&mut self, x: i32, y: i32, width: u32, height: u32, color: u64);
    fn flush(&self);
    fn match_resize(&mut self);
    fn set_background(&mut self, color: u64) -> Result<()>;
    fn set_font(&mut self, font_name: &str) -> Result<()>;
    fn set_foreground(&mut self, color: u64) -> Result<()>;
    fn set_line_width(&mut self, line_width: u32);
    fn text_size(&self, text: &str) -> Result<(u32, u32)>;
}

