//! This module provides functions to control a running WM from an external program.

use crate::common::error::*;


pub enum SettingMode {
    Set, Unset, Toggle
}

pub trait WMController<W> {
    fn activate_window(&self, window: W) -> Result<()>;
    fn center_window(&self, window: W) -> Result<()>;
    fn close_window(&self, window: W) -> Result<()>;
    fn count_workspaces(&self) -> Result<u32>;
    fn current_workspace(&self) -> Result<u32>;
    fn fullscreen_window(&self, window: W, mode: SettingMode) -> Result<()>;
    fn get_active_window(&self) -> Result<W>;
    fn get_workspace(&self, window: W) -> Result<u32>;
    fn pin_window(&self, window: W, mode: SettingMode) -> Result<()>;
    fn send_window_to_workspace(&self, window: W, workspace: u32) -> Result<()>;
    fn set_status(&self, status: String) -> Result<()>;
    fn switch_workspace(&self, workspace: u32) -> Result<()>;
    fn tile_window(&self, window: W, mode: SettingMode) -> Result<()>;
    fn window_is_fullscreen(&self, window: W) -> Result<bool>;
    fn window_is_pinned(&self, window: W) -> Result<bool>;
    fn window_is_tiled(&self, window: W) -> Result<bool>;
    fn workspaces(&self) -> Result<Vec<String>>;
}
