use clap::{builder::PossibleValue, ValueEnum};

pub mod nvim;
pub mod tmux;

#[derive(Clone)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl ValueEnum for Direction {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Left, Self::Right, Self::Up, Self::Down]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::Left => PossibleValue::new("left"),
            Self::Right => PossibleValue::new("right"),
            Self::Up => PossibleValue::new("up"),
            Self::Down => PossibleValue::new("down"),
        })
    }
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}
