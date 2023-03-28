//! The [Canvas] provides a nice wrapper for drawing onto a window

extern crate x11;

use std::ffi::*;
use std::mem::MaybeUninit;
use x11::xft;
use x11::xlib;
use x11::xrender;

use crate::common::*;
use crate::common::x11::window::*;
use crate::draw::*;

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

pub struct X11Canvas {
    display: *mut xlib::Display,
    screen: i32,
    window: xlib::Window,
    pixbuffer: xlib::Drawable,
    gc: xlib::GC,
    style: CanvasStyle,
}


impl X11Canvas {
    pub fn new_for_window(display: *mut xlib::Display, window: xlib::Window) -> Result<X11Canvas> {
        let screen = unsafe { xlib::XDefaultScreen(display) };
        let style = CanvasStyle::default();
        let pixbuffer = Self::create_pixmap(display, screen, window)?;
        let gc = Self::create_default_gc(display, window, false)
            .map_err(|e| unsafe { xlib::XFreePixmap(display, pixbuffer); e })?;

        Ok( X11Canvas {
            display, screen, window, pixbuffer, gc, style,
        } )
    }

    fn create_default_gc(display: *mut xlib::Display, window: xlib::Window, reverse_video: bool) -> Result<xlib::GC> {
        let style = CanvasStyle::default();
        unsafe {
            let screen = xlib::XDefaultScreen(display);
            let value_mask = 0;
            let mut values: xlib::XGCValues = MaybeUninit::zeroed().assume_init();

            let gc = xlib::XCreateGC(display, window, value_mask, &mut values);
            if gc.is_null() {
                return Err(MarsError::failed_request(stringify!(xlib::XCreateGC)));
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

            Ok(gc)
        }
    }

    fn create_pixmap(display: *mut xlib::Display, screen: i32, window: xlib::Window) -> Result<xlib::Drawable> {
        unsafe {
            let window_dims = window.x11_dimensions(display)?;
            let depth = xlib::XDefaultDepth(display, screen);
            let root = xlib::XDefaultRootWindow(display);
            Ok(xlib::XCreatePixmap(display, root, window_dims.w(), window_dims.h(), depth as u32))
        }
    }

    pub fn expose_loop(&mut self, display: *mut xlib::Display, draw: fn(&mut Self)) {
        loop {
            let mut event: MaybeUninit<xlib::XEvent> = MaybeUninit::uninit();
            unsafe {
                xlib::XNextEvent(display, event.as_mut_ptr());
                let event = event.assume_init();
                if event.get_type() == xlib::Expose
                        && event.expose.window == self.window() {
                    draw(self);
                    self.flush();
                }
            }
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

    fn alloc_color(&self, color: u64) -> Result<xft::XftColor> {
        let xr_color = {
            xrender::XRenderColor {
                red: ((color >> 16) & 0xff) as u16 * 0xff,
                green: ((color >> 8) & 0xff) as u16 * 0xff,
                blue: (color & 0xff) as u16 * 0xff,
                alpha: c_ushort::MAX,
            }
        };

        // allocate color for xft
        let mut xft_color: MaybeUninit<xft::XftColor> = MaybeUninit::uninit();
        unsafe {
            let visual = xlib::XDefaultVisual(self.display, self.screen);
            let colormap = xlib::XDefaultColormap(self.display, self.screen);
            if xft::XftColorAllocValue(self.display, visual, colormap, &xr_color, xft_color.as_mut_ptr()) == 0 {
                Err(MarsError::failed_request(stringify!(xft::XftColorAllocValue)))
            } else {
                Ok(xft_color.assume_init())
            }
        }
    }

    pub fn window(&self) -> xlib::Window {
        self.window
    }
}

impl Canvas for X11Canvas {
    fn draw_line(&mut self, pt1: (i32, i32), pt2: (i32, i32)) {
        unsafe {
            xlib::XDrawLine(self.display, self.pixbuffer, self.gc,
                            pt1.0, pt1.1, pt2.0, pt2.1);
        }
    }

    fn draw_line_with(&mut self, pt1: (i32, i32), pt2: (i32, i32), color: u64, line_width: u32) {
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

    fn draw_rectangle(&mut self, x: i32, y: i32, width: u32, height: u32) {
        unsafe {
            xlib::XDrawRectangle(self.display, self.pixbuffer, self.gc,
                            x, y, width, height);
        }
    }

    fn draw_rectangle_with(&mut self, x: i32, y: i32, width: u32, height: u32, color: u64, line_width: u32) {
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

    fn text_size(&self, text: &str) -> Result<(u32, u32)> {
        if let Some(xfont) = self.style.xfont {
            let strlen = match text.len().try_into() {
                Ok(i) => i,
                Err(_) => return Err(MarsError::failed_conversion(text.len(), stringify!(usize), stringify!(i32))),
            };

            unsafe {
                let mut extents: MaybeUninit<xrender::XGlyphInfo> = MaybeUninit::uninit();
                xft::XftTextExtentsUtf8(self.display, xfont, text.as_ptr(), strlen, extents.as_mut_ptr());
                let extents = extents.assume_init();
                let height = (*xfont).ascent + (*xfont).descent;

                Ok((extents.width.into(), height as u32))
            }
        } else {
            Err(MarsError::invalid_input("unable to get text size - no font specified"))
        }
    }

    fn draw_text(&mut self, x: i32, y: i32, height: u32, text: &str) -> Result<(u32, u32)> {
        let font = match self.style.xfont {
            Some(font) => font,
            None => return Err(MarsError::invalid_input("no font specified")),
        };
        // make string length c compatible
        let strlen = match text.len().try_into() {
            Ok(i) => i,
            Err(_) => return Err(MarsError::failed_conversion(text.len(), stringify!(usize), stringify!(i32))),
        };

        // get font dimensions
        let ascent = unsafe { (*font).ascent };
        let descent = unsafe { (*font).descent };
        let th = ascent + descent;

        let y = y + th + (height as i32 - th) / 2 - descent;

        unsafe {
            let visual = xlib::XDefaultVisual(self.display, self.screen);
            let colormap = xlib::XDefaultColormap(self.display, self.screen);
            let xft_draw = xft::XftDrawCreate(self.display, self.pixbuffer, visual, colormap);

            xft::XftDrawStringUtf8(xft_draw, (&self.style.xft_color) as *const xft::XftColor,
                                   font, x, y, text.as_ptr(), strlen);

            xft::XftDrawDestroy(xft_draw);
        }

        self.text_size(text)
    }

    fn fill_rectangle(&mut self, x: i32, y: i32, width: u32, height: u32) {
        unsafe {
            xlib::XFillRectangle(self.display, self.pixbuffer, self.gc,
                            x, y, width, height);
        }
    }

    fn fill_rectangle_with(&mut self, x: i32, y: i32, width: u32, height: u32, color: u64) {
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

    fn flush(&self) {
        unsafe {
            if let Ok(d) = self.window.x11_dimensions(self.display) {
                xlib::XCopyArea(self.display, self.pixbuffer, self.window, self.gc, 0, 0, d.w(), d.h(), 0, 0);
            }
            xlib::XFlush(self.display);
        }
    }

    fn match_resize(&mut self) {
        if let Ok(new_pixbuffer) = Self::create_pixmap(self.display, self.screen, self.window) {
            unsafe {
                xlib::XFreePixmap(self.display, self.pixbuffer);
            }
            self.pixbuffer = new_pixbuffer;
        }
    }

    fn set_background(&mut self, color: u64) -> Result<()> {
        let xft_color = self.alloc_color(color)?;
        unsafe {
            // set background for associated window
            let mut swa: xlib::XSetWindowAttributes = MaybeUninit::zeroed().assume_init();
            swa.background_pixel = color;
			xlib::XChangeWindowAttributes(self.display, self.window,  xlib::CWBackPixel, &mut swa);

            // set background for gc
            xlib::XSetBackground(self.display, self.gc, xft_color.pixel);
        }
        Ok(())
    }

    fn set_font(&mut self, font_name: &str) -> Result<()> {
        unsafe {
            // deallocate previous font if necessary
            if let Some(prev_font) = self.style.xfont.take() {
                xft::XftFontClose(self.display, prev_font);
            }

            // allocate font
            let xfont = xft::XftFontOpenName(self.display, self.screen,font_name.as_ptr() as *const i8);
            if xfont.is_null() {
                return Err(MarsError::failed_request(stringify!(xft::XftFontOpenName)));
            } else {
                self.style.xfont = Some(xfont);
            }
        }

        Ok(())
    }

    fn set_foreground(&mut self, color: u64) -> Result<()> {
        self.style.xft_color = self.alloc_color(color)?;
        self.gc_apply_foreground();
        Ok(())
    }

    fn set_line_width(&mut self, line_width: u32) {
        self.style.line_width = line_width;
        self.gc_apply_line_attrib();
    }
}

impl Drop for X11Canvas {
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
