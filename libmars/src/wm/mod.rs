//! Window manager library with the goal to make implementing the window management itself easier.

use std::cell::RefCell;
use std::rc::Rc;

use crate::common::*;

pub mod x11;

pub trait WindowManager<B: Backend<A>, A> {
    fn active_client(&self) -> Option<Rc<RefCell<B::Client>>>;
    fn active_workspace(&self, backend: &mut B) -> u32;
    fn activate_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>);
    fn client_switches_monitor(&mut self, client_rc: Rc<RefCell<B::Client>>, monitor: u32);
    fn clients(&self) -> Box<dyn Iterator<Item = &Rc<RefCell<B::Client>>> + '_>;
    fn focus_client(&mut self, backend: &mut B, client_option: Option<Rc<RefCell<B::Client>>>);
    fn fullscreen_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, state: bool);
    fn handle_button(&mut self, backend: &mut B, modifiers: u32, button: u32, client_option: Option<Rc<RefCell<B::Client>>>);
    fn handle_key(&mut self, backend: &mut B, modifiers: u32, key: u32, client_option: Option<Rc<RefCell<B::Client>>>);
    fn manage(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, workspace_preference: Option<u32>);
    fn move_request(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, x: i32, y: i32) -> bool;
    fn move_to_workspace(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, workspace_idx: u32);
    fn resize_request(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, width: u32, height: u32) -> bool;
    fn set_client_pinned(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, state: bool);
    fn switch_workspace(&mut self, backend: &mut B, workspace_idx: u32);
    fn tile_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>, state: bool);
    fn toggle_fullscreen_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>);
    fn toggle_tile_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>);
    fn unfocus_client(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>);
    fn unmanage(&mut self, backend: &mut B, client_rc: Rc<RefCell<B::Client>>);
    fn update_monitor_config(&mut self, backend: &mut B, configs: Vec<MonitorConfig>);
}

pub trait Client<A>: Eq + Dimensioned {
    fn application(&self) -> String;
    fn attributes(&self) -> &A;
    fn attributes_mut(&mut self) -> &mut A;
    fn bind_button(&mut self, modifiers: u32, button: u32);
    fn bind_key(&mut self, modifiers: u32, key: u32);
    fn center_on_screen(&mut self, monitor_conf: &MonitorConfig);
    fn close(&self);
    fn dont_decorate(&self) -> bool;
    fn export_pinned(&self, state: bool, workspace_idx: Option<u32>);
    fn export_tiled(&self, state: bool);
    fn export_workspace(&self, workspace_idx: u32);
    fn frame_width(&self) -> (u32, u32, u32, u32);
    fn hide(&mut self);
    fn inner_bw(&self) -> u32;
    // dimensions excluding all borders
    fn inner_dimensions(&self) -> Dimensions;
    fn is_dialog(&self) -> bool;
    fn is_fullscreen(&self) -> bool;
    fn is_visible(&self) -> bool;
    fn move_resize(&mut self, x: i32, y: i32, width: u32, height: u32);
    fn name(&self) -> &str;
    fn outer_bw(&self) -> u32;
    fn raise(&self);
    fn set_frame_color(&mut self, color: u64);
    fn set_frame_width(&mut self, width: (u32, u32, u32, u32));
    fn set_fullscreen(&mut self, monitor_conf: &MonitorConfig);
    fn set_height(&mut self, height: u32);
    fn set_inner_bw(&mut self, bw: u32);
    fn set_inner_color(&mut self, color: u64);
    fn set_outer_bw(&mut self, bw: u32);
    fn set_outer_color(&mut self, color: u64);
    fn show(&mut self);
    fn title(&self) -> String;
    fn total_bw(&self) -> (u32, u32, u32, u32);
    fn unset_fullscreen(&mut self);
    fn warp_pointer_to_center(&self);
}

pub trait Backend<A> {
    /// Associated client type
    type Client: Client<A>;

    /// Make active window information available to clients
    fn export_active_window(&self, client_option: &Option<Rc<RefCell<Self::Client>>>);

    /// Make client list information available to clients
    fn export_client_list(&self, clients: Vec<&Rc<RefCell<Self::Client>>>, clients_stacked: Vec<&Rc<RefCell<Self::Client>>>);

    /// Make currently active workspace available to clients
    fn export_current_workspace(&self, workspace_idx: u32);

    /// Make information about workspaces available to clients
    fn export_workspaces(&self, workspaces: Vec<(String, Dimensions, Dimensions)>);

    /// Get monitor configuration
    fn get_monitor_config(&self) -> Vec<MonitorConfig>;

    /// Handle windows existing before initialization
    fn handle_existing_windows(&mut self, wm: &mut dyn WindowManager<Self, A>);

    /// Move client with mouse
    fn mouse_move(&mut self, wm: &mut dyn WindowManager<Self, A>,
                  client_rc: Rc<RefCell<Self::Client>>, button: u32);

    /// Resize client with mouse
    fn mouse_resize(&mut self, wm: &mut dyn WindowManager<Self, A>,
                    client_rc: Rc<RefCell<Self::Client>>, button: u32);

    fn point_to_monitor(&self, point: (i32, i32)) -> Option<u32>;

    /// Get position of pointer on screen
    fn pointer_pos(&self) -> (i32, i32);

    /// Run window manager event loop
    fn run(self, wm: &mut (dyn WindowManager<Self, A>));

    /// Set client that receives mouse and keyboard inputs
    fn set_input_focus(&self, client_rc: Rc<RefCell<Self::Client>>);

    fn warp_pointer(&self, x: i32, y: i32);

    fn shutdown(&mut self);
}
