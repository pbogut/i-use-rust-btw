use crate::Direction;
use neovim_lib::{Handler, Integer, Neovim, NeovimApi, RequestHandler, Session, Value};
use std::path::Path;
use std::sync::mpsc;

impl Direction {
    fn vim(&self) -> &str {
        match self {
            Direction::Left => "h",
            Direction::Right => "l",
            Direction::Up => "k",
            Direction::Down => "j",
        }
    }
}

pub fn focus(id: usize, direction: &Direction) {
    if let Err(_) = switch_window(id, direction) {
        cmd(&current_exe(), &["--skip-nvim", &direction.to_string()]);
    };
}

fn switch_window(id: usize, direction: &Direction) -> Result<(), &str> {
    let user_id = unsafe { libc::getuid() };
    let servername = format!("/run/user/{}/nvim.{}.0", user_id, id);

    if !Path::new(&servername).exists() {
        return Err("Socket don't exists");
    }

    let mut nv = client(&servername);
    let old_winid = get_window_id(&mut nv);
    nv.command(&format!("wincmd {}", direction.vim()))
        .unwrap_or(());
    let new_winid = get_window_id(&mut nv);

    if old_winid == None || new_winid == None {
        Err("Could not get window id")
    } else if old_winid == new_winid {
        Err("Window did not change")
    } else {
        Ok(())
    }
}

fn client(address: &str) -> Neovim {
    let mut session = Session::new_unix_socket(Path::new(address)).unwrap();

    let (sender, _) = mpsc::channel();
    session.start_event_loop_handler(NeovimHandler(sender));

    Neovim::new(session)
}

fn get_window_id(nv: &mut Neovim) -> Option<u64> {
    let id = nv
        .eval("winnr()")
        .unwrap_or(Value::Integer(Integer::from(0)))
        .as_u64()
        .unwrap_or(0);

    if id == 0 {
        None
    } else {
        Some(id)
    }
}

fn current_exe() -> String {
    std::env::current_exe()
        .unwrap_or(Path::new("i3-focus").to_path_buf())
        .to_str()
        .unwrap_or("i3-focus")
        .to_string()
}

fn cmd(cmd: &str, args: &[&str]) {
    std::process::Command::new(cmd).args(args).status().unwrap();
}

enum Event {}

struct NeovimHandler(mpsc::Sender<Event>);

impl Handler for NeovimHandler {
    fn handle_notify(&mut self, name: &str, _args: Vec<Value>) {
        match name {
            _ => {}
        }
    }
}

impl RequestHandler for NeovimHandler {
    fn handle_request(&mut self, _name: &str, _args: Vec<Value>) -> Result<Value, Value> {
        Err(Value::from("not implemented"))
    }
}
