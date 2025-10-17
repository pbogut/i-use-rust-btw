use clap::{arg, value_parser, Command};
use hyprland::dispatch::{Direction as HyprDirection, Dispatch, DispatchType};
use i3_focus::{
    nvim, tmux,
    wezterm::{self, WezTermId},
    zellij, Direction,
};
use serde_json::Value;

fn cli() -> Command {
    Command::new("hypr-focus")
        .about("Change focus between hypr / tmux / vim")
        .args(vec![
            arg!(<DIRECTION> "Focus direction").value_parser(value_parser!(Direction)),
            arg!(--"skip-nvim" "Skip nvim check"),
        ])
}

fn main() {
    let matches = cli().get_matches();

    let direction = matches
        .get_one::<Direction>("DIRECTION")
        .expect("Direction has to be provided");

    match get_focused_name() {
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

            handle_hypr(direction);
        }
        None => handle_hypr(direction),
    }
}

fn handle_hypr(direction: &Direction) {
    let hypr_dir = match direction {
        Direction::Left => HyprDirection::Left,
        Direction::Right => HyprDirection::Right,
        Direction::Up => HyprDirection::Up,
        Direction::Down => HyprDirection::Down,
    };
    Dispatch::call(DispatchType::MoveFocus(hypr_dir)).unwrap();
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

// TODO: use Hyprland API or something (proably need to contribute to hyprland-rs first)
fn get_focused_name() -> Option<String> {
    let output = std::process::Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    let value: Value = serde_json::from_str(&stdout).ok()?;

    value.get("title")?.as_str().map(|title| title.to_string())
}

fn get_tmux_id(name: &str) -> Option<usize> {
    match name.splitn(2, " |t$").nth(1) {
        Some(id) => match id.parse::<usize>() {
            Ok(id) => Some(id),
            Err(_) => None,
        },
        None => None,
    }
}

fn get_wezterm_id(name: &str) -> Option<WezTermId> {
    match name.split(" |w$").nth(1) {
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
