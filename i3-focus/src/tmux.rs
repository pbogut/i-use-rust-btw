use crate::Direction;

impl Direction {
    fn tmux(&self) -> &str {
        match self {
            Direction::Left => "-L",
            Direction::Right => "-R",
            Direction::Up => "-U",
            Direction::Down => "-D",
        }
    }
}

pub fn focus(id: usize, direction: &Direction) {
    let target = format!("${}", id);
    cmd("tmux", &["select-pane", "-t", &target, direction.tmux()]);
}

pub fn is_tmux_edge(id: usize, direction: &Direction) -> bool {
    match maybe_is_tmux_edge(id, direction) {
        Some(true) => true,
        _ => false,
    }
}

fn maybe_is_tmux_edge(id: usize, direction: &Direction) -> Option<bool> {
    let offset = tmux_offset(id, direction)?;
    Some(match direction {
        Direction::Right => tmux_width(id)? - offset == 1,
        Direction::Left => offset == 0,
        Direction::Up => offset == 0,
        Direction::Down => tmux_height(id)? - offset == 1,
    })
}

fn tmux_width(id: usize) -> Option<usize> {
    let format = "#{window_width}";
    tmux_active_pane_format(id, format)
}

fn tmux_height(id: usize) -> Option<usize> {
    let format = "#{window_height}";
    tmux_active_pane_format(id, format)
}

fn tmux_offset(id: usize, direction: &Direction) -> Option<usize> {
    let format = match direction {
        Direction::Left => "#{pane_left}",
        Direction::Right => "#{pane_right}",
        Direction::Up => "#{pane_top}",
        Direction::Down => "#{pane_bottom}",
    };

    tmux_active_pane_format(id, format)
}

fn tmux_active_pane_format(id: usize, format: &str) -> Option<usize> {
    let separator = "::";
    let target = format!("${}", id);
    let output = cmd(
        "tmux",
        &[
            "list-panes",
            "-t",
            &target,
            "-F",
            &format!("{}{}#{{pane_active}}", format, separator),
        ],
    );
    let value = output
        .lines()
        .find(|line| match line.split(separator).last() {
            Some("1") => true,
            _ => false,
        })?
        .split(separator)
        .nth(0)?
        .parse::<usize>();

    match value {
        Ok(result) => Some(result),
        Err(_) => None,
    }
}

fn cmd(cmd: &str, args: &[&str]) -> String {
    let o = std::process::Command::new(cmd).args(args).output().unwrap();
    String::from_utf8(o.stdout).unwrap()
}
