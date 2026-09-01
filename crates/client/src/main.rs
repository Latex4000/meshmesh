use anyhow::{Result, anyhow, bail};
use protocol::{
    CLIENT_CTX,
    state::{
        ClientWindow::{Direct, Lobby, Room},
        Peer,
    },
};
use rustyline::{DefaultEditor, error::ReadlineError};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    protocol::init().await?;
    command_line().await?;
    Ok(())
}

async fn command_line() -> anyhow::Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut client_window = Lobby;

    loop {
        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
        if let Ok(ctx) = mutex.lock() {
            client_window = ctx.window.clone();
        }

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
                    "peers" => {
                        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
                        if let Ok(ctx) = mutex.lock() {
                            println!("\n Peers: {:?}", ctx.peers.keys().collect::<Vec<_>>())
                        }
                    }
                    "state" => println!("{:?}", CLIENT_CTX.get().unwrap()),
                    "info" => {
                        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
                        if let Ok(ctx) = mutex.lock() {
                            println!("{ctx}")
                        }
                    }
                    "cls" => clearscreen::clear().expect("failed to clear screen"),
                    "direct" => {
                        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
                        if let Ok(mut ctx) = mutex.lock() {
                            match args.join(" ").parse() {
                                Ok(id) if ctx.peers.contains_key(&id) => ctx.window = Direct(id),
                                Ok(id) => println!("You have not discovered peer ID {id}"),
                                Err(e) => println!("Could not connect to ID.\n{e}"),
                            }
                        }
                    }
                    "clearpeers" => {
                        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
                        if let Ok(mut ctx) = mutex.lock() {
                            ctx.peers.clear();
                        }
                    }
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
    if let Some(mutex) = CLIENT_CTX.get()
        && let Ok(ctx) = mutex.lock()
        && let Direct(recipient) = ctx.window
    {
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
    let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
    match cmd {
        "exit" | "quit" => {
            if let Ok(mut ctx) = mutex.lock() {
                ctx.window = Lobby;
            }
        }
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
