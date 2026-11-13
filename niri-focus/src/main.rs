use clap::{arg, builder::PossibleValue, value_parser, Command, ValueEnum};
use i3_focus::{
    herdr, nvim, tmux,
    wezterm::{self, WezTermId},
    zellij, Direction,
};
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response, Window, Workspace, WorkspaceReferenceArg};

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
            arg!(--"skip-empty-named-workspaces" "Skip empty named workspaces with next-workspace, prev-workspace, or up/down --switch-workspace"),
        ])
}

fn main() -> Result<(), String> {
    let matches = cli().get_matches();
    let skip_empty_named_workspaces = matches.get_flag("skip-empty-named-workspaces");

    let action = matches
        .get_one::<FocusAction>("DIRECTION")
        .expect("Direction has to be provided");
    let direction = match action {
        FocusAction::Direction(direction) => direction,
        FocusAction::NextWorkspace => {
            return handle_workspace(&Direction::Down, skip_empty_named_workspaces, false);
        }
        FocusAction::PrevWorkspace => {
            return handle_workspace(&Direction::Up, skip_empty_named_workspaces, false);
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
                    return Ok(());
                }
            }

            let wezterm_id = get_wezterm_id(name);
            if let Some(wezterm_id) = wezterm_id {
                if handle_wezterm(&wezterm_id, direction) {
                    return Ok(());
                }
            }

            let tmux_id = get_tmux_id(name);
            let tmux_edge = tmux_id.map_or(false, |id| tmux::is_tmux_edge(id, direction));
            if tmux_id.is_some() && !tmux_edge {
                handle_tmux(tmux_id.unwrap_or_default(), direction);
                return Ok(());
            }

            let zellij_id = get_zellij_id(name);
            if zellij_id.is_some() {
                if handle_zellij(&zellij_id.unwrap_or_default(), direction) {
                    return Ok(());
                }
            }

            if is_herdr_window(window.app_id.as_deref(), window.title.as_deref())
                && herdr::focus(direction)
            {
                return Ok(());
            }

            handle_niri(
                direction,
                switch_monitor,
                switch_workspace,
                skip_empty_named_workspaces,
            )
        }
        None => handle_niri(
            direction,
            switch_monitor,
            switch_workspace,
            skip_empty_named_workspaces,
        ),
    }
}

fn handle_niri(
    direction: &Direction,
    switch_monitor: bool,
    switch_workspace: bool,
    skip_empty_named_workspaces: bool,
) -> Result<(), String> {
    let action = match direction {
        Direction::Left if switch_monitor => Action::FocusColumnOrMonitorLeft {},
        Direction::Right if switch_monitor => Action::FocusColumnOrMonitorRight {},
        Direction::Up | Direction::Down if switch_workspace => {
            return handle_workspace(direction, skip_empty_named_workspaces, true);
        }
        Direction::Left => Action::FocusColumnLeft {},
        Direction::Right => Action::FocusColumnRight {},
        Direction::Up => Action::FocusWindowUp {},
        Direction::Down => Action::FocusWindowDown {},
    };
    send_action(action)
}

fn niri_request(request: Request) -> Result<Response, String> {
    let mut socket =
        Socket::connect().map_err(|error| format!("Failed to connect to niri socket: {error}"))?;
    socket
        .send(request)
        .map_err(|error| format!("Failed to send request to niri: {error}"))?
}

fn send_action(action: Action) -> Result<(), String> {
    match niri_request(Request::Action(action))? {
        Response::Handled => Ok(()),
        response => Err(format!("Unexpected niri action response: {response:?}")),
    }
}

fn workspace_state() -> Result<(Vec<Workspace>, Workspace), String> {
    match niri_request(Request::Workspaces)? {
        Response::Workspaces(workspaces) => {
            let focused = workspaces
                .iter()
                .find(|workspace| workspace.is_focused)
                .cloned()
                .ok_or("No focused workspace")?;
            Ok((workspaces, focused))
        }
        response => Err(format!("Unexpected niri workspaces response: {response:?}")),
    }
}

