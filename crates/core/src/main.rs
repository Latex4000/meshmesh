use anyhow::{Result, anyhow, bail};
use client::state::{
    ClientState::{Direct, Lobby, Room},
    Context,
};
use log::info;
use protocol::state::Peer;
use rustyline::{DefaultEditor, error::ReadlineError};

use std::sync::{Mutex, OnceLock};
// use std::sync::{LazyLock};
//static PROTO_CTX: LazyLock<Context> = LazyLock::new(|| {});
static CLIENT_CTX: OnceLock<Mutex<Context>> = OnceLock::new();
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let peer = protocol::init().await?;

    let ctx = Context {
        state: Lobby,
        local: peer,
        peers: Vec::new(),
    };
    CLIENT_CTX.set(Mutex::new(ctx)).unwrap();
    command_line().await?;
    Ok(())
}

async fn command_line() -> anyhow::Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut client_state = Lobby;

    loop {
        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
        if let Ok(ctx) = mutex.lock() {
            client_state = ctx.state.clone();
        }

        let recipient_opt = get_recipient();
        let readline_text = match client_state {
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
                        "cmds: help, state, peers, discover <ep>, join <roomid>, ping <peerid>, pingall, exit"
                    ),
                    "discover" => discover_peer(&args.join(" ")).await?,
                    "peers" => {
                        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
                        if let Ok(ctx) = mutex.lock() {
                            println!(
                                "\n Peers: {:?}",
                                ctx.peers.iter().map(|x| x.id).collect::<Vec<_>>()
                            )
                        }
                    }
                    "state" => println!("{:?}", CLIENT_CTX.get().unwrap()),
                    "cls" => clearscreen::clear().expect("failed to clear screen"),
                    "direct" => {
                        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
                        if let Ok(mut ctx) = mutex.lock() {
                            match args.join(" ").parse() {
                                Ok(id) => ctx.state = Direct(id),
                                Err(e) => println!("Could not connect to ID.\n{e}"),
                            }
                        }
                    }
                    _ => match client_state {
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
        && let Direct(recipient) = ctx.state
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
                ctx.state = Lobby;
            }
        }
        _ => {
            if let Some(recipient) = get_recipient() {
                send_chat(recipient, line).await?;
            }
        }
    }
    Ok(())
}
#[allow(unused_variables)]
fn room_cmds(line: &str, cmd: &str, args: Vec<&str>) -> anyhow::Result<()> {
    todo!();
}

async fn send_chat(recipient: u8, str: &str) -> anyhow::Result<()> {
    let mut peer: Option<Peer> = None;
    if let Some(mutex) = CLIENT_CTX.get()
        && let Ok(ctx) = mutex.lock()
    {
        match ctx.peers.iter().find(|x| x.id == recipient) {
            Some(p) => peer = Some(p.clone()),
            None => return Err(anyhow!("Could not find peer")),
        };
    }
    if let Some(peer) = peer {
        Peer::send_to(peer, str.to_string()).await?;
    }

    Ok(())
}
async fn discover_peer(ticket: &str) -> anyhow::Result<()> {
    if let Some(peer) = Peer::discover(ticket).await?
        && let Some(mutex) = CLIENT_CTX.get()
        && let Ok(mut ctx) = mutex.lock()
    {
        info!("Adding peer -> {:?}", peer);
        ctx.peers.push(peer.clone());
    }

    Ok(())
}
#[expect(dead_code)]
#[allow(unused_variables)]
fn ping_peer(id: u8) -> anyhow::Result<()> {
    todo!()
}
