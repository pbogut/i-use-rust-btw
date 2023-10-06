use std::{io::Result, process::ExitStatus};

fn main() -> Result<()> {
    let full_path = get_full_path();
    let repo_uri = get_repository_uri();

    git(&["clone", "--bare", &repo_uri, &format!("{full_path}/.bare")]);
    cd(&full_path);
    std::fs::write(format!("{full_path}/.git"), format!("gitdir: ./.bare"))?;
    git(&[
        "config",
        "remote.origin.fetch",
        "+refs/heads/*:refs/remotes/origin/*",
    ]);
    let main_branch = std::fs::read_to_string(".bare/HEAD")?;
    let main_branch = main_branch.trim().split('/').last().unwrap();
    git(&["worktree", "add", &format!("{full_path}/{main_branch}")]);
    git(&["fetch"]);
    git(&["branch", &format!("--set-upstream-to=origin/{main_branch}")]);

    Ok(())
}

fn git(args: &[&str]) -> ExitStatus {
    match cmd("git", args) {
        status if status.success() => status,
        status => {
            println!("Command failed: {:?}", status.code().unwrap_or(1));
            std::process::exit(status.code().unwrap_or(1));
        }
    }
}

fn cmd(cmd: &str, args: &[&str]) -> ExitStatus {
    println!("Running: {} {}", cmd, args.join(" "));
    std::process::Command::new(cmd).args(args).status().unwrap()
}

fn cd(path: &str) {
    std::env::set_current_dir(path).unwrap();
}

fn to_repository_uri(repo_path: &str) -> String {
    if repo_path.matches("@").count() != 0 {
        return repo_path.to_string();
    }
    if !is_known_repository(repo_path) {
        return repo_path.to_string();
    }

    let (domain, vendor, repo) = arg_to_parts(repo_path);
    return format!("git@{}:{}/{}", domain, vendor, repo);
}

fn get_repository_uri() -> String {
    let args: Vec<String> = std::env::args().collect();
    let repo = match args.get(1) {
        Some(repo) => repo,
        None => {
            println!("No repo specified");
            std::process::exit(1);
        }
    };

    to_repository_uri(repo)
}

fn known_repos() -> [&'static str; 3] {
    ["github.com", "gitlab.com", "bitbucket.com"]
}

fn is_known_repository(repo: &str) -> bool {
    for known_domain in known_repos() {
        if repo.starts_with(&(String::from("https://") + known_domain))
            || repo.starts_with(&(String::from("http://") + known_domain))
            || repo.starts_with(known_domain)
        {
            return true;
        }
    }
    if repo.starts_with("https://") || repo.starts_with("http://") || repo.contains("@") {
        return false;
    }

    return true;
}

fn arg_to_parts(repo_path: &str) -> (String, String, String) {
    let striped = repo_path
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .replace(':', "/");

    let mut parts = striped.split("/").collect::<Vec<&str>>();

    let user = std::env::var("USER").unwrap_or("user".to_string());

    let repo = parts.pop().unwrap();
    let vendor = parts.pop().unwrap_or(&user);
    let domain = parts.pop().unwrap_or("github.com").to_string();

    (domain, vendor.to_string(), repo.to_string())
}

fn to_full_path(repo: &str) -> String {
    let cwd = std::env::current_dir().unwrap();
    let dir = std::env::var("PROJECTS").unwrap_or(cwd.to_str().unwrap().to_string());

    if !is_known_repository(repo) {
        let path = repo
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .replace(':', "/")
            .split("@")
            .last()
            .unwrap()
            .to_lowercase();

        return dir + "/" + &path;
    }

    let (mut domain, org, name) = arg_to_parts(&repo);

    if domain.contains("@") {
        domain = domain.splitn(2, "@").collect::<Vec<&str>>()[1].to_string();
    }

    format!(
        "{}/{}/{}/{}",
        dir,
        domain.to_lowercase(),
        org.to_lowercase(),
        name.to_lowercase()
    )
}

fn get_full_path() -> String {
    let args: Vec<String> = std::env::args().collect();
    let repo = match args.get(1) {
        Some(repo) => repo,
        None => {
            println!("No repo specified");
            std::process::exit(1);
        }
    };

    to_full_path(repo)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_getting_path_from_argument_only_repo_name() {
        let user = std::env::var("USER").unwrap_or("user".to_string());
        let result = to_repository_uri("repo-name");

        assert_eq!(
            result,
            format!("git@{}:{}/{}", "github.com", user, "repo-name")
        );
    }

    #[test]
    fn test_getting_path_from_argument_vendor_slsh_repo_name() {
        let result = to_repository_uri("someone/repo-name");

        assert_eq!(
            result,
            format!("git@{}:{}/{}", "github.com", "someone", "repo-name")
        );
    }

    #[test]
    fn test_getting_path_from_argument_domain_vendor_slsh_repo_name() {
        let result = to_repository_uri("gitlab.com/someone/repo-name");

        assert_eq!(
            result,
            format!("git@{}:{}/{}", "gitlab.com", "someone", "repo-name")
        );
    }

    #[test]
    fn test_getting_path_from_argument_full_uri_with_schema() {
        let result = to_repository_uri("https://gitlab.com/someone/repo-name");

        assert_eq!(
            result,
            format!("git@{}:{}/{}", "gitlab.com", "someone", "repo-name")
        );
    }

    #[test]
    fn test_getting_path_from_argument_full_uri_without_schema_with_user() {
        let result = to_repository_uri("me@mylab.com/someone/repo-name");

        assert_eq!(result, "me@mylab.com/someone/repo-name");
    }

    #[test]
    fn test_getting_folder_from_repo_with_at_sign() {
        let cwd = std::env::current_dir().unwrap();
        let dir = std::env::var("PROJECTS").unwrap_or(cwd.to_str().unwrap().to_string());
        let result = to_full_path("me@mylab.com/SomeOne/repo-name");

        assert_eq!(result, dir + "/mylab.com/someone/repo-name");
    }

    #[test]
    fn test_getting_path_from_personal_git_url_with_no_vendor() {
        let cwd = std::env::current_dir().unwrap();
        let dir = std::env::var("PROJECTS").unwrap_or(cwd.to_str().unwrap().to_string());
        let result = to_full_path("https://my.git.com/Repo-Name");

        assert_eq!(result, dir + "/my.git.com/repo-name");
    }

    #[test]
    fn test_getting_folder_from_personal_git_url_with_no_vendor() {
        let result = to_repository_uri("https://my.git.com/repo-name");

        assert_eq!(result, "https://my.git.com/repo-name");
    }
}
