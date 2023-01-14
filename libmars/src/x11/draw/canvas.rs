use std::ffi::*;
use std::mem::MaybeUninit;
use x11::xft;
use x11::xlib;
use x11::xrender;

use crate::x11::window::X11Window;
use crate::Dimensioned;

#[derive(Clone, Copy)]
struct CanvasStyle {
    line_width: u32,
    line_style: i32,
    cap_style: i32,
    join_style: i32,
    fill_style: i32,
    xft_color: xft::XftColor,
    xfont: Option<*mut xft::XftFont>,
}

pub struct Canvas {
    display: *mut xlib::Display,
    screen: i32,
    window: xlib::Window,
    pixbuffer: xlib::Drawable,
    gc: xlib::GC,
    style: CanvasStyle,
}


impl Canvas {
    pub fn new_for_window(display: *mut xlib::Display, window: xlib::Window) -> Result<Canvas, String> {
        let screen = unsafe { xlib::XDefaultScreen(display) };
        let style = CanvasStyle::default();
        let pixbuffer = Self::create_pixmap(display, screen, window)?;
        let gc = Self::create_default_gc(display, window, false)
            .map_err(|e| unsafe { xlib::XFreePixmap(display, pixbuffer); e })?;

        return Ok( Canvas {
            display, screen, window, pixbuffer, gc, style,
        } );
    }

    fn create_default_gc(display: *mut xlib::Display, window: xlib::Window, reverse_video: bool) -> Result<xlib::GC, String> {
        let style = CanvasStyle::default();
        unsafe {
            let screen = xlib::XDefaultScreen(display);
            let value_mask = 0;
            let mut values: xlib::XGCValues = MaybeUninit::zeroed().assume_init();

            let gc = xlib::XCreateGC(display, window, value_mask, &mut values);
            if gc.is_null() {
                return Err("XCreateGC() failed".to_owned());
            }

            // allocate foreground and background colors
            if reverse_video {
                xlib::XSetForeground(display, gc, xlib::XWhitePixel(display, screen));
                xlib::XSetBackground(display, gc, xlib::XBlackPixel(display, screen));
            } else {
                xlib::XSetForeground(display, gc, xlib::XBlackPixel(display, screen));
                xlib::XSetBackground(display, gc, xlib::XWhitePixel(display, screen));
            }

            // set drawing style
            xlib::XSetLineAttributes(display, gc, style.line_width, style.line_style, style.cap_style, style.join_style);
            xlib::XSetFillStyle(display, gc, style.fill_style);

            return Ok(gc);
        }
    }

    fn create_pixmap(display: *mut xlib::Display, screen: i32, window: xlib::Window) -> Result<xlib::Drawable, String> {
        unsafe {
            let window_dims = window.x11_dimensions(display)?;
            let depth = xlib::XDefaultDepth(display, screen);
            let root = xlib::XDefaultRootWindow(display);
            return Ok(xlib::XCreatePixmap(display, root, window_dims.w(), window_dims.h(), depth as u32));
        }
    }

    pub fn draw_line(&self, pt1: (i32, i32), pt2: (i32, i32)) {
        unsafe {
            xlib::XDrawLine(self.display, self.pixbuffer, self.gc,
                            pt1.0, pt1.1, pt2.0, pt2.1);
        }
    }

    pub fn draw_line_with(&mut self, pt1: (i32, i32), pt2: (i32, i32), color: &str, line_width: u32) {
        if let Ok(xft_color) = self.alloc_color(color) {
            let saved_color = self.style.xft_color;
            let saved_line_width = self.style.line_width;

            // configure new style
            self.style.xft_color = xft_color;
            self.style.line_width = line_width;
            self.gc_apply_foreground();
            self.gc_apply_line_attrib();

            self.draw_line(pt1, pt2);

            // restore previous style
            self.style.line_width = saved_line_width;
            self.style.xft_color = saved_color;
            self.gc_apply_foreground();
            self.gc_apply_line_attrib();
        } else {
            self.draw_line(pt1, pt2);
        }
    }


