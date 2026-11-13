use clap::{arg, builder::PossibleValue, value_parser, Command, ValueEnum};
use i3_focus::{
    herdr, nvim, tmux,
    wezterm::{self, WezTermId},
    zellij, Direction,
};
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response, Window};

#[derive(Clone)]
enum FocusAction {
    Direction(Direction),
    NextWorkspace,
    PrevWorkspace,
}

impl ValueEnum for FocusAction {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Direction(Direction::Left),
            Self::Direction(Direction::Right),
            Self::Direction(Direction::Up),
            Self::Direction(Direction::Down),
            Self::NextWorkspace,
            Self::PrevWorkspace,
        ]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Self::Direction(direction) => direction.to_possible_value(),
            Self::NextWorkspace => Some(
                PossibleValue::new("next-workspace")
                    .help("Focus the workspace below on the focused monitor"),
            ),
            Self::PrevWorkspace => Some(
                PossibleValue::new("prev-workspace")
                    .help("Focus the workspace above on the focused monitor"),
            ),
        }
    }
}

fn cli() -> Command {
    Command::new("niri-focus")
        .about("Change focus between niri / terminal multiplexers / vim")
        .args(vec![
            arg!(<DIRECTION> "Focus direction or workspace action")
                .value_parser(value_parser!(FocusAction)),
            arg!(--"skip-nvim" "Skip nvim check"),
            arg!(--"switch-monitor" "Allow left/right focus to switch monitors at the edge"),
            arg!(--"switch-workspace" "Allow up/down focus to switch workspaces at the edge"),
        ])
}

fn main() {
    let matches = cli().get_matches();

    let action = matches
        .get_one::<FocusAction>("DIRECTION")
        .expect("Direction has to be provided");
    let direction = match action {
        FocusAction::Direction(direction) => direction,
        FocusAction::NextWorkspace => {
            send_action(Action::FocusWorkspaceDown {});
            return;
        }
        FocusAction::PrevWorkspace => {
            send_action(Action::FocusWorkspaceUp {});
            return;
        }
    };
    let switch_monitor = matches.get_flag("switch-monitor");
    let switch_workspace = matches.get_flag("switch-workspace");

    match get_focused_window() {
        Some(window) => {
            let name = window.title.as_deref().unwrap_or_default();
            let skip_vim = matches.get_flag("skip-nvim");
            if !skip_vim {
                if let Some(nvim_id) = nvim::id_from_title(name) {
                    handle_nvim(nvim_id, direction);
                    return;
                }
            }

            let wezterm_id = get_wezterm_id(name);
            if let Some(wezterm_id) = wezterm_id {
                if handle_wezterm(&wezterm_id, direction) {
                    return;
                }
            }

            let tmux_id = get_tmux_id(name);
            let tmux_edge = tmux_id.map_or(false, |id| tmux::is_tmux_edge(id, direction));
            if tmux_id.is_some() && !tmux_edge {
                handle_tmux(tmux_id.unwrap_or_default(), direction);
                return;
            }

            let zellij_id = get_zellij_id(name);
            if zellij_id.is_some() {
                if handle_zellij(&zellij_id.unwrap_or_default(), direction) {
                    return;
                }
            }

            if is_herdr_window(window.app_id.as_deref(), window.title.as_deref())
                && herdr::focus(direction)
            {
                return;
            }

            handle_niri(direction, switch_monitor, switch_workspace);
        }
        None => handle_niri(direction, switch_monitor, switch_workspace),
    }
}

fn handle_niri(direction: &Direction, switch_monitor: bool, switch_workspace: bool) {
    let action = match direction {
        Direction::Left if switch_monitor => Action::FocusColumnOrMonitorLeft {},
        Direction::Right if switch_monitor => Action::FocusColumnOrMonitorRight {},
        Direction::Up if switch_workspace => Action::FocusWindowOrWorkspaceUp {},
        Direction::Down if switch_workspace => Action::FocusWindowOrWorkspaceDown {},
        Direction::Left => Action::FocusColumnLeft {},
        Direction::Right => Action::FocusColumnRight {},
        Direction::Up => Action::FocusWindowUp {},
        Direction::Down => Action::FocusWindowDown {},
    };
    send_action(action);
}

fn send_action(action: Action) {
    let mut socket = Socket::connect().expect("Failed to connect to niri socket");
    let _ = socket
        .send(Request::Action(action))
        .expect("Failed to send action to niri");
}

fn handle_nvim(id: &str, direction: &Direction) {
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

fn get_focused_window() -> Option<Window> {
    let mut socket = Socket::connect().ok()?;
    let reply = socket.send(Request::FocusedWindow).ok()?;
    match reply {
        Ok(Response::FocusedWindow(window)) => window,
        _ => None,
    }
}

fn is_herdr_window(app_id: Option<&str>, title: Option<&str>) -> bool {
    app_id == Some("Alacritty.herdr") || title.is_some_and(|title| title.ends_with(" |h$"))
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

#[cfg(test)]
mod tests {
    use super::is_herdr_window;

    #[test]
    fn detects_herdr_by_app_id_or_title_marker() {
        assert!(is_herdr_window(Some("Alacritty.herdr"), Some("zsh")));
        assert!(is_herdr_window(
            Some("Alacritty"),
            Some("zsh | project |h$")
        ));
        assert!(!is_herdr_window(Some("Alacritty"), Some("zsh")));
    }
}
