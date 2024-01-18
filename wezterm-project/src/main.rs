use glob;
use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Session {
    cwd: String,
    pid: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Sessions {
    sessions: Vec<Session>,
}

impl Sessions {
    fn session_file_path(&self) -> String {
        let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".into());
        format!("{}/wezterm_sessions.json", dir)
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
            let shell = std::env::var("SHELL").expect("Can not get shell");
            let command =
                format!("while :; do clear; nvim; echo '[enter] nvim\n[ctr+c] quit'; read; done",);

            let args: Vec<String> = env::args().collect();
            let mut project = "";
            if args.len() > 2 {
                project = &args[2];
            }

            let pid = cmdpid(
                "env",
                &[
                    &format!("WEZTERM_PROJECT={project}"),
                    "wezterm",
                    "start",
                    "--always-new-process",
                    "--cwd",
                    path,
                    &shell,
                    "-ic",
                    &command,
                ],
            );
            let session = Session {
                cwd: path.into(),
                pid,
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
        cmd(
            "swaymsg",
            &[&format!(r#"[title=" \|w\${}:"] focus"#, self.pid)],
        );
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut sessions = get_sessions();
    if args.len() > 1 {
        let path = &args[1];
        sessions.for_path(path).focus();
    }
}

fn get_client_pids() -> Vec<usize> {
    let socket = std::env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".into());
    let pattern = format!("{socket}/wezterm/gui-sock-*");

    let running_pids = cmd("pgrep", &["wezterm-gui"]);
    if running_pids.is_empty() {
        return vec![];
    }

    let running_pids = running_pids
        .trim()
        .split('\n')
        .map(|pid| pid.parse::<usize>().expect("Can not parse pid"))
        .collect::<Vec<usize>>();

    glob::glob(&pattern)
        .expect("Can not get list of wezterm sockets")
        .map(|p| {
            let x = p.expect("Can not get path");
            let pid_str = x.to_str().expect("Can not convert path to str");
            let pid_str = pid_str
                .split('-')
                .last()
                .expect("Can not get pid part from socket");

            pid_str
                .parse::<usize>()
                .expect("Can't convert socket pid to usize")
        })
        .filter(|pid| {
            for running_pid in &running_pids {
                if *running_pid == *pid {
                    return true;
                }
            }
            false
        })
        .collect::<Vec<usize>>()
}

fn get_sessions() -> Sessions {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or("/tmp".into());
    let file_path = format!("{}/wezterm_sessions.json", dir);

    let json_string = std::fs::read_to_string(file_path).unwrap_or_else(|_| String::from("[]"));

    let client_pids = get_client_pids();

    let sessions = serde_json::from_str::<Vec<Session>>(&json_string)
        .unwrap_or_default()
        .iter()
        .filter(|session| {
            for client_pid in &client_pids {
                if *client_pid == session.pid {
                    return true;
                }
            }
            false
        })
        .cloned()
        .collect::<Vec<Session>>();

    let result = Sessions { sessions };
    result.save().expect("Can not save session file");
    result
}

fn cmd(cmd: &str, args: &[&str]) -> String {
    let o = std::process::Command::new(cmd)
        .args(args)
        .output()
        .expect("Can not run command");
    String::from_utf8(o.stdout).expect("Can not get stdout from cmd")
}

fn cmdpid(cmd: &str, args: &[&str]) -> usize {
    let child = std::process::Command::new(cmd)
        .args(args)
        .spawn()
        .expect("Can not run command");
    child.id() as usize
}
