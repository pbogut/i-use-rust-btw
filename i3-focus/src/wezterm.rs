use crate::Direction;

impl Direction {
    fn wezterm(&self) -> &str {
        match self {
            Direction::Left => "Left",
            Direction::Right => "Right",
            Direction::Up => "Up",
            Direction::Down => "Down",
        }
    }
    fn wezterm_shortcut(&self) -> &str {
        match self {
            Direction::Left => "h",
            Direction::Right => "l",
            Direction::Up => "k",
            Direction::Down => "j",
        }
    }
}

pub fn focus(id: usize, direction: &Direction) -> bool {
    let pane = cmd(
        "wezterm",
        &[
            "cli",
            "get-pane-direction",
            direction.wezterm(),
            "--pane-id",
            &format!("{id}"),
        ],
    );

    match pane.trim().parse::<usize>() {
        Ok(pane_id) => {
            if pane_id != id {
                // this one is not refreshing wezterm right away :(
                // cmd(
                //     "wezterm",
                //     &["cli", "activate-pane", "--pane-id", &format!("{pane_id}")],
                // );

                // use shortcuts as hack to force refresh
                cmd(
                    "wtype",
                    &[
                        "-M",
                        "ctrl",
                        "-M",
                        "alt",
                        "-P",
                        direction.wezterm_shortcut(),
                        "-p",
                        direction.wezterm_shortcut(),
                        "-m",
                        "alt",
                        "-m",
                        "ctrl",
                    ],
                );
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

fn cmd(cmd: &str, args: &[&str]) -> String {
    let o = std::process::Command::new(cmd).args(args).output().unwrap();
    String::from_utf8(o.stdout).unwrap()
}
