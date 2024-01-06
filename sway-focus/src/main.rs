use clap::{arg, value_parser, Command};
use i3_focus::{
    nvim, tmux,
    wezterm::{self, WezTermId},
    zellij, Direction,
};
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
            let nvim_id = get_nvim_id(&name);
            if nvim_id.is_some() && !skip_vim {
                handle_nvim(nvim_id.unwrap_or_default(), direction);
                return;
            }

            let wezterm_id = get_wezterm_id(&name);
            if let Some(wezterm_id) = wezterm_id {
                if handle_wezterm(&wezterm_id, direction) {
                    return;
                }
            }

            let tmux_id = get_tmux_id(&name);
            let tmux_edge = tmux_id.map_or(false, |id| tmux::is_tmux_edge(id, direction));
            if tmux_id.is_some() && !tmux_edge {
                handle_tmux(tmux_id.unwrap_or_default(), direction);
                return;
            }

            let zellij_id = get_zellij_id(&name);
            if zellij_id.is_some() {
                if handle_zellij(&zellij_id.unwrap_or_default(), direction) {
                    return;
                }
            }

            handle_sway(&mut sway, direction);
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

fn handle_wezterm(wezterm_id: &WezTermId, direction: &Direction) -> bool {
    wezterm::focus(wezterm_id, direction)
}

fn handle_zellij(id: &str, direction: &Direction) -> bool {
    zellij::focus(id, direction)
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

fn get_wezterm_id(name: &str) -> Option<WezTermId> {
    match name.split(" |w$").last() {
        Some(pid_and_pane_id) => {
            let mut parts = pid_and_pane_id.splitn(2, ':');
            let pid = parts.next()?.parse::<usize>().ok()?;
            let pane_id = parts.next()?.parse::<usize>().ok()?;

            Some(WezTermId { pid, pane_id })
        }
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
