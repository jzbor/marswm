use libmars::platforms::x11::draw::widget::*;
use libmars::utils::configuration::read_config_file;
use libmars::utils::configuration::read_file;
use serde::{Serialize, Deserialize};
use std::path;
use x11::xlib;

use crate::tray::*;


const CONFIG_NAME: &str = "marswm";
const CONFIG_FILE: &str = "marsbar.yaml";

const DEFAULT_FONT: &str = "serif";
const DEFAULT_TEXT_PADDING_HORZ: u32 = 5;
const DEFAULT_TEXT_PADDING_VERT: u32 = 0;


#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
#[serde(default)]
pub struct BarStyle {
    pub background: u64,
    pub expand_workspace_widgets: bool,
    pub height: u32,
    pub font: String,
    pub workspaces: ContainerWidgetStyle,
    pub title: TextWidgetStyle,
    pub status: ContainerWidgetStyle,
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
#[serde(default)]
pub struct TextWidgetStyle {
    pub foreground: u64,
    pub background: u64,
    pub padding_horz: u32,
    pub padding_vert: u32,
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Clone)]
pub struct ContainerWidgetStyle {
    pub foreground: u64,
    pub inner_background: u64,
    pub outer_background: u64,
    pub padding_horz: u32,
    pub padding_vert: u32,
    pub text_padding_horz: u32,
    pub text_padding_vert: u32,
    pub spacing: u32,
}

#[derive(Default,Serialize,Deserialize,PartialEq,Debug,Clone)]
#[serde(default)]
pub struct Configuration {
    pub status_cmd: Option<String>,
    pub action_cmd: Option<String>,
    pub style: BarStyle,
}


impl Default for BarStyle {
    fn default() -> Self {
        BarStyle {
            background: 0x262626,
            expand_workspace_widgets: false,
            height: 31,
            font: DEFAULT_FONT.to_owned(),
            workspaces: ContainerWidgetStyle::default_workspaces(),
            title: TextWidgetStyle::default(),
            status: ContainerWidgetStyle::default_status(),
        }
    }
}

impl Default for TextWidgetStyle {
    fn default() -> Self {
        TextWidgetStyle {
            foreground: 0xbcbcbc,
            background: 0x262626,
            padding_horz: DEFAULT_TEXT_PADDING_HORZ,
            padding_vert: DEFAULT_TEXT_PADDING_VERT
        }
    }
}

impl ContainerWidgetStyle {
    pub fn create_flow_layout_widget<W: Widget>(&self, display: *mut xlib::Display, parent: xlib::Window)
            -> Result<X11FlowLayoutWidget<W>, String> {
        X11FlowLayoutWidget::new(display, parent, 0, 0, self.padding_horz, self.padding_vert, self.spacing,
                                     Vec::new(), self.outer_background)
            .map_err(|e| e.to_string())
    }

    pub fn create_text_widget(&self, display: *mut xlib::Display, parent: xlib::Window, font: &str)
            -> Result<X11TextWidget, String> {
        X11TextWidget::new(display, parent, 0, 0, self.text_padding_horz, self.text_padding_vert,
                               "".to_string(), font, self.foreground, self.inner_background)
            .map_err(|e| e.to_string())
    }

    pub fn create_systray_widget(&self, display: *mut xlib::Display, parent: xlib::Window, parent_height: u32)
            -> Result<SystemTrayWidget, String> {
        SystemTrayWidget::new(display, parent, 0, 0, parent_height - 2 * self.spacing, self.padding_horz,
                                     self.padding_horz, self.padding_vert, self.inner_background)
            .map_err(|e| e.to_string())
    }

    fn default_status() -> Self {
        ContainerWidgetStyle {
            foreground: 0x262626,
            inner_background: 0xaf5f5f,
            outer_background: 0x262626,
            padding_horz: 4,
            padding_vert: 4,
            text_padding_horz: 5,
            text_padding_vert: 0,
            spacing: 4,
        }
    }

    fn default_workspaces() -> Self {
        ContainerWidgetStyle {
            foreground: 0x262626,
            inner_background: 0x5f87af,
            outer_background: 0x262626,
            padding_horz: 0,
            padding_vert: 0,
            text_padding_horz: 10,
            text_padding_vert: 4,
            spacing: 0,
        }
    }
}

impl TextWidgetStyle {
    pub fn create_text_widget(&self, display: *mut xlib::Display, parent: xlib::Window, font: &str)
            -> Result<X11TextWidget, String> {
        X11TextWidget::new(display, parent, 0, 0, self.padding_horz, self.padding_vert,
                               "".to_string(), font, self.foreground, self.background)
            .map_err(|e| e.to_string())
    }
}


pub fn read_config(overwrite_path: Option<path::PathBuf>) -> Configuration {
    let result = if let Some(path) = overwrite_path {
        read_file(&path)
    } else {
        read_config_file(CONFIG_NAME, CONFIG_FILE)
    };

    return match result {
        Ok(config) => config,
        Err(msg) => {
            println!("Unable to read configuration: {}", msg);
            Configuration::default()
        },
    };
}

