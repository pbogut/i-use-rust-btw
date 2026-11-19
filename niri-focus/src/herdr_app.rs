use i3_focus::{nvim, Direction};
use serde_json::Value;
use std::process::Command;

fn request(program: &str, args: &[&str]) -> Option<Value> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

fn focused_pane() -> Option<String> {
    Some(
        request("herdr-app", &["status"])?["focused_pane"]
            .as_str()?
            .to_owned(),
    )
}

fn local_terminal<'a>(world: &'a Value, pane: &str) -> Option<&'a str> {
    let space = world["spaces"]
        .as_array()?
        .iter()
        .find(|s| s["active"] == true)?;
    if !space["machine"].is_null() {
        return None;
    }
    let tab = space["tabs"]
        .as_array()?
        .iter()
        .find(|t| t["active"] == true)?;
    let terminal = tab["terminals"]
        .as_array()?
        .iter()
        .find(|t| t["pane"] == pane)?;
    terminal["terminal"].as_str()
}

fn nvim_socket(terminal: &Value) -> Option<&str> {
    let foreground = &terminal["foreground"];
    if foreground["name"] != "nvim" {
        return None;
    }
    if let Some(argv) = foreground["argv"].as_array() {
        if let Some(address) = argv.windows(2).find(|pair| pair[0] == "--listen") {
            if let Some(path) = address[1].as_str().filter(|s| s.starts_with('/')) {
                return Some(path);
            }
        }
    }
    nvim::id_from_title(terminal["title"].as_str()?)
}

fn focus_nvim(pane: &str, direction: &Direction) -> Option<()> {
    // status.agent is the agent shown in the UI, not necessarily the focused terminal.
    let world = request("herdr-app", &["ls"])?;
    let terminal = request("hc", &["get", local_terminal(&world, pane)?])?;
    nvim::switch_window(nvim_socket(&terminal)?, direction).ok()
}

pub fn focus(direction: &Direction, skip_nvim: bool) -> bool {
    let Some(before) = focused_pane() else {
        return false;
    };
    if !skip_nvim && focus_nvim(&before, direction).is_some() {
        return true;
    }
    // pane.go also returns ok at an edge.
    let reply = request(
        "herdr-app",
        &["do", "pane.go", "--direction", &direction.to_string()],
    );
    reply.is_some_and(|v| v["ok"] == true) && focused_pane().is_some_and(|after| after != before)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn looks_up_the_focused_pane_only_in_the_active_local_tab() {
        let mut world = json!({"spaces": [{"active":true, "machine":null, "tabs":[
            {"active":false, "terminals":[{"pane":"p3", "terminal":"hidden"}]},
            {"active":true, "terminals":[{"pane":"p1", "terminal":"t1"}, {"pane":"p2", "terminal":"t2"}]}
        ]}]});
        assert_eq!(local_terminal(&world, "p2"), Some("t2"));
        assert_eq!(local_terminal(&world, "p3"), None);
        world["spaces"][0]["machine"] = json!("vps");
        assert_eq!(local_terminal(&world, "p2"), None);
    }

    #[test]
    fn finds_the_foreground_editor_and_ignores_a_stale_title() {
        let mut terminal = json!({
            "title":"user@host:nvim:123:project",
            "foreground":{"name":"nvim", "argv":["nvim"]}
        });
        assert_eq!(nvim_socket(&terminal), Some("123"));
        terminal["foreground"]["argv"] = json!(["nvim", "--listen", "/tmp/editor.sock"]);
        assert_eq!(nvim_socket(&terminal), Some("/tmp/editor.sock"));
        terminal["foreground"]["name"] = json!("zsh");
        assert_eq!(nvim_socket(&terminal), None);
    }
}
