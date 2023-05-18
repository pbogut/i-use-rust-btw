use neovim_lib::{Handler, Neovim, RequestHandler, Session, Value};
use std::path::Path;
use std::sync::mpsc;

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

pub fn client(address: &str) -> std::io::Result<Neovim> {
    let mut session = Session::new_unix_socket(Path::new(address))?;

    let (sender, _) = mpsc::channel();
    session.start_event_loop_handler(NeovimHandler(sender));

    Ok(Neovim::new(session))
}
