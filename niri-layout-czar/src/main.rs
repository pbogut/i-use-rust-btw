use niri_ipc::socket::Socket;
use niri_ipc::{Action, Event, Output, Request, Response, Window, Workspace};
use std::collections::HashMap;

/// Per-window info we track.
struct WindowInfo {
    workspace_id: Option<u64>,
    is_floating: bool,
    tile_width: f64,
}

/// Daemon state.
struct State {
    /// window_id -> info
    windows: HashMap<u64, WindowInfo>,
    /// workspace_id -> output name
    workspace_output: HashMap<u64, String>,
    /// output name -> logical width
    output_width: HashMap<String, f64>,
    /// workspace_id -> previous tiled window count
    prev_tiled_count: HashMap<u64, usize>,
    /// workspace_id -> the sole tiled window id (when count was 1)
    sole_window: HashMap<u64, u64>,
    /// currently focused window id
    focused_id: Option<u64>,
}

impl State {
    fn new() -> Self {
        Self {
            windows: HashMap::new(),
            workspace_output: HashMap::new(),
            output_width: HashMap::new(),
            prev_tiled_count: HashMap::new(),
            sole_window: HashMap::new(),
            focused_id: None,
        }
    }

    fn update_workspaces(&mut self, workspaces: &[Workspace]) {
        self.workspace_output.clear();
        for ws in workspaces {
            if let Some(output) = &ws.output {
                self.workspace_output.insert(ws.id, output.clone());
            }
        }
    }

    fn update_outputs(&mut self, outputs: &HashMap<String, Output>) {
        self.output_width.clear();
        for (name, o) in outputs {
            if let Some(logical) = &o.logical {
                self.output_width.insert(name.clone(), logical.width as f64);
            }
        }
    }

    /// Rebuild state from a full window list (initial sync).
    fn reset_windows(&mut self, windows: &[Window]) {
        self.windows.clear();
        for w in windows {
            self.windows.insert(
                w.id,
                WindowInfo {
                    workspace_id: w.workspace_id,
                    is_floating: w.is_floating,
                    tile_width: w.layout.tile_size.0,
                },
            );
            if w.is_focused {
                self.focused_id = Some(w.id);
            }
        }
    }

    /// Update or insert a window. Returns the old workspace_id if the window moved.
    fn upsert_window(&mut self, w: &Window) -> Option<(u64, Option<u64>)> {
        let old_ws = self.windows.get(&w.id).and_then(|info| info.workspace_id);
        let new_ws = w.workspace_id;
        let moved = match (old_ws, new_ws) {
            (Some(old), Some(new)) if old != new => Some((w.id, old_ws)),
            (None, Some(_)) => {
                // Could be a new window or first time seeing it; check if we knew it before.
                if self.windows.contains_key(&w.id) {
                    Some((w.id, old_ws))
                } else {
                    None
                }
            }
            _ => None,
        };
        self.windows.insert(
            w.id,
            WindowInfo {
                workspace_id: w.workspace_id,
                is_floating: w.is_floating,
                tile_width: w.layout.tile_size.0,
            },
        );
        if w.is_focused {
            self.focused_id = Some(w.id);
        }
        moved
    }

    fn remove_window(&mut self, id: u64) {
        self.windows.remove(&id);
        if self.focused_id == Some(id) {
            self.focused_id = None;
        }
    }

    /// Get the output width for a workspace.
    fn workspace_width(&self, ws_id: u64) -> Option<f64> {
        let output = self.workspace_output.get(&ws_id)?;
        self.output_width.get(output).copied()
    }

    /// Check if a window's column appears maximized (tile fills output width).
    fn is_maximized(&self, win_id: u64) -> bool {
        let info = match self.windows.get(&win_id) {
            Some(i) => i,
            None => return false,
        };
        let ws_id = match info.workspace_id {
            Some(id) => id,
            None => return false,
        };
        let output_w = match self.workspace_width(ws_id) {
            Some(w) => w,
            None => return false,
        };
        info.tile_width >= output_w * 0.95
    }

    /// Count tiled (non-floating) windows on a workspace.
    fn tiled_count(&self, ws_id: u64) -> usize {
        self.windows
            .values()
            .filter(|info| info.workspace_id == Some(ws_id) && !info.is_floating)
            .count()
    }

    /// Get the sole tiled window on a workspace, if exactly one.
    fn sole_tiled_window(&self, ws_id: u64) -> Option<u64> {
        let mut found = None;
        for (&win_id, info) in &self.windows {
            if info.workspace_id == Some(ws_id) && !info.is_floating {
                if found.is_some() {
                    return None;
                }
                found = Some(win_id);
            }
        }
        found
    }