fn handle_workspace(
    direction: &Direction,
    skip_empty_named_workspaces: bool,
    window_first: bool,
) -> Result<(), String> {
    let native_action = match (direction, window_first) {
        (Direction::Up, true) => Action::FocusWindowOrWorkspaceUp {},
        (Direction::Down, true) => Action::FocusWindowOrWorkspaceDown {},
        (Direction::Up, false) => Action::FocusWorkspaceUp {},
        (Direction::Down, false) => Action::FocusWorkspaceDown {},
        _ => unreachable!("Workspace navigation is vertical"),
    };
    if !skip_empty_named_workspaces {
        return send_action(native_action);
    }

    let (mut workspaces, mut current) = match workspace_state() {
        Ok(state) => state,
        Err(error) => {
            eprintln!("niri-focus: {error}; falling back to native workspace navigation");
            return send_action(native_action);
        }
    };

    if window_first {
        send_action(match direction {
            Direction::Up => Action::FocusWindowUp {},
            _ => Action::FocusWindowDown {},
        })?;
        let (updated_workspaces, updated_current) = workspace_state()?;
        // Layout selection can change even when no application has keyboard focus.
        if current.id != updated_current.id
            || current.output != updated_current.output
            || current.active_window_id != updated_current.active_window_id
        {
            return Ok(());
        }
        workspaces = updated_workspaces;
        current = updated_current;
    }

    if let Some(workspace) = next_workspace(&workspaces, &current, direction) {
        send_action(Action::FocusWorkspace {
            reference: WorkspaceReferenceArg::Id(workspace.id),
        })?;
    }
    Ok(())
}

fn next_workspace<'a>(
    workspaces: &'a [Workspace],
    current: &Workspace,
    direction: &Direction,
) -> Option<&'a Workspace> {
    let eligible = workspaces.iter().filter(|workspace| {
        workspace.output == current.output
            && (workspace.name.is_none() || workspace.active_window_id.is_some())
    });
    match direction {
        Direction::Up => eligible
            .filter(|workspace| workspace.idx < current.idx)
            .max_by_key(|workspace| workspace.idx),
        Direction::Down => eligible
            .filter(|workspace| workspace.idx > current.idx)
            .min_by_key(|workspace| workspace.idx),
        _ => None,
    }
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
    use super::{is_herdr_window, next_workspace};
    use i3_focus::Direction;
    use niri_ipc::Workspace;

    fn workspace(id: u64, idx: u8, name: Option<&str>, window: Option<u64>) -> Workspace {
        Workspace {
            id,
            idx,
            name: name.map(str::to_owned),
            output: Some("DP-1".to_owned()),
            is_urgent: false,
            is_active: false,
            is_focused: false,
            active_window_id: window,
        }
    }

    #[test]
    fn skips_empty_named_workspaces_in_index_order() {
        let workspaces = [
            workspace(70, 7, Some("occupied"), Some(1)),
            workspace(20, 2, Some("empty"), None),
            workspace(80, 8, None, None),
            workspace(60, 6, Some("also-empty"), None),
            workspace(500, 1, None, None),
            workspace(99, 5, Some("work"), Some(2)),
            workspace(30, 3, Some("another-empty"), None),
        ];
        for (id, direction, expected) in [
            (500, Direction::Down, Some(99)),
            (99, Direction::Up, Some(500)),
            (99, Direction::Down, Some(70)),
            (70, Direction::Up, Some(99)),
            (70, Direction::Down, Some(80)),
            (80, Direction::Up, Some(70)),
            (80, Direction::Down, None),
            (500, Direction::Up, None),
            (20, Direction::Down, Some(99)),
            (20, Direction::Up, Some(500)),
        ] {
            let current = workspaces
                .iter()
                .find(|workspace| workspace.id == id)
                .unwrap();
            assert_eq!(
                next_workspace(&workspaces, current, &direction).map(|workspace| workspace.id),
                expected,
                "from {id} going {direction}",
            );
        }
    }

    #[test]
    fn stops_when_only_empty_named_workspaces_or_other_outputs_remain() {
        let mut workspaces = [
            workspace(1, 1, None, Some(1)),
            workspace(2, 2, Some("empty-above"), None),
            workspace(3, 3, Some("current"), Some(3)),
            workspace(4, 4, Some("empty-below"), None),
            workspace(5, 5, None, Some(5)),
        ];
        workspaces[0].output = Some("DP-2".to_owned());
        workspaces[4].output = Some("DP-2".to_owned());
        for direction in [Direction::Up, Direction::Down] {
            assert!(next_workspace(&workspaces, &workspaces[2], &direction).is_none());
        }
    }

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
