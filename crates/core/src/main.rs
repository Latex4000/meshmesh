use anyhow::{Result, anyhow, bail};
use client::state::{
    ClientState::{self, Direct, Lobby},
    Context,
};
use log::info;
use protocol::state::Peer;
use rustyline::{DefaultEditor, Editor, error::ReadlineError, history::FileHistory};

use std::sync::{LazyLock, Mutex, OnceLock};
//static PROTO_CTX: LazyLock<Context> = LazyLock::new(|| {});
static CLIENT_CTX: OnceLock<Mutex<Context>> = OnceLock::new();
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let peer = protocol::init().await?;

    let ctx = Context {
        state: client::state::ClientState::Lobby,
        local: peer,
        peers: Vec::new(),
    };
    CLIENT_CTX.set(Mutex::new(ctx)).unwrap();
    command_line().await?;
    Ok(())
}

async fn command_line() -> anyhow::Result<()> {
    let mut rl = DefaultEditor::new()?;

    loop {
        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
        if let Ok(ctx) = mutex.lock() {
            match ctx.state {
                client::state::ClientState::Lobby => {
                    drop(ctx);
                    do_lobby(&mut rl).await?
                }
                client::state::ClientState::Direct(_) => {
                    drop(ctx);
                    do_direct(&mut rl).await?
                }
                client::state::ClientState::Room(_) => {
                    drop(ctx);
                    do_room(&mut rl).await?
                }
            };
        }
    }
    Ok(())
}

async fn do_lobby(rl: &mut Editor<(), FileHistory>) -> anyhow::Result<()> {
    let readline = rl.readline("meshmesh (lobby)> ");
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
                    "cmds: help, state , discover <ep> , join <roomid> , ping <peerid> , pingall , exit"
                ),
                "discover" => discover_peer(&args.join(" ")).await?,
                "state" => println!("{:?}", CLIENT_CTX.get().unwrap()),
                "cls" => clearscreen::clear().expect("failed to clear screen"),
                "direct" => {
                    let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
                    if let Ok(mut ctx) = mutex.lock() {
                        ctx.state = Direct(args.join(" ").parse()?);
                    }
                }
                "exit" | "quit" => {}
                _ => {}
            }
        }
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
            bail!("quit");
        }
        Err(err) => {
            println!("Error: {err}");
        }
    }
    Ok(())
}
async fn do_direct(rl: &mut Editor<(), FileHistory>) -> anyhow::Result<()> {
    let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
    let mut ctx = mutex.lock().unwrap();
    if let ClientState::Direct(recipient) = ctx.state {
        let readline = rl.readline(&format!("meshmesh (direct to {})> ", recipient).to_string());
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
                        "cmds: help, state , discover <ep> , join <roomid> , ping <peerid> , pingall , exit"
                    ),
                    "discover" => discover_peer(&args.join(" ")).await?,
                    "state" => {
                        drop(ctx);
                        println!("{:?}", CLIENT_CTX.get().unwrap())
                    }
                    "cls" => clearscreen::clear().expect("failed to clear screen"),

                    "exit" | "quit" => ctx.state = Lobby,
                    _ => {
                        drop(ctx);
                        send_chat(recipient, line).await?;
                    }
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

    Ok(())
}

async fn do_room(rl: &Editor<(), FileHistory>) -> anyhow::Result<()> {
    todo!()
}
async fn send_chat(recipient: u8, str: &str) -> anyhow::Result<()> {
    if let Some(mutex) = CLIENT_CTX.get() {
        let ctx = mutex.lock().unwrap();
        let peer = ctx.peers.iter().find(|x| x.id == recipient).unwrap();
        Peer::send_to(peer.clone(), str.to_string()).await?;
    }

    Ok(())
}
async fn discover_peer(ticket: &str) -> anyhow::Result<()> {
    if let Some(peer) = Peer::discover(ticket).await? {
        if let Some(mutex) = CLIENT_CTX.get() {
            info!("Adding peer -> {:?}", peer);
            let mut ctx = mutex.lock().unwrap();
            ctx.peers.push(peer.clone());
        }
    }

    Ok(())
}
fn ping_peer(id: u8) -> anyhow::Result<()> {
    todo!()
}
