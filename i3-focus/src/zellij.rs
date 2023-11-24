use crate::Direction;

impl Direction {
    fn zellij(&self) -> &str {
        match self {
            Direction::Left => "left",
            Direction::Right => "right",
            Direction::Up => "up",
            Direction::Down => "down",
        }
    }
}

pub fn focus(id: &str, direction: &Direction) -> bool {
    let before = cmd("zellij", &["-s", id, "action", "dump-layout"]);
    cmd(
        "zellij",
        &["-s", id, "action", "move-focus", direction.zellij()],
    );
    let after = cmd("zellij", &["-s", id, "action", "dump-layout"]);

    before != after
}

fn cmd(cmd: &str, args: &[&str]) -> String {
    let o = std::process::Command::new(cmd).args(args).output().unwrap();
    String::from_utf8(o.stdout).unwrap()
}
