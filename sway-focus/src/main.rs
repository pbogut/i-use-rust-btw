use clap::{arg, value_parser, Command};
use i3_focus::{nvim, tmux, Direction};
use swayipc::Connection;
use swayipc_types::Node;

fn cli() -> Command {
    Command::new("sway-focus")
        .about("Change focus between sway / tmux / vim")
        .args(vec![
            arg!(<DIRECTION> "Focus direction").value_parser(value_parser!(Direction)),
            arg!(--"skip-nvim" "Skip nvim check"),
        ])
}

fn main() {
    let matches = cli().get_matches();

    let mut sway = swayipc::Connection::new().expect("Can not connect to sway ipc");

    let direction = matches
        .get_one::<Direction>("DIRECTION")
        .expect("Direction has to be provided");

    match get_focused_name(&mut sway) {
        Some(name) => {
            let skip_vim = matches.get_flag("skip-nvim");
            let tmux_id = get_tmux_id(&name);
            let nvim_id = get_nvim_id(&name);
            let tmux_edge = tmux_id.map_or(false, |id| tmux::is_tmux_edge(id, direction));
            // let tmux_edge = match tmux_id {
            //     Some(id) => tmux::is_tmux_edge(id, direction),
            //     None => false,
            // };

            match (nvim_id, skip_vim, tmux_id, tmux_edge) {
                (Some(id), false, _, _) => handle_nvim(id, direction),
                (_, _, Some(id), false) => handle_tmux(id, direction),
                _ => handle_sway(&mut sway, direction),
            }
        }
        None => handle_sway(&mut sway, direction),
    }
}

fn handle_sway(sway: &mut Connection, direction: &Direction) {
    sway.run_command(format!("focus {}", direction))
        .unwrap_or_default();
}

fn handle_nvim(id: usize, direction: &Direction) {
    nvim::focus(id, direction);
}

fn handle_tmux(id: usize, direction: &Direction) {
    tmux::focus(id, direction);
}

fn get_focused_name(sway: &mut Connection) -> Option<String> {
    match sway.get_tree() {
        Ok(tree) => {
            let focused_list = get_focused(&tree);

            match focused_list.first() {
                Some(focused) => focused.name.clone(),
                None => None,
            }
        }
        Err(_) => None,
    }
}

fn get_tmux_id(name: &str) -> Option<usize> {
    match name.split(" |t$").last() {
        Some(id) => match id.parse::<usize>() {
            Ok(id) => Some(id),
            Err(_) => None,
        },
        None => None,
    }
}

fn get_nvim_id(name: &str) -> Option<usize> {
    let mut p = name.split(':');
    match (p.nth(1), p.next()) {
        (Some("nvim"), Some(id)) => match id.parse() {
            Ok(id) => Some(id),
            Err(_) => None,
        },
        _ => None,
    }
}

fn collect_focused<'a>(node: &'a Node, mut r: Vec<&'a Node>) -> Vec<&'a Node> {
    if node.focused {
        r.push(node)
    }
    for n in &node.nodes {
        r = collect_focused(n, r)
    }
    r
}

fn get_focused(node: &Node) -> Vec<&Node> {
    let v: Vec<&Node> = vec![];
    collect_focused(node, v)
}
