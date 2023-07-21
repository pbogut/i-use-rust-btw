use clap::{arg, Command};
use swayipc::Connection;

enum Direction {
    Next,
    Prev,
}

fn cli() -> Command {
    Command::new("sway-workspace")
        .about("Manage workspaces in sway")
        .args(vec![
            arg!(--"same-output" "Stay on the same output"),
            arg!(--"next" "Swich to the next workspace (default)"),
            arg!(--"prev" "Swich to the prev workspace"),
            arg!(--"move" "Move currently focused container"),
            arg!(--"send" "Sends currently focused container"),
            arg!(--"new" "Create new workspace"),
            arg!(--"start-idx" <index> "Minimum index to start from when creating new workspace"),
        ])
}

fn main() -> Result<(), swayipc::Error> {
    let matches = cli().get_matches();
    let mut sway = swayipc::Connection::new().expect("Can not connect to sway ipc");

    let do_move = matches.get_flag("move");
    let do_send = matches.get_flag("send");
    let prev = matches.get_flag("prev");
    let same_output = matches.get_flag("same-output");

    let mut direction = Direction::Next;
    if prev {
        direction = Direction::Prev;
    };

    if matches.get_flag("new") {
        let start_idx = matches
            .get_one::<String>("start-idx")
            .map_or(1, |idx| idx.parse().expect("Index has to be number"));
        create_workspace(&mut sway, start_idx, do_move, do_send)?;
    } else {
        switch_workspace(&mut sway, &direction, do_move, do_send, same_output)?;
    }

    Ok(())
}

fn switch_workspace(
    sway: &mut Connection,
    direction: &Direction,
    do_move: bool,
    do_send: bool,
    same_output: bool,
) -> Result<(), swayipc::Error> {
    let mut workspaces = sway.get_workspaces().map_or(vec![], |ws| ws);

    //allow for cycle through
    workspaces.extend(workspaces.clone());

    match direction {
        Direction::Prev => workspaces.reverse(),
        Direction::Next => (),
    };

    let mut focused_output: Option<String> = None;
    let mut switch_to = false;

    for ws in workspaces {
        if same_output
            && focused_output.is_some()
            && ws.output != focused_output.clone().expect("No focused output")
        {
            continue;
        }
        if switch_to {
            if do_move || do_send {
                sway.run_command(format!("move container to workspace {}", ws.name))?;
            }
            if !do_send {
                sway.run_command(format!("workspace {}", ws.name))?;
            }
            break;
        }
        if ws.focused {
            focused_output = Some(ws.output);
            switch_to = true;
            continue;
        }
        if ws.focused {
            if do_move || do_send {
                sway.run_command(format!("move container to workspace {}", ws.name))?;
            }
            if !do_send {
                sway.run_command(format!("workspace {}", ws.name))?;
            }
            break;
        }
    }

    Ok(())
}

fn create_workspace(
    sway: &mut Connection,
    start_idx: i32,
    do_move: bool,
    do_send: bool,
) -> Result<(), swayipc::Error> {
    let workspaces = sway.get_workspaces().map_or(vec![], |ws| ws);
    let num_workspaces = workspaces
        .iter()
        .filter_map(|ws| {
            if ws.num.to_string() == ws.name {
                Some(ws.num)
            } else {
                None
            }
        })
        .collect::<Vec<i32>>();

    for new_num in start_idx..100 + start_idx {
        if !num_workspaces.contains(&new_num) {
            if do_move || do_send {
                sway.run_command(format!("move container to workspace {new_num}"))?;
            }
            if !do_send {
                sway.run_command(format!("workspace {new_num}"))?;
            }
            break;
        }
    }

    Ok(())
}
