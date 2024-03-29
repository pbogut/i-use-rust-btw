use std::fs::File;
use std::io::{Result, Write};

fn main() -> Result<()> {
    let full_path = get_full_path();

    println!("New repository path: {}", full_path);

    if std::path::Path::new(&full_path).exists() {
        println!("Path already exists");
        std::process::exit(1);
    }

    mkdir_p(&full_path);
    cd(&full_path);
    cmd("git", &["init", "--bare", ".bare"]);
    File::create(".git")?.write_all(b"gitdir: ./.bare")?;
    cmd("git", &["worktree", "add", "master", "--orphan"]);
    cmd(
        "git",
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ],
    );

    Ok(())
}

fn cmd(cmd: &str, args: &[&str]) {
    println!("Running: {} {}", cmd, args.join(" "));
    std::process::Command::new(cmd).args(args).status().unwrap();
}

fn cd(path: &str) {
    std::env::set_current_dir(path).unwrap();
}

fn mkdir_p(path: &str) {
    std::fs::create_dir_all(path).unwrap();
}

fn get_args() -> (String, String, String) {
    let args: Vec<String> = std::env::args().collect();
    let repo = match args.get(1) {
        Some(repo) => repo,
        None => {
            println!("No repo specified");
            std::process::exit(1);
        }
    };

    let mut parts = repo.splitn(3, "/").collect::<Vec<&str>>();

    let user = std::env::var("USER").unwrap_or("user".to_string());

    let name = parts.pop().unwrap();
    let org = parts.pop().unwrap_or(&user);
    let domain = parts.pop().unwrap_or("github.com");

    (name.to_string(), org.to_string(), domain.to_string())
}

fn get_full_path() -> String {
    let cwd = std::env::current_dir().unwrap();
    let dir = std::env::var("PROJECTS").unwrap_or(cwd.to_str().unwrap().to_string());

    let (name, org, domain) = get_args();

    format!("{}/{}/{}/{}", dir, domain, org, name)
}
