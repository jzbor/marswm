use x11::xlib;
use std::cmp;

use crate::Dimensions;
use crate::Dimensioned;
use crate::x11::window::X11Window;
use crate::x11::draw::canvas::Canvas;

pub trait WidgetEventHandler {
    fn handle_action_event(&self, event: WidgetEvent, already_handled: bool) -> bool;
}

pub trait Widget {
    fn move_to(&mut self, x: i32, y: i32);
    fn redraw(&mut self);
    fn register_event_handler(&mut self, event_handler: Box<dyn WidgetEventHandler>);
    fn report_event(&mut self, event: WidgetEvent);
    fn size(&self) -> (u32, u32);
    fn wid(&self) -> xlib::Window;
}

#[derive(Copy,Clone,Debug)]
pub enum WidgetEvent {
    ButtonPressed(u32),
    Redraw(),
}

pub struct FlowLayoutWidget<W: Widget> {
    display: *mut xlib::Display,
    children: Vec<W>,
    window: xlib::Window,
    canvas: Canvas,
    event_handlers: Vec<Box<dyn WidgetEventHandler>>,
    width: u32,
    height: u32,
    hpad: u32,
    vpad: u32,
}

pub struct TextWidget {
    display: *mut xlib::Display,
    label: String,
    window: xlib::Window,
    canvas: Canvas,
    event_handlers: Vec<Box<dyn WidgetEventHandler>>,
    width: u32,
    height: u32,
    hpad: u32,
    vpad: u32,
    fg_color: u64,
    bg_color: u64,
}

impl<W: Widget> FlowLayoutWidget<W> {
    pub fn new(display: *mut xlib::Display, parent: xlib::Window, x: i32, y: i32, hpad: u32, vpad: u32,
               children: Vec<W>, bg_color: u64) -> Result<FlowLayoutWidget<W>, String> {
        let outer_dimensions = Dimensions::new(x, y, 10, 10);
        let window = create_widget_window(display, parent, outer_dimensions)?;
        let mut canvas = Canvas::new_for_window(display, window)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;

        canvas.set_foreground(bg_color)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;

        let mut widget = FlowLayoutWidget {
            display,
            children,
            window, canvas,
            event_handlers: Vec::new(),
            width: 10, height: 10,
            hpad, vpad,
        };

        widget.rearrange();

        return Ok(widget);
    }

    fn arrange_children(&mut self) {
        let mut x = self.hpad as i32;
        for child in &mut self.children {
            let (cw, ch) = child.size();
            let y = (self.height as i32 - ch as i32) / 2;
            child.move_to(x, y);
            x += (cw + self.hpad) as i32;
        }
    }

    pub fn children(&self) -> Box<dyn Iterator<Item = &W> + '_> {
        return Box::new(self.children.iter());
    }

    pub fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut W> + '_> {
        return Box::new(self.children.iter_mut());
    }

    pub fn child(&self, index: usize) -> Option<&W> {
        return self.children.get(index);
    }

    pub fn child_mut(&mut self, index: usize) -> Option<&mut W> {
        return self.children.get_mut(index);
    }

    pub fn count_children(&self) -> usize {
        return self.children.len();
    }

    pub fn push(&mut self, widget: W) {
        self.children.push(widget);
        self.rearrange();
    }

    pub fn rearrange(&mut self) {
        self.resize_to_content();
        self.arrange_children();
    }

    pub fn remove(&mut self, index: usize) {
        self.children.remove(index);
    }

    fn resize_to_content(&mut self) {
        let mut w = self.hpad;
        let mut h = 0;

        for child in &self.children {
            let (cw, ch) = child.size();
            w += cw + self.hpad;
            h = cmp::max(h, ch);
        }
        h += 2 * self.vpad;

        self.width = w;
        self.height = h;

        unsafe {
            xlib::XResizeWindow(self.display, self.window, self.width, self.height);
        }

        self.canvas.match_resize();
    }

    pub fn truncate_children(&mut self, len: usize) {
        self.children.truncate(len);
    }
}

impl TextWidget {
    pub fn new(display: *mut xlib::Display, parent: xlib::Window, x: i32, y: i32, hpad: u32, vpad: u32,
               label: String, font: &str, fg_color: u64, bg_color: u64) -> Result<TextWidget, String> {

        let outer_dimensions = Dimensions::new(x, y, 10, 10);
        let window = create_widget_window(display, parent, outer_dimensions)?;
        let mut canvas = Canvas::new_for_window(display, window)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;

        canvas.set_foreground(fg_color)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;
        canvas.set_font(&font)
            .map_err(|err| unsafe { xlib::XDestroyWindow(display, window); err })?;

        let mut widget = TextWidget {
            display,
            label,
            window, canvas,
            event_handlers: Vec::new(),
            width: 10, height: 10,
            hpad, vpad,
            fg_color, bg_color,
        };

        widget.resize_to_content();

        return Ok(widget);
    }

