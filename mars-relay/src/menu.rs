use std::io::Write;
use std::process;
use std::thread;

use crate::*;

const MENU_PROGRAM: &str = "xmenu";

fn menu_input() -> String {
    let mut string = String::new();
    string.push_str("x Close	close\n");
    string.push_str("% Tile	tile\n");
    string.push_str("+ Fullscreen	fullscreen\n");
    string.push_str("' Pin	pin\n");

    return string;
}

pub fn display_menu() -> Result<Command, String> {
    let mut child = process::Command::new(MENU_PROGRAM)
        .stdin(process::Stdio::piped())
        .stdout(process::Stdio::piped())
        .spawn()
        .map_err(|_| format!("Please make sure `{}` is installed properly", MENU_PROGRAM))?;

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
        "tile" => Ok(Command::Tiled(Modifier::Toggle.into())),
        "fullscreen" => Ok(Command::Fullscreen(Modifier::Toggle.into())),
        "pin" => Ok(Command::Pinned(Modifier::Toggle.into())),
        _ => return Err("no match".to_string()),
    };
}