    pub fn draw_rectangle(&self, x: i32, y: i32, width: u32, height: u32) {
        unsafe {
            xlib::XDrawRectangle(self.display, self.pixbuffer, self.gc,
                            x, y, width, height);
        }
    }

    pub fn draw_rectangle_with(&mut self, x: i32, y: i32, width: u32, height: u32, color: &str, line_width: u32) {
        if let Ok(xft_color) = self.alloc_color(color) {
            let saved_color = self.style.xft_color;
            let saved_line_width = self.style.line_width;

            // configure new style
            self.style.xft_color = xft_color;
            self.style.line_width = line_width;
            self.gc_apply_foreground();
            self.gc_apply_line_attrib();

            self.draw_rectangle(x, y, width, height);

            // restore previous style
            self.style.line_width = saved_line_width;
            self.style.xft_color = saved_color;
            self.gc_apply_foreground();
            self.gc_apply_line_attrib();
        } else {
            self.draw_rectangle(x, y, width, height);
        }
    }

    pub fn text_size(&self, text: &str) -> Result<(u32, u32), String> {
        if let Some(xfont) = self.style.xfont {
            let strlen = match text.len().try_into() {
                Ok(i) => i,
                Err(_) => return Err("unable to convert text size to i32".to_owned()),
            };

            unsafe {
                let mut extents: MaybeUninit<xrender::XGlyphInfo> = MaybeUninit::uninit();
                xft::XftTextExtentsUtf8(self.display, xfont, text.as_ptr(), strlen, extents.as_mut_ptr());
                let extents = extents.assume_init();
                let height = (*xfont).ascent + (*xfont).descent;

                return Ok((extents.width.into(), height as u32));
            }
        } else {
            return Err("unable to get text size - no font specified".to_owned());
        }
    }

    pub fn draw_text(&mut self, x: i32, y: i32, height: u32, text: &str) -> Result<(u32, u32), String> {
        let font = match self.style.xfont {
            Some(font) => font,
            None => return Err("no font specified".to_owned()),
        };
        // make string length c compatible
        let strlen = match text.len().try_into() {
            Ok(i) => i,
            Err(_) => return Err("unable to convert text size to i32".to_owned()),
        };

        // get font dimensions
        let ascent = unsafe { (*font).ascent };
        let descent = unsafe { (*font).descent };
        let th = ascent + descent;

        let y = y + th + (height as i32 - th as i32) / 2 - descent;

        unsafe {
            let visual = xlib::XDefaultVisual(self.display, self.screen);
            let colormap = xlib::XDefaultColormap(self.display, self.screen);
            let xft_draw = xft::XftDrawCreate(self.display, self.pixbuffer, visual, colormap);

            xft::XftDrawStringUtf8(xft_draw, (&self.style.xft_color) as *const xft::XftColor,
                                   font, x, y, text.as_ptr(), strlen);

            xft::XftDrawDestroy(xft_draw);
        }

        return self.text_size(text);
    }

    pub fn fill_rectangle(&self, x: i32, y: i32, width: u32, height: u32) {
        unsafe {
            xlib::XFillRectangle(self.display, self.pixbuffer, self.gc,
                            x, y, width, height);
        }
    }

    pub fn fill_rectangle_with(&mut self, x: i32, y: i32, width: u32, height: u32, color: &str) {
        if let Ok(xft_color) = self.alloc_color(color) {
            let saved_color = self.style.xft_color;

            // configure new style
            self.style.xft_color = xft_color;
            self.gc_apply_foreground();

            self.fill_rectangle(x, y, width, height);

            // restore previous style
            self.style.xft_color = saved_color;
            self.gc_apply_foreground();
        } else {
            self.fill_rectangle(x, y, width, height)
        }
    }

    pub fn flush(&self) {
        unsafe {
            if let Ok(d) = self.window.x11_dimensions(self.display) {
                xlib::XCopyArea(self.display, self.pixbuffer, self.window, self.gc, 0, 0, d.w(), d.h(), 0, 0);
            }
            xlib::XFlush(self.display);
        }
    }

