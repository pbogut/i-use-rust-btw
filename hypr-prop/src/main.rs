use clap::{arg, Command};
use hyprland::event_listener::{EventListener,WindowEventData};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct Props {
    title: Option<String>,
    class: Option<String>,
    address: Option<String>,
}

fn cli() -> Command {
    Command::new("hypr-prop")
        .about("Window properties change listen tool for hyprland")
        .args(vec![
            arg!(-a --"address" "Prints window address"),
            arg!(-c --"class" "Prints window class"),
            arg!(-t --"title" "Print window title"),
        ])
}

fn main() -> hyprland::Result<()> {
    let matches = cli().get_matches();

    let print_address = matches.get_flag("address");
    let print_class = matches.get_flag("class");
    let print_title = matches.get_flag("title");

    let print_props = !print_title && !print_class && !print_address;

    let mut listener = EventListener::new();
    listener.add_active_window_changed_handler(move |data| {
        let props = data.clone().unwrap();

        if print_props {
            display_props(&props);
        }
        if print_title {
            display_title(&props.title);
        }
        if print_class {
            display_class(&props.class);
        }
        if print_address {
            display_address(&props.address.to_string());
        }
    });
    listener.start_listener()?;
    Ok(())
}

fn display_props(data: &WindowEventData) {
    let props = Props {
        title: Some(data.title.clone()),
        class: Some(data.class.clone()),
        address: Some(data.address.to_string())
    };
    println!("{}", json!(props));
}

fn display_title(title: &str) {
    println!("{}", title);
}

fn display_class(class: &str) {
    println!("{}", class);
}

fn display_address(address: &str) {
    println!("{}", address);
}
