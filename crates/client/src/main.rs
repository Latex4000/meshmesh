use anyhow::{Result, bail};
use protocol::{
    state::{
        ClientWindow::{Direct, Lobby, Room},
        Peer,
    },
    use_ctx,
};
use rustyline::{DefaultEditor, error::ReadlineError};

#[cfg(feature = "gui")]
use dioxus_native::prelude::*;

#[cfg(not(feature = "gui"))]
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    protocol::init().await?;
    command_line().await?;
    Ok(())
}

#[cfg(feature = "gui")]
fn main() {
    tracing_subscriber::fmt::init();
    dioxus_native::launch(App);
}

#[cfg(feature = "gui")]
#[component]
fn App() -> Element {
    rsx! {
        div {
            h1 { "welcome to my app" }
        }
    }
}

async fn command_line() -> anyhow::Result<()> {
    let mut rl = DefaultEditor::new()?;

    loop {
        let client_window = use_ctx(|ctx| ctx.window.clone());

        let recipient_opt = get_recipient();
        let readline_text = match client_window {
            Lobby => "meshmesh (lobby)> ".to_string(),
            Direct(_) => match recipient_opt {
                Some(recipient) => format!("meshmesh (direct to {})> ", recipient),
                None => "meshmesh (direct to UNKNOWN)> ".to_string(),
            },
            Room(_) => "".to_string(),
        };
        let readline = rl.readline(&readline_text);
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    return Ok(());
                }
                rl.add_history_entry(line)?;

                let mut parts = line.split_whitespace();
                let cmd = parts.next().unwrap_or("");
                let args: Vec<&str> = parts.collect();

                match cmd {
                    "help" => println!(
                        "cmds: help, state, peers, ping <peerid/ep>, discover <ep>, join <roomid>, direct <peerid>, pingall, clearpeers, exit"
                    ),
                    "discover" => {
                        if let Err(e) = Peer::discover(&args.join(" ")).await {
                            println!("{e}");
                        }
                    }
                    "ping" => {
                        if let Err(e) = Peer::ping(&args.join(" ")).await {
                            println!("{e}");
                        }
                    }
                    "peers" => use_ctx(|ctx| {
                        println!("\n Peers: {:?}", ctx.peers.keys().collect::<Vec<_>>())
                    }),
                    "state" => use_ctx(|ctx| println!("{:?}", ctx)),
                    "info" => use_ctx(|ctx| println!("{ctx}")),
                    "cls" => clearscreen::clear().expect("failed to clear screen"),
                    "direct" => use_ctx(|ctx| match args.join(" ").parse() {
                        Ok(id) if ctx.peers.contains_key(&id) => ctx.window = Direct(id),
                        Ok(id) => println!("You have not discovered peer ID {id}"),
                        Err(e) => println!("Could not parse ID.\n{e}"),
                    }),
                    "clearpeers" => use_ctx(|ctx| ctx.peers.clear()),
                    _ => match client_window {
                        Lobby => lobby_cmds(line, cmd, args)?,
                        Direct(_) => direct_cmds(line, cmd, args).await?,
                        Room(_) => room_cmds(line, cmd, args)?,
                    },
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                bail!("quit");
            }
            Err(err) => {
                println!("Error: {err}");
            }
        }
    }
}

fn get_recipient() -> Option<u8> {
    if let Direct(recipient) = use_ctx(|ctx| ctx.window.clone()) {
        Some(recipient)
    } else {
        None
    }
}

#[allow(unused_variables)]
fn lobby_cmds(line: &str, cmd: &str, args: Vec<&str>) -> anyhow::Result<()> {
    Ok(())
}
#[allow(unused_variables)]
async fn direct_cmds(line: &str, cmd: &str, args: Vec<&str>) -> anyhow::Result<()> {
    match cmd {
        "exit" | "quit" => use_ctx(|ctx| ctx.window = Lobby),
        _ => {
            if let Some(recipient) = get_recipient()
                && let Err(e) = Peer::send_to(recipient, line.to_string()).await
            {
                println!("{e}");
            }
        }
    }
    Ok(())
}
#[allow(unused_variables)]
fn room_cmds(line: &str, cmd: &str, args: Vec<&str>) -> anyhow::Result<()> {
    todo!();
}

#[expect(dead_code)]
#[allow(unused_variables)]
fn ping_peer(id: u8) -> anyhow::Result<()> {
    todo!()
}
