use clap::{arg, value_parser, Command};
use hyprland::dispatch::{Dispatch, DispatchType};
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
            arg!(--"switch-monitor" "Allow left/right focus to switch monitors at the edge"),
            arg!(--"switch-workspace" "Allow up/down focus to switch workspaces at the edge"),
        ])
}

fn main() {
    let matches = cli().get_matches();

    let direction = matches
        .get_one::<Direction>("DIRECTION")
        .expect("Direction has to be provided");
    let switch_monitor = matches.get_flag("switch-monitor");
    let switch_workspace = matches.get_flag("switch-workspace");

    match get_focused_name() {
        Some(name) => {
            let skip_vim = matches.get_flag("skip-nvim");
            let nvim_id = nvim::id_from_title(&name);
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

            handle_hypr(direction, switch_monitor, switch_workspace);
        }
        None => handle_hypr(direction, switch_monitor, switch_workspace),
    }
}

fn handle_hypr(direction: &Direction, switch_monitor: bool, switch_workspace: bool) {
    let horizontal = matches!(direction, Direction::Left | Direction::Right);
    let step = match direction {
        Direction::Left | Direction::Up => -1,
        Direction::Right | Direction::Down => 1,
    };
    // Horizontal scrolling: check column boundaries before layout focus can wrap.
    let dispatcher = format!(
        r#"(function()
    local direction = "{direction}"
    local horizontal = {horizontal}
    local step = {step}
    local window = hl.get_active_window()
    local workspace = window and window.workspace
        or hl.get_active_special_workspace() or hl.get_active_workspace()
    if not workspace then
        return hl.dsp.no_op()
    end
    if workspace.tiled_layout ~= "scrolling" or (window and window.floating) then
        return hl.dsp.focus({{ direction = direction }})
    end

    if window then
        local layout = window.layout
        if not layout or not layout.column then
            return hl.dsp.no_op()
        end
        if horizontal then
            local next_column = layout.column.index + step
            for _, candidate in ipairs(hl.get_workspace_windows(workspace)) do
                local other = candidate.layout
                if other and other.column and other.column.index == next_column then
                    return hl.dsp.layout("focus " .. direction)
                end
            end
        else
            -- Layout indices are zero-based; column.windows is a Lua array.
            local next_index = layout.index_in_column + step
            if next_index >= 0 and next_index < #layout.column.windows then
                return hl.dsp.layout("focus " .. direction)
            end
        end
    end

    if horizontal and {switch_monitor} then
        local monitor = hl.get_monitor(direction)
        if monitor and monitor ~= hl.get_active_monitor() then
            return hl.dsp.focus({{ monitor = monitor }})
        end
    elseif not horizontal and {switch_workspace} then
        return hl.dsp.focus({{ workspace = step < 0 and "m-1" or "m+1" }})
    end
    return hl.dsp.no_op()
end)()"#
    );
    Dispatch::call(DispatchType::Custom(&dispatcher, "")).unwrap();
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
