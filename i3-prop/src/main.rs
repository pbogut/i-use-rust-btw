use clap::{arg, Command};
use i3_ipc::{
    event::{Event, Subscribe},
    I3Stream,
};
use i3ipc_types::reply;
use serde_json::json;
use std::io;

fn cli() -> Command {
    Command::new("i3-prop")
        .about("Window properties quiry tool for i3 / sway")
        .args(vec![
            arg!(-l --"listen" "Listen for focus changes"),
            arg!(-t --"title" "Print window title"),
            arg!(-c --"class" "Prints window class"),
            arg!(-s --"instance-class" "Prints window instance and class"),
        ])
}

fn main() -> io::Result<()> {
    let matches = cli().get_matches();

    let mut i3 = I3Stream::conn_sub([Subscribe::Window, Subscribe::Workspace])?;

    let print_title = matches.get_flag("title");
    let print_class = matches.get_flag("class");
    let print_icls = matches.get_flag("instance-class");

    let print_props = !print_title && !print_class && !print_icls;

    get_focused_node(&mut i3).map_or((), |node| {
        display_node(&node, print_props, print_title, print_class, print_icls);
    });

    if matches.get_flag("listen") {
        for e in i3.listen() {
            match e? {
                Event::Window(ev) => {
                    display_node(
                        &ev.container,
                        print_props,
                        print_title,
                        print_class,
                        print_icls,
                    );
                }
                Event::Workspace(_ev) => (),
                Event::Output(_ev) => (),
                Event::Mode(_ev) => (),
                Event::BarConfig(_ev) => (),
                Event::Binding(_ev) => (),
                Event::Shutdown(_ev) => (),
                Event::Tick(_ev) => (),
            }
        }
    }

    Ok(())
}

fn display_node(
    node: &reply::Node,
    print_props: bool,
    print_title: bool,
    print_class: bool,
    print_icls: bool,
) {
    if print_props {
        display_props(&node.window_properties);
    }
    if print_title {
        display_title(&node.window_properties);
    }
    if print_class {
        display_class(&node.window_properties);
    }
    if print_icls {
        display_icls(&node.window_properties);
    }
}

fn display_props(window_properties: &Option<reply::WindowProperties>) {
    println!("{}", json!(window_properties));
}

fn display_title(window_properties: &Option<reply::WindowProperties>) {
    println!(
        "{}",
        window_properties
            .clone()
            .and_then(|p| p.title)
            .map_or_else(String::new, |t| t)
    );
}

fn display_class(window_properties: &Option<reply::WindowProperties>) {
    println!(
        "{}",
        window_properties
            .clone()
            .and_then(|p| p.class)
            .map_or_else(String::new, |t| t)
    );
}

fn display_icls(window_properties: &Option<reply::WindowProperties>) {
    println!(
        "{} {}",
        window_properties
            .clone()
            .and_then(|p| p.instance)
            .map_or_else(String::new, |t| t),
        window_properties
            .clone()
            .and_then(|p| p.class)
            .map_or_else(String::new, |t| t)
    );
}

fn collect_focused<'a>(node: &'a reply::Node, mut r: Vec<&'a reply::Node>) -> Vec<&'a reply::Node> {
    if node.focused {
        r.push(node);
    }
    for n in &node.nodes {
        r = collect_focused(n, r);
    }
    r
}

fn get_focused(node: &reply::Node) -> Vec<&reply::Node> {
    let v: Vec<&reply::Node> = vec![];
    collect_focused(node, v)
}

fn get_focused_node(i3: &mut I3Stream) -> Option<reply::Node> {
    i3.get_tree().map_or(None, |tree| {
        let focused_list = get_focused(&tree);
        focused_list.first().map(|&node| node.clone())
    })
}