    fn resize_to_content(&mut self) {
        let (tw, th) = match self.canvas.text_size(&self.label) {
            Ok(size) => size,
            Err(_) => return,
        };

        self.width = tw + 2 * self.hpad;
        self.height = th + 2 * self.vpad;

        unsafe {
            xlib::XResizeWindow(self.display, self.window, self.width, self.height);
        }

        self.canvas.match_resize();
    }

    pub fn set_foreground(&mut self, color: u64) -> Result<(), String> {
        self.canvas.set_foreground(color)?;
        self.fg_color = color;
        self.redraw();
        return Ok(());
    }

    pub fn set_background(&mut self, color: u64) -> Result<(), String> {
        // TODO check color before assigning
        self.bg_color = color;
        self.redraw();
        return Ok(());
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.resize_to_content();
        self.redraw();
    }
}

impl<W: Widget> Widget for FlowLayoutWidget<W> {
    fn move_to(&mut self, x: i32, y: i32) {
        unsafe {
            xlib::XMoveWindow(self.display, self.window, x, y);
        }
        self.redraw();
    }

    fn redraw(&mut self) {
        self.canvas.fill_rectangle(0, 0, self.width, self.height);
        self.canvas.flush();
        for child in &mut self.children {
            child.redraw();
        }
    }

    fn register_event_handler(&mut self, event_handler: Box<dyn WidgetEventHandler>) {
        self.event_handlers.push(event_handler);
    }

    fn report_event(&mut self, event: WidgetEvent) {
        let _handled = self.event_handlers.iter()
            .fold(false, |already_handled, handler| handler.handle_action_event(event, already_handled));
    }

    fn size(&self) -> (u32, u32) {
        return (self.width, self.height);
    }

    fn wid(&self) -> xlib::Window {
        return self.window;
    }
}

impl Widget for TextWidget {
    fn move_to(&mut self, x: i32, y: i32) {
        unsafe {
            xlib::XMoveWindow(self.display, self.window, x, y);
        }
        self.redraw();
    }

    fn redraw(&mut self) {
        self.canvas.fill_rectangle_with(0, 0, self.width, self.height, self.bg_color);
        let _ = self.canvas.draw_text(self.hpad as i32, self.vpad as i32, self.height - 2*self.vpad, &self.label);
        self.canvas.flush();
    }

    fn register_event_handler(&mut self, event_handler: Box<dyn WidgetEventHandler>) {
        self.event_handlers.push(event_handler);
    }

    fn report_event(&mut self, event: WidgetEvent) {
        let _handled = self.event_handlers.iter()
            .fold(false, |already_handled, handler| handler.handle_action_event(event, already_handled));
    }

    fn size(&self) -> (u32, u32) {
        return (self.width, self.height);
    }

    fn wid(&self) -> xlib::Window {
        return self.window;
    }
}

impl<W: Widget> Drop for FlowLayoutWidget<W> {
    fn drop(&mut self) {
        self.window.x11_destroy(self.display);
    }
}

impl Drop for TextWidget {
    fn drop(&mut self) {
        self.window.x11_destroy(self.display);
    }
}

pub fn create_widget_window(display: *mut xlib::Display, parent: xlib::Window, dimensions: Dimensions) -> Result<xlib::Window, String> {
    unsafe {
        let screen = xlib::XDefaultScreen(display);
        let border_width = 0;

        let win = xlib::XCreateSimpleWindow(display, xlib::XDefaultRootWindow(display),
                                       dimensions.x(), dimensions.y(), dimensions.w(), dimensions.h(), border_width,
                                       xlib::XBlackPixel(display, screen),
                                       xlib::XWhitePixel(display, screen));

        // subscribe to StructureNotifyMask for MapNotify events
        // subscribe to SubstructureNotifyMask for widgets reparenting other widgets or windows
        // subscribe to ExposureMask for Expose events
        let mask = xlib::StructureNotifyMask | xlib::SubstructureNotifyMask | xlib::ExposureMask | xlib::ButtonPressMask;
        xlib::XSelectInput(display, win, mask);

        // reparent window
        let status = xlib::XReparentWindow(display, win, parent, dimensions.x(), dimensions.y());
        if status == 0 {
            xlib::XDestroyWindow(display, win);
            return Err("unable to reparent widget".to_owned());
        }

        // make window visible on screen
        xlib::XMapWindow(display, win);

        // sync requests with x11 server
        xlib::XFlush(display);

        return Ok(win);
    }
}

pub fn distribute_widget_event<'a, I: Iterator<Item=&'a mut dyn Widget>>(widgets: &mut I, xevent: xlib::XEvent) {
    unsafe {
        let widget_event = match xevent.get_type() {
            xlib::ButtonPress => {
                let button = xevent.button.button;
                WidgetEvent::ButtonPressed(button)
            },
            xlib::Expose => {
                if let Some(widget) = widgets.find(|w| w.wid() == xevent.any.window) {
                    widget.redraw();
                }
                return;
            },
            _ => return,
        };

        if let Some(widget) = widgets.find(|w| w.wid() == xevent.any.window) {
            widget.report_event(widget_event);
        }
    }
}
