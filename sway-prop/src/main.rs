use clap::{arg, Command};
use serde::{Deserialize, Serialize};
use serde_json::json;
use swayipc::{Connection, EventType, Fallible};
use swayipc_types::{Event, Node, WindowProperties};

#[derive(Serialize, Deserialize)]
struct Props {
    title: Option<String>,
    app_id: Option<String>,
    class: Option<String>,
    instance: Option<String>,
}

fn cli() -> Command {
    Command::new("i3-prop")
        .about("Window properties quiry tool for i3 / sway")
        .args(vec![
            arg!(-l --"listen" "Listen for focus changes"),
            arg!(-t --"title" "Print window title"),
            arg!(-i --"app-id" "Prints window class"),
            arg!(--"x-class" "Prints window class"),
            arg!(-x --"x-instance-class" "Prints window instance and class"),
        ])
}

fn main() -> Fallible<()> {
    let matches = cli().get_matches();

    let subs = [EventType::Window];

    let print_app_id = matches.get_flag("app-id");
    let print_title = matches.get_flag("title");
    let print_class = matches.get_flag("x-class");
    let print_icls = matches.get_flag("x-instance-class");

    let print_props = !print_title && !print_class && !print_icls && !print_app_id;

    let mut sway = Connection::new()?;

    get_focused_node(&mut sway).map_or((), |node| {
        display_node(
            &node,
            print_props,
            print_app_id,
            print_title,
            print_class,
            print_icls,
        );
    });

    if matches.get_flag("listen") {
        for event in (sway.subscribe(subs)?).flatten() {
            if let Event::Window(ev) = event {
                display_node(
                    &ev.container,
                    print_props,
                    print_app_id,
                    print_title,
                    print_class,
                    print_icls,
                );
            }
        }
    }

    Ok(())
}

fn display_node(
    node: &Node,
    print_props: bool,
    print_app_id: bool,
    print_title: bool,
    print_class: bool,
    print_icls: bool,
) {
    if print_props {
        display_props(&node);
    }
    if print_app_id {
        display_app_id(&node);
    }
    if print_title {
        display_title(&node);
    }
    if print_class {
        display_class(&node.window_properties);
    }
    if print_icls {
        display_icls(&node.window_properties);
    }
}

fn display_props(node: &Node) {
    let props = Props {
        title: node.name.clone(),
        app_id: node.app_id.clone(),
        class: node.window_properties.clone().and_then(|p| p.class),
        instance: node.window_properties.clone().and_then(|p| p.instance),
    };
    println!("{}", json!(props));
}

fn display_title(node: &Node) {
    println!("{}", node.name.clone().map_or_else(String::new, |t| t));
}

fn display_app_id(node: &Node) {
    println!("{}", node.app_id.clone().map_or_else(String::new, |t| t));
}

fn display_class(window_properties: &Option<WindowProperties>) {
    println!(
        "{}",
        window_properties
            .clone()
            .and_then(|p| p.class)
            .map_or_else(String::new, |t| t)
    );
}

fn display_icls(window_properties: &Option<WindowProperties>) {
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

fn collect_focused<'a>(node: &'a Node, mut r: Vec<&'a Node>) -> Vec<&'a Node> {
    if node.focused {
        r.push(node);
    }
    for n in &node.nodes {
        r = collect_focused(n, r);
    }
    for n in &node.floating_nodes {
        r = collect_focused(n, r);
    }
    r
}

fn get_focused(node: &Node) -> Vec<&Node> {
    let v: Vec<&Node> = vec![];
    collect_focused(node, v)
}

fn get_focused_node(sway: &mut Connection) -> Option<Node> {
    sway.get_tree().map_or(None, |tree| {
        let focused_list = get_focused(&tree);
        focused_list.first().map(|&node| node.clone())
    })
}
