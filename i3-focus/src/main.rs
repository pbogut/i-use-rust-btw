use clap::{arg, value_parser, Command};
use i3_focus::{nvim, tmux, zellij, Direction};
use i3_ipc::{Connect, I3Stream, I3};
use i3ipc_types::reply;
use std::io;

fn cli() -> Command {
    Command::new("i3-focus")
        .about("Change focus between i3 / tmux / vim")
        .args(vec![
            arg!(<DIRECTION> "Focus direction").value_parser(value_parser!(Direction)),
            arg!(--"skip-nvim" "Skip nvim check"),
        ])
}

fn main() -> io::Result<()> {
    let matches = cli().get_matches();

    let mut i3 = I3::connect()?;

    let direction = matches
        .get_one::<Direction>("DIRECTION")
        .expect("Direction has to be provided");

    match get_focused_name(&mut i3) {
        Some(name) => {
            let skip_vim = matches.get_flag("skip-nvim");
            let nvim_id = get_nvim_id(&name);
            if nvim_id.is_some() && !skip_vim {
                handle_nvim(nvim_id.unwrap_or_default(), direction);
                return Ok(());
            }

            let tmux_id = get_tmux_id(&name);
            let tmux_edge = tmux_id.map_or(false, |id| tmux::is_tmux_edge(id, direction));
            if tmux_id.is_some() && !tmux_edge {
                handle_tmux(tmux_id.unwrap_or_default(), direction);
                return Ok(());
            }

            let zellij_id = get_zellij_id(&name);
            if zellij_id.is_some() {
                if handle_zellij(&zellij_id.unwrap_or_default(), direction) {
                    return Ok(());
                }
            }

            handle_i3(&mut i3, direction);
        }
        None => handle_i3(&mut i3, direction),
    }

    Ok(())
}

fn handle_i3(i3: &mut I3Stream, direction: &Direction) {
    i3.run_command(format!("focus {}", direction))
        .unwrap_or_default();
}

fn handle_nvim(id: usize, direction: &Direction) {
    nvim::focus(id, direction)
}

fn handle_tmux(id: usize, direction: &Direction) {
    tmux::focus(id, direction);
}

fn handle_zellij(id: &str, direction: &Direction) -> bool {
    zellij::focus(id, direction)
}

fn get_focused_name(i3: &mut I3Stream) -> Option<String> {
    match i3.get_tree() {
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

fn get_zellij_id(name: &str) -> Option<String> {
    if name.starts_with("Zellij (") {
        let start = name.find('(')?;
        let end = name.find(')')?;
        Some(name[start + 1..end].to_string())
    } else {
        None
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

fn collect_focused<'a>(node: &'a reply::Node, mut r: Vec<&'a reply::Node>) -> Vec<&'a reply::Node> {
    if node.focused {
        r.push(node)
    }
    for n in &node.nodes {
        r = collect_focused(n, r)
    }
    r
}

fn get_focused(node: &reply::Node) -> Vec<&reply::Node> {
    let v: Vec<&reply::Node> = vec![];
    collect_focused(node, v)
}
