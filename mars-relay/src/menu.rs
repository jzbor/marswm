use std::io::Write;
use std::process;
use std::thread;

use crate::*;

fn menu_input() -> String {
    let mut string = String::new();
    string.push_str("x Close	close\n");
    string.push_str("+ Fullscreen	fullscreen\n");
    string.push_str("' Pin	pin\n");
    string.push_str("* Unpin	unpin\n");

    return string;
}

pub fn display_menu() -> Result<Command, String> {
    let mut child = process::Command::new("xmenu")
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let mut stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => return Err("unable to open stdin".to_owned()),
    };
    // might cause deadlock if the program doesn't clear the pipe without reading
    thread::spawn(move || {
        let string = menu_input();
        stdin.write_all(string.as_bytes()).unwrap();
    });


    let output = child.wait_with_output()
        .map_err(|e| e.to_string())?;
    let output = String::from_utf8_lossy(&output.stdout)
        .to_string();

    return match output.trim() {
        "close" => Ok(Command::Close),
        "fullscreen" => Ok(Command::ToggleFullscreen),
        "pin" => Ok(Command::Pin),
        "unpin" => Ok(Command::Unpin),
        _ => return Err("no match".to_string()),
    };
}
