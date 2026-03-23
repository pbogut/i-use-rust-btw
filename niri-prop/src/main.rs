use clap::{arg, Command};
use niri_ipc::socket::Socket;
use niri_ipc::{Event, Request, Response, Window};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct Props {
    window_id: u64,
    title: Option<String>,
    app_id: Option<String>,
    workspace_id: Option<u64>,
    is_floating: bool,
}

fn cli() -> Command {
    Command::new("niri-prop")
        .about("Window properties query tool for niri")
        .args(vec![
            arg!(-l --"listen" "Listen for focus changes"),
            arg!(-t --"title" "Print window title"),
            arg!(-i --"app-id" "Prints window app_id"),
        ])
}

fn main() {
    let matches = cli().get_matches();

    let print_app_id = matches.get_flag("app-id");
    let print_title = matches.get_flag("title");

    let print_props = !print_title && !print_app_id;

    if let Some(window) = get_focused_window() {
        display_window(&window, print_props, print_app_id, print_title);
    }

    if matches.get_flag("listen") {
        let mut socket = Socket::connect().expect("Failed to connect to niri socket");
        let reply = socket
            .send(Request::EventStream)
            .expect("Failed to request event stream");

        match reply {
            Ok(Response::Handled) => {}
            other => {
                eprintln!("Unexpected reply to EventStream: {other:?}");
                std::process::exit(1);
            }
        }

        let mut read_event = socket.read_events();

        loop {
            let event = match read_event() {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Event stream error: {e}");
                    break;
                }
            };

            match event {
                Event::WindowOpenedOrChanged { window } => {
                    if window.is_focused {
                        display_window(&window, print_props, print_app_id, print_title);
                    }
                }
                Event::WindowFocusChanged { id } => {
                    if id.is_some() {
                        if let Some(window) = get_focused_window() {
                            display_window(&window, print_props, print_app_id, print_title);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn get_focused_window() -> Option<Window> {
    let mut socket = Socket::connect().ok()?;
    let reply = socket.send(Request::FocusedWindow).ok()?;
    match reply {
        Ok(Response::FocusedWindow(window)) => window,
        _ => None,
    }
}

fn display_window(window: &Window, print_props: bool, print_app_id: bool, print_title: bool) {
    if print_props {
        display_props(window);
    }
    if print_app_id {
        display_app_id(window);
    }
    if print_title {
        display_title(window);
    }
}

fn display_props(window: &Window) {
    let props = Props {
        window_id: window.id.clone(),
        title: window.title.clone(),
        app_id: window.app_id.clone(),
        workspace_id: window.workspace_id.clone(),
        is_floating: window.is_floating,
    };
    println!("{}", json!(props));
}

fn display_title(window: &Window) {
    println!("{}", window.title.clone().map_or_else(String::new, |t| t));
}

fn display_app_id(window: &Window) {
    println!("{}", window.app_id.clone().map_or_else(String::new, |t| t));
}
