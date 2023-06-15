use clap::{arg, ArgGroup, ArgMatches, Command};
use std::time::{SystemTime, UNIX_EPOCH};

fn cli() -> Command {
    Command::new("tmux-kill-idle-sessions")
        .about("Kill tmux idle sessions")
        .group(
            ArgGroup::new("timeout")
                .args(["seconds", "minutes", "hours"])
                .required(true),
        )
        .args(vec![
            arg!(-S --"seconds" <seconds> "idle time in seconds"),
            arg!(-M --"minutes" <minutes> "idle time in minutes"),
            arg!(-H --"hours" <hours> "idle time in hours"),
            arg!(--"dry-run" "dry run"),
        ])
}

fn main() {
    let matches = cli().get_matches();

    let session_list = cmd_out(
        "tmux",
        &["list-sessions", "-F", "#{session_id}:#{session_activity}"],
    );

    let dry_run = matches.get_flag("dry-run");

    let idle_time = get_idle_time(matches);
    println!("Killing sessions that are not active for: {}s", idle_time);
    let mut killed = false;

    session_list.split("\n").for_each(|session| {
        let parts: Vec<&str> = session.splitn(2, ":").collect();
        let session_id = parts[0];
        let last_activity = parts[1].parse::<u64>().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let difference = now - last_activity;

        if difference > idle_time {
            if !dry_run {
                println!("Killing session with id: {}", session_id);
                cmd("tmux", &["kill-session", "-t", session_id]);
            } else {
                println!("Killing session with id: {} [SKIPPED:DRY_RUN]", session_id);
            }
            killed = true
        }
    });

    if killed {
        println!("Done");
    } else {
        println!("Nothing to be killed");
    }
}

fn get_idle_time(matches: ArgMatches) -> u64 {
    let secs = matches
        .get_one::<String>("seconds")
        .unwrap_or(&String::from("0"))
        .parse::<u64>()
        .expect("Timeout has to be a number");

    let mins = matches
        .get_one::<String>("minutes")
        .unwrap_or(&String::from("0"))
        .parse::<u64>()
        .expect("Timeout has to be a number");

    let hours = matches
        .get_one::<String>("hours")
        .unwrap_or(&String::from("0"))
        .parse::<u64>()
        .expect("Timeout has to be a number");

    match (secs, mins, hours) {
        (0, 0, 0) => panic!("Timeout has to be provided"),
        (secs, 0, 0) => secs,
        (0, mins, 0) => mins * 60,
        (0, 0, hours) => hours * 60 * 60,
        (_, _, _) => panic!("Timeout has to be a number"),
    }
}

fn cmd_out(cmd: &str, args: &[&str]) -> String {
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()
        .expect("failed to execute process");

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn cmd(cmd: &str, args: &[&str]) -> String {
    let o = std::process::Command::new(cmd).args(args).output().unwrap();
    String::from_utf8(o.stdout).unwrap()
}
