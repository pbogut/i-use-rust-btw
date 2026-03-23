use clap::{arg, Command};
use niri_ipc::socket::Socket;
use niri_ipc::{Action, Request, Response, SizeChange, Workspace, WorkspaceReferenceArg};

fn cli() -> Command {
    Command::new("niri-named-workspace")
        .about("Focus or move window to a named workspace in niri")
        .args(vec![
            arg!(<ACTION> "Action to perform: 'focus' or 'move'"),
            arg!(<NAME> "Name of the workspace"),
            arg!(-o --"output" <OUTPUT> "Create named workspace on this output instead of the focused one"),
            arg!(-c --"column-width" <SIZE> "Set column width after moving (e.g. '50%', '800', '+10%')"),
            arg!(-w --"window-width" <SIZE> "Set window width after moving (e.g. '50%', '800', '+10%')"),
        ])
}

fn main() {
    let matches = cli().get_matches();

    let action = matches
        .get_one::<String>("ACTION")
        .expect("Action is required");
    let name = matches.get_one::<String>("NAME").expect("Name is required");
    let output = matches.get_one::<String>("output").cloned();
    let column_width = matches.get_one::<String>("column-width").map(|s| {
        s.parse::<SizeChange>().unwrap_or_else(|e| {
            eprintln!("Invalid --column-width value: {e}");
            std::process::exit(1);
        })
    });
    let window_width = matches.get_one::<String>("window-width").map(|s| {
        s.parse::<SizeChange>().unwrap_or_else(|e| {
            eprintln!("Invalid --window-width value: {e}");
            std::process::exit(1);
        })
    });

    match action.as_str() {
        "focus" => focus_workspace(name, output.as_deref(), column_width, window_width),
        "move" => move_to_workspace(name, output.as_deref(), column_width, window_width),
        other => {
            eprintln!("Unknown action: {other}. Use 'focus' or 'move'.");
            std::process::exit(1);
        }
    }
}

fn fetch_workspaces() -> Vec<Workspace> {
    let mut socket = Socket::connect().expect("Failed to connect to niri socket");
    match socket.send(Request::Workspaces) {
        Ok(Ok(Response::Workspaces(ws))) => ws,
        other => {
            eprintln!("Failed to fetch workspaces: {other:?}");
            vec![]
        }
    }
}

fn send_action(action: Action) {
    let mut socket = Socket::connect().expect("Failed to connect to niri socket");
    if let Err(e) = socket.send(Request::Action(action)) {
        eprintln!("Failed to send action: {e}");
    }
}

/// Find the output of the currently focused workspace.
fn focused_output(workspaces: &[Workspace]) -> Option<String> {
    workspaces
        .iter()
        .find(|ws| ws.is_focused)
        .and_then(|ws| ws.output.clone())
}

/// Find a workspace by name.
fn find_workspace_by_name<'a>(workspaces: &'a [Workspace], name: &str) -> Option<&'a Workspace> {
    workspaces
        .iter()
        .find(|ws| ws.name.as_deref() == Some(name))
}

/// Find the first empty (no active window) unnamed workspace on the given output.
fn find_empty_unnamed_workspace<'a>(
    workspaces: &'a [Workspace],
    output: &str,
) -> Option<&'a Workspace> {
    workspaces
        .iter()
        .filter(|ws| ws.output.as_deref() == Some(output))
        .filter(|ws| ws.name.is_none())
        .find(|ws| ws.active_window_id.is_none())
}

/// Ensure a workspace with the given name exists. If it doesn't, name the first
/// empty unnamed workspace on the specified (or focused) output. Returns true if
/// the workspace exists (or was created), false otherwise.
fn ensure_named_workspace(name: &str, output: Option<&str>) -> bool {
    let workspaces = fetch_workspaces();

    if let Some(ws) = find_workspace_by_name(&workspaces, name) {
        println!("{}", ws.id);
        return true;
    }

    let target_output = match output {
        Some(o) => o.to_string(),
        None => match focused_output(&workspaces) {
            Some(o) => o,
            None => {
                eprintln!("No focused output found");
                return false;
            }
        },
    };

    match find_empty_unnamed_workspace(&workspaces, &target_output) {
        Some(ws) => {
            send_action(Action::SetWorkspaceName {
                name: name.to_string(),
                workspace: Some(WorkspaceReferenceArg::Id(ws.id)),
            });
            println!("{}", ws.id);
            true
        }
        None => {
            eprintln!("No empty unnamed workspace available on output {target_output}");
            false
        }
    }
}

fn apply_size_actions(column_width: Option<SizeChange>, window_width: Option<SizeChange>) {
    if let Some(change) = column_width {
        send_action(Action::SetColumnWidth { change });
    }
    if let Some(change) = window_width {
        send_action(Action::SetWindowWidth { id: None, change });
    }
}

fn focus_workspace(
    name: &str,
    output: Option<&str>,
    column_width: Option<SizeChange>,
    window_width: Option<SizeChange>,
) {
    if !ensure_named_workspace(name, output) {
        return;
    }

    send_action(Action::FocusWorkspace {
        reference: WorkspaceReferenceArg::Name(name.to_string()),
    });

    apply_size_actions(column_width, window_width);
}

fn move_to_workspace(
    name: &str,
    output: Option<&str>,
    column_width: Option<SizeChange>,
    window_width: Option<SizeChange>,
) {
    if !ensure_named_workspace(name, output) {
        return;
    }

    send_action(Action::MoveWindowToWorkspace {
        window_id: None,
        reference: WorkspaceReferenceArg::Name(name.to_string()),
        focus: true,
    });

    apply_size_actions(column_width, window_width);
}
