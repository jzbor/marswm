//! Loading configuration files using [serde_yaml].

use std::env;
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
        Ok(config) => return Ok(config),
        Err(e) => return Err((true, e.to_string())),
    };
}

/// Print config files to stdout
pub fn print_config(config: &impl Serialize) {
    let ser = serde_yaml::to_string(config);
    match ser {
        Ok(ser) => println!("{}", ser),
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Read config file
///
/// * `config_dir` - The subdirectory name (not the whole path)
/// * `file_name` - The file name (with extension)
pub fn read_config_file<T: for<'a> Deserialize<'a>>(config_dir: &str, file_name: &str) -> Result<T, String>{
    // check configuration dir as specified in xdg base dir specification
    if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
        let path = path::Path::new(&xdg_config).join(config_dir).join(file_name);
        if path.is_file() {
            return deserialize_file(&path).map_err(|(_, msg)| msg);
        }
    }

    // check ~/.config
    if let Ok(home) = env::var("HOME") {
        let path = path::Path::new(&home).join(".config").join(config_dir).join(file_name);
        if path.is_file() {
            return deserialize_file(&path).map_err(|(_, msg)| msg);
        }
    }

    // check local working directory
    let path = path::Path::new(file_name);
    if path.is_file() {
        return deserialize_file(path).map_err(|(_, msg)| msg);
    } else {
        return Err(format!("configuration {} not found", file_name));
    }
}
