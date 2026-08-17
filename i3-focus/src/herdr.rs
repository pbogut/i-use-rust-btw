use crate::Direction;
use serde_json::Value;
use std::process::Command;

pub fn focus(direction: &Direction) -> bool {
    let output = match Command::new("herdr")
        .args(["pane", "focus", "--direction", &direction.to_string()])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return false,
    };

    focus_changed(&output.stdout)
}

fn focus_changed(output: &[u8]) -> bool {
    serde_json::from_slice::<Value>(output)
        .ok()
        .and_then(|value| value.pointer("/result/focus/changed")?.as_bool())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::focus_changed;

    #[test]
    fn detects_changed_focus() {
        assert!(focus_changed(
            br#"{"result":{"type":"pane_focus_direction","focus":{"changed":true}}}"#
        ));
    }

    #[test]
    fn rejects_unchanged_or_invalid_responses() {
        assert!(!focus_changed(
            br#"{"result":{"type":"pane_focus_direction","focus":{"changed":false}}}"#
        ));
        assert!(!focus_changed(br#"{"error":{"code":"not_found"}}"#));
        assert!(!focus_changed(b"not json"));
    }
}
