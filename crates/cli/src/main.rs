use ClientWindow::*;
use protocol::{
    EndpointId, SecretKey,
    error::Error,
    node::{Event, Node},
    parse_ticket,
    state::Peer,
};
use rustyline::{DefaultEditor, error::ReadlineError};
#[derive(Debug, Clone)]
pub enum ClientWindow {
    Lobby,
    Direct(EndpointId),
    Room(u8),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let (node, mut events) = Node::spawn(SecretKey::generate()).await?;
    tokio::spawn(async move {
        while let Some(ev) = events.recv().await {
            match ev {
                Event::DirectMessage { from, text } => {
                    println!("\n[dm {}] {text}", from.fmt_short())
                }
                Event::PeerDiscovered(id) => println!("\n[+peer {}]", id.fmt_short()),
            }
        }
    });
    command_line(&node).await?;
    Ok(())
}

async fn command_line(node: &Node) -> anyhow::Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut client_window: ClientWindow = ClientWindow::Lobby;
    loop {
        let readline_text = match client_window {
            Lobby => "meshmesh (lobby)> ".to_string(),
            Direct(id) => format!("meshmesh (direct to {})> ", id),

            Room(_) => "".to_string(),
        };
        let readline = rl.readline(&readline_text);
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    return Ok(());
                }
                if let Err(e) = rl.add_history_entry(line) {
                    println!("Error occurred while adding history entry: {e}");
                };
                let mut parts = line.split_whitespace();
                let cmd = parts.next().unwrap_or("");
                let args: Vec<&str> = parts.collect();

                match cmd {
                    "help" => println!(
                        "cmds: help, state, peers, ping <peerid/ep>, discover <ep>, join <roomid>, direct <peerid>, pingall, clearpeers, exit"
                    ),
                    "discover" => {
                        if let Err(e) = node.discover(parse_ticket(&args.join(" "))?).await {
                            println!("discover error: {e}");
                        }
                    }
                    "ping" => {
                        if let Some(e) = Node::resolve(&node, &args.join(" ")) {
                            let dur = node.ping(e).await?;
                            println!("ping! -> {:?}", dur);
                        } else {
                            println!("ping error");
                        }
                    }
                    "peers" => println!("\n Peers: {:?}", node.peers()),
                    "info" => println!("{:?}", node.ticket()),
                    "cls" => clearscreen::clear().expect("failed to clear screen"),
                    "direct" => {
                        if let Some(id) = Node::resolve(&node, &args.join(" ")) {
                            client_window = Direct(id)
                        } else {
                            println!("could not resolve node");
                        }
                    }
                    "clearpeers" => node.forget_peers(),
                    _ => {
                        let res = match client_window {
                            Lobby => lobby_cmds(line, cmd, args),
                            Direct(_) => {
                                direct_cmds(line, cmd, args, &node, &mut client_window).await
                            }
                            Room(_) => room_cmds(line, cmd, args),
                        };
                        if let Err(e) = res {
                            println!("Error occured when matching wildcard: {e}");
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("quit");
                node.shutdown().await?;
                return Ok(());
            }
            Err(err) => {
                println!("Error: {err}");
            }
        }
    }
}

#[allow(unused_variables)]
fn lobby_cmds(line: &str, cmd: &str, args: Vec<&str>) -> Result<(), Error> {
    Ok(())
}
#[allow(unused_variables)]
async fn direct_cmds(
    line: &str,
    cmd: &str,
    args: Vec<&str>,
    node: &Node,
    client_window: &mut ClientWindow,
) -> Result<(), Error> {
    match cmd {
        "exit" | "quit" => *client_window = Lobby,
        _ => {
            let Direct(id) = *client_window else {
                return Err(Error::MissingPeerError);
            };
            let node = node.clone();
            let message_line = line.to_string();
            tokio::spawn(async move {
                if let Err(e) = node.send_dm(id, message_line).await {
                    println!("direct message error: {e}");
                }
            });
        }
    }
    Ok(())
}
#[allow(unused_variables)]
fn room_cmds(line: &str, cmd: &str, args: Vec<&str>) -> Result<(), Error> {
    todo!();
}
