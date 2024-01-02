use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
};

use serde::{Deserialize, Serialize};

// #[derive(Clone, Debug, Deserialize)]
// struct Size {
//     rows: i32,
//     cols: i32,
//     pixel_width: i32,
//     pixel_height: i32,
//     dpi: i32,
// }

#[derive(Clone, Debug, Deserialize)]
struct Client {
    pid: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct Pane {
    window_id: i32,
    // tab_id: i32,
    pane_id: i32,
    // workspace: String,
    // size: Size,
    // title: String,
    // cwd: String,
    // cursor_x: i32,
    // cursor_y: i32,
    // cursor_shape: String,
    // cursor_visibility: String,
    // left_col: i32,
    // top_row: i32,
    // tab_title: String,
    // window_title: String,
    // is_active: bool,
    // is_zoomed: bool,
    // tty_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Session {
    cwd: String,
    window_id: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Sessions {
    pid: usize,
    sessions: Vec<Session>,
}

impl Sessions {
    fn session_file_path(&self) -> String {
        format!("/tmp/wezterm_sessions_{}.json", self.pid)
    }

    fn save(&self) -> std::io::Result<()> {
        let file = File::create(self.session_file_path())?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &self.sessions)?;
        writer.flush()?;
        Ok(())
    }

    fn for_path(&mut self, path: &str) -> Session {
        let sessions = self
            .sessions
            .iter()
            .filter(|session| session.cwd == path)
            .cloned()
            .collect::<Vec<Session>>();

        if sessions.is_empty() {
            // will it work for first instance, or get blocked?
            let response = cmderr("wezterm", &["start", "--cwd", path]);
            let session = Session {
                cwd: path.into(),
                window_id: get_window_id_from_response(&response),
            };

            self.sessions.push(session.clone());
            self.save().expect("Can not save session file");

            session
        } else {
            sessions[0].clone()
        }
    }
}

impl Session {
    fn focus(&self) {
        for pane in get_panes_for_window_id(self.window_id).unwrap_or_default() {
            let pane_id = pane.pane_id;
            // TODO: stop on first success
            cmd(
                "swaymsg",
                &[&format!(r#"[title=" \|w\${pane_id}$"] focus"#)],
            );
        }
    }
}

fn get_window_id_from_response(response: &str) -> i32 {
    let mut parts = response.splitn(2, "window_id: ");
    parts.next();
    let win_part = parts
        .next()
        .expect("Can not extract window_id form response [1]");
    let mut parts = win_part.splitn(2, ',');
    parts
        .next()
        .expect("Can not extract window_id form response [2]")
        .parse::<i32>()
        .expect("Can not parse window_id form response")
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if let Some(pid) = get_default_pid() {
        let mut sessions = get_sessions(pid);
        if args.len() > 1 {
            let path = &args[1];
            sessions.for_path(path).focus();
        }
    };
}

fn get_sessions(pid: usize) -> Sessions {
    let file_path = format!("/tmp/wezterm_sessions_{pid}.json");
    let json_string = std::fs::read_to_string(file_path).unwrap_or_else(|_| String::from("[]"));

    let window_ids = get_panes()
        .unwrap_or_default()
        .iter()
        .map(|pane| pane.window_id)
        .collect::<Vec<i32>>();

    let sessions = serde_json::from_str::<Vec<Session>>(&json_string)
        .unwrap_or_default()
        .iter()
        .filter(|session| {
            for window_id in &window_ids {
                if *window_id == session.window_id {
                    return true;
                }
            }
            false
        })
        .cloned()
        .collect::<Vec<Session>>();

    let result = Sessions { pid, sessions };
    result.save().expect("Can not save session file");
    result
}

fn get_default_pid() -> Option<usize> {
    let json_string = cmd("wezterm", &["cli", "list-clients", "--format", "json"]);
    let json_list = serde_json::from_str::<Vec<Client>>(&json_string).ok()?;

    if json_list.is_empty() {
        None
    } else {
        Some(json_list[0].pid)
    }
}

fn get_panes_for_window_id(window_id: i32) -> Option<Vec<Pane>> {
    let panes = get_panes()?
        .iter()
        .filter(|pane| pane.window_id == window_id)
        .cloned()
        .collect::<Vec<Pane>>();

    Some(panes)
}

fn get_panes() -> Option<Vec<Pane>> {
    let json_string = cmd("wezterm", &["cli", "list", "--format", "json"]);
    serde_json::from_str::<Vec<Pane>>(&json_string).ok()
}

fn cmd(cmd: &str, args: &[&str]) -> String {
    let o = std::process::Command::new(cmd)
        .args(args)
        .output()
        .expect("Can not run command");
    String::from_utf8(o.stdout).expect("Can not get stdout from cmd")
}

fn cmderr(cmd: &str, args: &[&str]) -> String {
    let o = std::process::Command::new(cmd)
        .args(args)
        .output()
        .expect("Can not run command");
    String::from_utf8(o.stderr).expect("Can not get stderr from cmd")
}
