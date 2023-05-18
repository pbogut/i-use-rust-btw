mod nvim;
use clap::{arg, ArgAction, Command};
use neovim_lib::NeovimApi;

fn cli() -> Command {
    Command::new("nvru")
        .about("Remote control Neovim processes.")
        .args(vec![
            arg!(--servername <addr> "Set the address to be used. Defaults to $NVIM environment variable."),
            arg!(--"server-id" <id> "Set the id to be used."),
            arg!(-c --command <cmd> "Execute a command.").action(ArgAction::Append),
        ])
}

fn main() -> std::io::Result<()> {
    let matches = cli().get_matches();

    let servername = matches.get_one::<String>("servername");
    let server_id = matches.get_one::<String>("server-id");
    let address = match (servername, server_id) {
        (Some(address), _) => address.to_string(),
        (_, Some(id)) => get_address_by_id(id),
        _ => match std::env::var("NVIM") {
            Ok(addr) => addr,
            Err(_) => {
                cli().print_help()?;
                println!("");
                println!("None of the `servername`, `server-id` nor `$NVIM` was provided.");
                std::process::exit(1);
            }
        },
    };

    let mut client = nvim::client(&address)?;

    match matches.get_many::<String>("command") {
        Some(commands) => {
            commands.for_each(|cmd| client.command(cmd).unwrap_or(()));
        }
        None => (),
    };

    Ok(())
}

fn get_address_by_id(id: &String) -> String {
    let user_id = unsafe { libc::getuid() };
    format!("/run/user/{}/nvim.{}.0", user_id, id)
}
