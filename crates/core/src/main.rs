use anyhow::{Result, anyhow, bail};
use client::state::{
    ClientState::{Direct, Lobby, Room},
    Context,
};
use log::info;
use protocol::state::Peer;
use rustyline::{DefaultEditor, Editor, error::ReadlineError, history::FileHistory};

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
        match client_state {
            Lobby => do_lobby(&mut rl).await?,
            Direct(_) => do_direct(&mut rl).await?,
            Room(_) => do_room(&rl).await?,
        }
    }
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
    let mut recipient_opt = None;
    if let Ok(ctx) = mutex.lock()
        && let Direct(recipient) = ctx.state
    {
        recipient_opt = Some(recipient);
    }

    if let Some(recipient) = recipient_opt {
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
                        "cmds: help, state, peers, discover <ep>, join <roomid>, ping <peerid>, pingall, exit"
                    ),
                    "discover" => discover_peer(&args.join(" ")).await?,
                    "state" => {
                        println!("{:?}", CLIENT_CTX.get().unwrap())
                    }
                    "cls" => clearscreen::clear().expect("failed to clear screen"),
                    "exit" | "quit" => {
                        let mutex = CLIENT_CTX.get().ok_or(anyhow!("couldnt get mutex"))?;
                        if let Ok(mut ctx) = mutex.lock() {
                            ctx.state = Lobby;
                        }
                    }
                    _ => {
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

#[allow(unused_variables)]
async fn do_room(rl: &Editor<(), FileHistory>) -> anyhow::Result<()> {
    todo!()
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
    Peer::send_to(peer.unwrap().clone(), str.to_string()).await?;

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
