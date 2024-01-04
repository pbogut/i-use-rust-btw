fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cwd = std::env::current_dir()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let path = match args.get(1) {
        Some(path) => path,
        None => &cwd,
    };

    let abs_path = cmd_out("git", &["-C", &path, "rev-parse", "--absolute-git-dir"]);
    let top_path = cmd_out("git", &["-C", &path, "rev-parse", "--show-toplevel"]);
    let is_bare = cmd_out("git", &["-C", &path, "rev-parse", "--is-bare-repository"]) == "true";
    let branch_name = cmd_out("git", &["-C", &path, "rev-parse", "--abbrev-ref", "HEAD"]);

    let path = if format!("{}/.git", top_path) == abs_path {
        // simple git repo
        cmd_out("realpath", &[&top_path])
    } else if is_bare {
        // main bare repo folder
        cmd_out("realpath", &[&abs_path])
    } else if abs_path != "" && top_path != "" {
        // branch within bare repo
        cmd_out("realpath", &[&format!("{}/..", top_path)])
    } else {
        path.to_string()
    };

    let base_name = path.split("/").last().unwrap();
    if branch_name != "" {
        println!("{}", format!(" {}   {}", base_name, branch_name));
    } else {
        println!("{}", format!(" {}", base_name));
    }
}

fn cmd_out(cmd: &str, args: &[&str]) -> String {
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()
        .expect("failed to execute process");

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
