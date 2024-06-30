use clap::{arg, Command};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::mpsc;

#[derive(Debug)]
struct LogLine {
    level: String,
    timestamp: String,
    location: String,
    source: String,
    server: String,
    channel: String,
    message: String,
}

impl LogLine {
    fn from(line: &str) -> Option<Self> {
        let mut parts = line.split('\t');
        let level_time_location = parts.next().unwrap().to_string();
        let mut level_time_location = level_time_location.split(']');

        let level = level_time_location.next().unwrap();
        let level = level[1..].to_string();
        let timestamp = level_time_location.next().unwrap();
        let timestamp = timestamp[1..].to_string();
        let location = level_time_location.next().unwrap();
        let location = location[1..].to_string();
        let source = parts.next()?;
        let source = source[1..source.len() - 1].to_string();
        let server = parts.next()?;
        let server = server[1..server.len() - 1].to_string();
        let channel = parts.next()?;
        let channel = channel[1..channel.len() - 1].to_string();
        let message = parts.next()?;
        let message = message[1..message.len() - 1].to_string();
        let message = message.replace("\\n", "\n").replace("\\t", "\t");

        Some(LogLine {
            level,
            timestamp,
            location,
            source,
            server,
            channel,
            message,
        })
    }
}

fn cli() -> Command {
    Command::new("nvim-lsp-log")
        .about("Print neovim lsp log")
        .args(vec![
            arg!(--"server" <server_name> "Print only output of selected server"),
            arg!(--"log-file" <log_file> "Log file path"),
        ])
}

fn print_line(line: &str, selected_server: &Option<&String>) {
    match LogLine::from(line) {
        Some(log) => {
            if selected_server.is_none() {
                print!("{}: {}", log.server, log.message);
            } else if log.server.contains(selected_server.unwrap()) {
                print!("{}", log.message);
            }
        }
        None => (),
    };
}

fn main() -> notify::Result<()> {
    let matches = cli().get_matches();
    let selected_server = matches.get_one::<String>("server");

    let cwd = std::env::current_dir().unwrap();
    let mut dir = std::env::var("HOME").unwrap_or(cwd.to_str().unwrap().to_string());
    dir.push_str("/.local/state/nvim/lsp.log");

    let log_file = matches.get_one::<String>("log-file").unwrap_or(&dir);

    let path = PathBuf::from(log_file);

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    watcher.watch(&path, RecursiveMode::NonRecursive).unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    let mut pos = contents.len() as u64;

    for line in contents.lines() {
        print_line(line, &selected_server);
    }

    loop {
        match rx.recv() {
            Ok(_event) => {
                let mut f = File::open(&path).unwrap();
                f.seek(SeekFrom::Start(pos)).unwrap();

                pos = f.metadata().unwrap().len();

                let reader = BufReader::new(f);
                for line in reader.lines() {
                    match line {
                        Ok(line) => {
                            print_line(&line, &selected_server);
                        }
                        Err(err) => {
                            eprintln!("Error: {:?}", err);
                            std::process::exit(1);
                        }
                    }
                }
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                std::process::exit(1);
            }
        }
    }
}