    /// Collect all workspace IDs that have at least one tiled window.
    fn active_workspace_ids(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self
            .windows
            .values()
            .filter_map(|info| {
                if !info.is_floating {
                    info.workspace_id
                } else {
                    None
                }
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Snapshot current tiled counts for all workspaces.
    fn snapshot_counts(&self) -> HashMap<u64, usize> {
        let mut counts = HashMap::new();
        for ws_id in self.active_workspace_ids() {
            counts.insert(ws_id, self.tiled_count(ws_id));
        }
        counts
    }
}

/// Send an action on a fresh socket connection.
fn send_action(action: Action) {
    let mut sock = match Socket::connect() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect for action: {e}");
            return;
        }
    };
    if let Err(e) = sock.send(Request::Action(action)) {
        eprintln!("Failed to send action: {e}");
    }
}

/// Toggle maximize on a window. Handles focus switching and restoring.
fn toggle_maximize(state: &State, win_id: u64) {
    let need_refocus = state.focused_id != Some(win_id);
    let restore_to = state.focused_id;

    if need_refocus {
        send_action(Action::FocusWindow { id: win_id });
    }

    send_action(Action::MaximizeColumn {});

    if need_refocus {
        if let Some(restore_id) = restore_to {
            if state.windows.contains_key(&restore_id) {
                send_action(Action::FocusWindow { id: restore_id });
            }
        }
    }
}

/// Compare previous and current tiled counts, act on transitions.
/// `moved_window` is Some((win_id, old_workspace_id)) if a window just changed workspaces.
fn reconcile(state: &mut State, moved_window: Option<(u64, Option<u64>)>) {
    let current_counts = state.snapshot_counts();

    // Collect all workspace IDs from both old and new counts.
    let mut all_ws: Vec<u64> = current_counts.keys().copied().collect();
    for &ws_id in state.prev_tiled_count.keys() {
        all_ws.push(ws_id);
    }
    all_ws.sort_unstable();
    all_ws.dedup();

    for ws_id in all_ws {
        let prev = state.prev_tiled_count.get(&ws_id).copied().unwrap_or(0);
        let curr = current_counts.get(&ws_id).copied().unwrap_or(0);

        if prev == curr {
            continue;
        }

        if curr == 1 {
            // Transitioned to exactly 1 tiled window: maximize it if not already.
            if let Some(win_id) = state.sole_tiled_window(ws_id) {
                if !state.is_maximized(win_id) {
                    toggle_maximize(state, win_id);
                }
            }
        } else if prev == 1 && curr > 1 {
            // Went from 1 to many: unmaximize the previously-sole window if it's maximized.
            if let Some(old_win_id) = state.sole_window.get(&ws_id).copied() {
                if state.windows.contains_key(&old_win_id) && state.is_maximized(old_win_id) {
                    toggle_maximize(state, old_win_id);
                }
            }
        }
    }

    // If a window moved to a non-empty workspace and it's maximized, unmaximize it.
    if let Some((win_id, _old_ws)) = moved_window {
        if let Some(info) = state.windows.get(&win_id) {
            if let Some(new_ws) = info.workspace_id {
                let count = current_counts.get(&new_ws).copied().unwrap_or(0);
                if count > 1 && state.is_maximized(win_id) {
                    toggle_maximize(state, win_id);
                }
            }
        }
    }

    // Update sole_window tracking.
    for (&ws_id, &count) in &current_counts {
        if count == 1 {
            if let Some(win_id) = state.sole_tiled_window(ws_id) {
                state.sole_window.insert(ws_id, win_id);
            }
        } else {
            state.sole_window.remove(&ws_id);
        }
    }

    state.prev_tiled_count = current_counts;
}

/// Fetch outputs from niri.
fn fetch_outputs() -> HashMap<String, Output> {
    let mut sock = Socket::connect().expect("Failed to connect to niri socket");
    match sock.send(Request::Outputs) {
        Ok(Ok(Response::Outputs(outputs))) => outputs,
        other => {
            eprintln!("Failed to fetch outputs: {other:?}");
            HashMap::new()
        }
    }
}

/// Fetch workspaces from niri.
fn fetch_workspaces() -> Vec<Workspace> {
    let mut sock = Socket::connect().expect("Failed to connect to niri socket");
    match sock.send(Request::Workspaces) {
        Ok(Ok(Response::Workspaces(ws))) => ws,
        other => {
            eprintln!("Failed to fetch workspaces: {other:?}");
            vec![]
        }
    }
}

fn main() {
    // Fetch initial output and workspace info.
    let outputs = fetch_outputs();
    let workspaces = fetch_workspaces();

    let mut state = State::new();
    state.update_outputs(&outputs);
    state.update_workspaces(&workspaces);

    // Connect event stream.
    let mut event_socket = Socket::connect().expect("Failed to connect to niri socket");
    let reply = event_socket
        .send(Request::EventStream)
        .expect("Failed to request event stream");

    match reply {
        Ok(Response::Handled) => {}
        other => {
            eprintln!("Unexpected reply to EventStream: {other:?}");
            std::process::exit(1);
        }
    }

    let mut read_event = event_socket.read_events();

    loop {
        let event = match read_event() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Event stream error: {e}");
                break;
            }
        };

        match event {
            Event::WindowsChanged { windows } => {
                state.reset_windows(&windows);
                // On full reset, set prev counts to current so we don't
                // spuriously act on startup state.
                state.prev_tiled_count = state.snapshot_counts();
                // Track sole windows for each workspace.
                for (&ws_id, &count) in &state.prev_tiled_count {
                    if count == 1 {
                        if let Some(win_id) = state.sole_tiled_window(ws_id) {
                            state.sole_window.insert(ws_id, win_id);
                        }
                    }
                }
            }
            Event::WindowOpenedOrChanged { window } => {
                let moved = state.upsert_window(&window);
                reconcile(&mut state, moved);
            }
            Event::WindowClosed { id } => {
                state.remove_window(id);
                reconcile(&mut state, None);
            }
            Event::WindowFocusChanged { id } => {
                state.focused_id = id;
            }
            Event::WorkspacesChanged { workspaces } => {
                state.update_workspaces(&workspaces);
                let outputs = fetch_outputs();
                state.update_outputs(&outputs);
            }
            Event::WindowLayoutsChanged { changes } => {
                // Keep tile widths up to date for is_maximized checks.
                for (win_id, layout) in &changes {
                    if let Some(info) = state.windows.get_mut(win_id) {
                        info.tile_width = layout.tile_size.0;
                    }
                }
            }
            _ => {}
        }
    }
}
