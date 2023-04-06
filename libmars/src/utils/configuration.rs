//! Loading configuration files using [serde_yaml].

use std::fs;
use std::path;
use serde::{Serialize, Deserialize};


fn deserialize_file<T: for<'a> Deserialize<'a>>(path: &path::Path) -> Result<T, (bool, String)> {
    let fs_result = fs::read(path);
    let raw = match fs_result {
        Ok(content) => content,
        Err(e) => return Err((true, e.to_string())),
    };

    match serde_yaml::from_slice(&raw) {
        Ok(config) => Ok(config),
        Err(e) => Err((true, e.to_string())),
    }
}

/// Print config files to stdout
pub fn print_config(config: &impl Serialize) {
    let ser = serde_yaml::to_string(config);
    match ser {
        Ok(ser) => println!("{}", ser),
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Read config file from a direct path
///
/// * `path` - The whole path to the config file
pub fn read_file<T: for<'a> Deserialize<'a>>(path: &path::Path) -> Result<T, String> {
    if path.is_file() {
        deserialize_file(path).map_err(|(_, msg)| msg)
    } else {
        Err(format!("configuration {} not found", path.to_string_lossy()))
    }
}

/// Read config file
///
/// * `config_name` - The name of the applications configuration subdirectory
/// * `file_name` - The file name (with extension)
pub fn read_config_file<T: for<'a> Deserialize<'a>>(config_name: &str, file_name: &str) -> Result<T, String> {
    let config_dir = xdg::BaseDirectories::with_prefix(config_name)
        .map_err(|e| format!("unable to open config dir ({})", e))?;
    if let Some(path) = config_dir.find_config_file(file_name) {
        deserialize_file(&path).map_err(|(_, msg)| msg)
    } else {
        Err(format!("configuration {} not found", file_name))
    }
}