    fn gc_apply_foreground(&self) {
        unsafe {
            xlib::XSetForeground(self.display, self.gc, self.style.xft_color.pixel);
        }
    }

    fn gc_apply_line_attrib(&self) {
        unsafe {
            xlib::XSetLineAttributes(self.display, self.gc, self.style.line_width,
                                     self.style.line_style, self.style.cap_style, self.style.join_style);
        }
    }

    fn alloc_color(&self, color_name: &str) -> Result<xft::XftColor, String> {
        // allocate color for xft
        let color_cstring = match CString::new(color_name) {
            Ok(cstring) => cstring,
            Err(_) => return Err(format!("unable to convert color '{}' to C compatible string", color_name))
        };
        let mut xft_color: MaybeUninit<xft::XftColor> = MaybeUninit::uninit();
        unsafe {
            let visual = xlib::XDefaultVisual(self.display, self.screen);
            let colormap = xlib::XDefaultColormap(self.display, self.screen);
            if xft::XftColorAllocName(self.display, visual, colormap, color_cstring.as_ptr(), xft_color.as_mut_ptr()) == 0 {
                return Err(format!("unable to allocate color '{}'", color_name));
            } else {
                return Ok(xft_color.assume_init());
            }
        }
    }

    pub fn match_resize(&mut self) {
        if let Ok(new_pixbuffer) = Self::create_pixmap(self.display, self.screen, self.window) {
            unsafe {
                xlib::XFreePixmap(self.display, self.pixbuffer);
            }
            self.pixbuffer = new_pixbuffer;
        }
    }

    pub fn set_font(&mut self, font_name: &str) -> Result<(), String> {
        unsafe {
            // deallocate previous font if necessary
            if let Some(prev_font) = self.style.xfont.take() {
                xft::XftFontClose(self.display, prev_font);
            }

            // allocate font
            let xfont = xft::XftFontOpenName(self.display, self.screen,font_name.as_ptr() as *const i8);
            if xfont.is_null() {
                return Err("unable to allocate font".to_owned());
            } else {
                self.style.xfont = Some(xfont);
            }
        }

        return Ok(());
    }

    pub fn set_foreground(&mut self, color: &str) -> Result<(), String> {
        self.style.xft_color = self.alloc_color(color)?;
        self.gc_apply_foreground();
        return Ok(());
    }

    pub fn set_foreground_by_name(&self, name: &str) -> Result<(), String> {
        unsafe {
            let screen_colormap = xlib::XDefaultColormap(self.display, self.screen);
            let mut color: MaybeUninit<xlib::XColor> = MaybeUninit::uninit();
            let rc = xlib::XAllocNamedColor(self.display, screen_colormap, name.as_ptr() as *const i8,
                                            color.as_mut_ptr(), color.as_mut_ptr());
            if rc == 0 {
                return Err(format!("unable to allocate named color '{}'", name));
            }
        }

        self.gc_apply_foreground();
        return Ok(());
    }

    pub fn set_line_width(&mut self, line_width: u32) {
        self.style.line_width = line_width;
        self.gc_apply_line_attrib();
    }

    pub fn window(&self) -> xlib::Window {
        return self.window;
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        unsafe {
            xlib::XFreeGC(self.display, self.gc);
            xlib::XFreePixmap(self.display, self.pixbuffer);
        }
    }
}

impl Default for CanvasStyle {
    fn default() -> Self {
        let default_xft_color = xft::XftColor {
            pixel: 0,
            color: xrender::XRenderColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: c_ushort::MAX,
            },
        };

        CanvasStyle {
            line_width: 10,
            line_style: xlib::LineSolid,
            cap_style: xlib::CapRound,
            join_style: xlib::JoinMiter,
            fill_style: xlib::FillSolid,
            xft_color: default_xft_color,
            xfont: None,
        }
    }
}
