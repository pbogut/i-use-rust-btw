use crate::Direction;
use neovim_lib::{Handler, Integer, Neovim, NeovimApi, RequestHandler, Session, Value};
use std::path::{Path, PathBuf};
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

pub fn id_from_title(name: &str) -> Option<&str> {
    let (_, suffix) = name.split_once(":nvim:")?;
    let (id, _) = suffix.split_once(':')?;
    (!id.is_empty()).then_some(id)
}

pub fn focus(id: impl std::fmt::Display, direction: &Direction) {
    if let Err(_) = switch_window(&id.to_string(), direction) {
        cmd(&current_exe(), &["--skip-nvim", &direction.to_string()]);
    };
}

fn socket_path(
    id: &str,
    runtime_dir: &Path,
    user_id: libc::uid_t,
) -> Result<PathBuf, &'static str> {
    if let Some(compact) = id.strip_prefix("H/") {
        let (session, workspace) = compact.split_once('/').ok_or("Invalid Herdr socket ID")?;
        if session.len() != 16
            || !session.bytes().all(|c| c.is_ascii_hexdigit())
            || workspace.is_empty()
            || !workspace
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err("Invalid Herdr socket ID");
        }
        Ok(runtime_dir
            .join("herdr-nvim")
            .join(session)
            .join(format!("{workspace}.sock")))
    } else if Path::new(id).is_absolute() {
        Ok(PathBuf::from(id))
    } else {
        Ok(PathBuf::from(format!("/run/user/{user_id}/nvim.{id}.0")))
    }
}

fn switch_window(id: &str, direction: &Direction) -> Result<(), &'static str> {
    let user_id = unsafe { libc::getuid() };
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("/run/user/{user_id}")));
    let servername = socket_path(id, &runtime_dir, user_id)?;

    if !servername.exists() {
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

fn client(address: &Path) -> Neovim {
    let mut session = Session::new_unix_socket(address).unwrap();

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

#[allow(unused)]
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

#[cfg(test)]
mod tests {
    use super::{id_from_title, socket_path};
    use std::path::{Path, PathBuf};

    #[test]
    fn parses_nvim_ids_with_herdr_suffix() {
        for id in [
            "123",
            "herdr-w2C",
            "H/0123456789abcdef/w2C",
            "/run/user/1000/herdr-nvim/0123456789abcdef/w2C.sock",
        ] {
            let title = format!("user@host:nvim:{id}:~/project [workspace] |h$");
            assert_eq!(id_from_title(&title), Some(id));
        }
        for title in [
            "zsh [workspace] |h$",
            "user@host:nvim::~/project",
            "user@host:nvim:~/project",
        ] {
            assert_eq!(id_from_title(title), None);
        }
    }

    #[test]
    fn resolves_compact_ids_in_runtime_directory() {
        for runtime in ["/run/user/1000", "/tmp/custom-runtime"] {
            assert_eq!(
                socket_path("H/0123456789abcdef/w2C", Path::new(runtime), 1000).unwrap(),
                Path::new(runtime).join("herdr-nvim/0123456789abcdef/w2C.sock")
            );
        }
    }

    #[test]
    fn preserves_existing_socket_addresses() {
        for id in ["123", "herdr-w2C"] {
            assert_eq!(
                socket_path(id, Path::new("/tmp/custom-runtime"), 1000).unwrap(),
                PathBuf::from(format!("/run/user/1000/nvim.{id}.0"))
            );
        }
        let address = "/run/user/1000/herdr-nvim/0123456789abcdef/w2C.sock";
        assert_eq!(
            socket_path(address, Path::new("/tmp/custom-runtime"), 1000).unwrap(),
            PathBuf::from(address)
        );
    }

    #[test]
    fn rejects_malformed_compact_ids() {
        for id in [
            "H/",
            "H/short/w2C",
            "H/zzzzzzzzzzzzzzzz/w2C",
            "H/0123456789abcdef/",
            "H/0123456789abcdef/..",
            "H/0123456789abcdef/w2C/extra",
        ] {
            assert!(
                socket_path(id, Path::new("/run/user/1000"), 1000).is_err(),
                "{id}"
            );
        }
    }
}
