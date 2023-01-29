use crate::common::error::*;


pub mod x11;


pub enum SettingMode {
    Set, Unset, Toggle
}

pub trait WMController<W> {
    fn activate_window(&self, window: W) -> Result<()>;
    fn close_window(&self, window: W) -> Result<()>;
    fn current_workspace(&self) -> Result<u32>;
    fn fullscreen_window(&self, window: W, mode: SettingMode) -> Result<()>;
    fn get_workspace(&self, window: W) -> Result<u32>;
    fn pin_window(&self, window: W, mode: SettingMode) -> Result<()>;
    fn send_window_to_workspace(&self, window: W, workspace: u32) -> Result<()>;
    fn switch_workspace(&self, workspace: u32) -> Result<()>;
    fn window_is_fullscreen(&self, window: W) -> Result<bool>;
    fn window_is_pinned(&self, window: W) -> Result<bool>;
}
