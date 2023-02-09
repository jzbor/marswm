use serde::{Serialize, Deserialize};
use libmars::utils::configuration::*;

use crate::bindings::*;
use crate::layouts::LayoutType;
use crate::layouts::StackMode;
use crate::layouts::StackPosition;
use crate::rules::*;

const CONFIG_DIR: &str = "marswm";
const CONFIG_FILE: &str = "marswm.yaml";
const KEYBINDINGS_FILE: &str = "keybindings.yaml";
const KEYBINDINGS_EXT_FILE: &str = "keybindings_ext.yaml";
const RULES_FILE: &str = "rules.yaml";

#[derive(Serialize,Deserialize,PartialEq,Debug,Copy,Clone)]
#[serde(default)]
pub struct Configuration {
    /// number of workspaces for primary monitor
    pub primary_workspaces: u32,

    /// number of workspaces for secondary monitors
    pub secondary_workspaces: u32,

    /// layout configuration
    pub layout: LayoutConfiguration,

    /// theming configuration
    pub theming: ThemingConfiguration,
}

#[derive(Serialize,Deserialize,PartialEq,Debug,Copy,Clone)]
#[serde(default)]
pub struct LayoutConfiguration {
    /// default layout for each workspace
    pub default: LayoutType,

    /// width of the gap between windows in a tiled layout
    pub gap_width: u32,

    /// ratio of main area vs stack area in a tiled layout
    pub main_ratio: f32,

    /// number of windows in the main area
    pub nmain: u32,

    /// position of the stack relative to the main windows (dynamic layout)
    pub stack_position: StackPosition,

    /// mode of laying out the windows in the stack area (dynamic layout)
    pub stack_mode: StackMode,
}

#[derive(Serialize,Deserialize,PartialEq,Eq,Debug,Copy,Clone)]
#[serde(default)]
pub struct ThemingConfiguration {
    /// primary highlight color
    pub primary_color: u64,

    /// secondary highlight color
    pub secondary_color: u64,

    /// default color
    pub background_color: u64,

    /// width of the frame that client windows are reparented to
    pub frame_width: u32,

    /// width of the border around the inner window
    pub inner_border_width: u32,

    /// width of the border around the frame
    pub outer_border_width: u32,
}

impl Default for Configuration {
    fn default() -> Self {
        return Configuration {
            primary_workspaces: 8,
            secondary_workspaces: 2,
            layout: LayoutConfiguration::default(),
            theming: ThemingConfiguration::default(),
        }
    }
}

impl Default for LayoutConfiguration {
    fn default() -> Self {
        return LayoutConfiguration {
            default: LayoutType::Floating,
            gap_width: 4,
            main_ratio: 0.6,
            nmain: 1,
            stack_position: StackPosition::Right,
            stack_mode: StackMode::Split,
        };
    }
}

impl Default for ThemingConfiguration {
    fn default() -> Self {
        return ThemingConfiguration {
            primary_color: 0xae0c0c,
            secondary_color: 0x1f464f,
            background_color: 0xceccc6,
            frame_width: 4,
            inner_border_width: 1,
            outer_border_width: 1
        };
    }
}

pub fn read_config() -> Configuration {
    return match read_config_file(CONFIG_DIR, CONFIG_FILE) {
        Ok(config) => config,
        Err(msg) => {
            println!("Unable to read configuration: {}", msg);
            Configuration::default()
        },
    };
}

pub fn read_keybindings(nworkspaces: u32) -> Vec<Keybinding> {
    // read keybindings file
    let mut keybindings = match read_config_file(CONFIG_DIR, KEYBINDINGS_FILE) {
        Ok(config) => config,
        Err(msg) => {
            println!("Unable to read key bindings: {}", msg);
            default_keybindings(nworkspaces)
        },
    };

    // read extended keybindings
    if let Ok(config) = read_config_file::<Vec<Keybinding>>(CONFIG_DIR, KEYBINDINGS_EXT_FILE) {
        keybindings.extend(config);
    }

    return keybindings;
}

pub fn read_rules() -> Vec<Rule> {
    let result = read_config_file(CONFIG_DIR, RULES_FILE);
    return match result {
        Ok(rules) => rules,
        Err(msg) => {
            println!("Unable to read window rules: {}", msg);
            Vec::new()
        },
    };
}

