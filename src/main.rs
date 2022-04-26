mod client;
mod config;
mod connections;
mod crypto;
mod error;
mod handler;
mod secure;
mod state;
mod storage;
mod tor;

use futures::future::try_join3;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::channel;

use crate::config::Config;
use crate::connections::{incoming, outgoing};
use crate::state::State;
use crate::storage::Storage;

#[tokio::main]
async fn main() {
    let config = Config::load();
    let control = tor::spawn_tor(&config).expect("Failed to spawn Tor process");

    let storage = Arc::new(Storage::new());
    let state = Arc::new(Mutex::new(
        State::new(&config).expect("Failed to initialize state"),
    ));

    let (outgoing_tx, outgoing_rx) = channel(1);

    {
        let outgoing_tx = outgoing_tx.clone();
        let storage = storage.clone();
        let state = state.clone();

        use std::io::Write;
        use tokio::runtime::Runtime;

        let addresses = state
            .lock()
            .unwrap()
            .addresses
            .values()
            .map(|x| String::from_utf8_lossy(&x.onion.hostname).to_string())
            .collect::<Vec<_>>();

        println!("Listening addresses: {:#?}\n", addresses);

        print!("Testing for alice? (Y/n): ");
        std::io::stdout().flush().unwrap();

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();

        match buf.to_lowercase().trim() {
            "Y" | "" => {
                println!("Starting Alice test (listening for connections)");

                std::thread::spawn(move || {
                    let runtime = Runtime::new().unwrap();
                    runtime.block_on(test_alice(storage, state));

                    println!("Finished Alice test");

                    unsafe { libc::kill(std::process::id().try_into().unwrap(), libc::SIGTERM) };
                });
            }
            _ => {
                println!("Starting Bob test. Please have Alice ready to accept connections");

                let alice = loop {
                    print!("Alice address: ");
                    std::io::stdout().flush().unwrap();

                    let mut buf = String::new();
                    std::io::stdin().read_line(&mut buf).unwrap();
                    let buf = buf.trim().to_uppercase();

                    if buf.len() != 56 {
                        println!("Invalid address. Address must be 56 characters with no `.onion` at the end");
                        continue;
                    }

                    break buf;
                };

                let from = loop {
                    print!("Address to connect from: ");
                    std::io::stdout().flush().unwrap();

                    let mut buf = String::new();
                    std::io::stdin().read_line(&mut buf).unwrap();
                    let buf = buf.trim().to_uppercase();

                    if buf.len() != 56 {
                        println!("Invalid address. Address must be 56 characters with no `.onion` at the end");
                        continue;
                    }

                    if !addresses.contains(&buf) {
                        println!("Address must be one of the ones listed above");
                        continue;
                    }

                    break buf.trim().to_string();
                };

                println!("Connecting to Alice at `{}` from `{}`", alice, from);

                std::thread::spawn(move || {
                    let runtime = Runtime::new().unwrap();
                    runtime.block_on(test_bob(alice, from, outgoing_tx, storage, state));

                    println!("Finished Bob test");

                    unsafe { libc::kill(std::process::id().try_into().unwrap(), libc::SIGTERM) };
                });
            }
        }
    }

    let a = tor::handle_tor(control);
    let b = incoming::start_incoming(&config, &state, &storage);
    let c = outgoing::start_outgoing(&config, &state, &storage, outgoing_rx);

    if let Err(_e) = try_join3(a, b, c).await {}
}

use ed25519_dalek::ExpandedSecretKey;
use std::{fs::OpenOptions, io::Write};
use tokio::sync::mpsc::Sender;

use crate::client::model::ClientPacket;
use crate::connections::model::*;
use crate::error::Result;

async fn test_alice(storage: Arc<Storage>, state: Arc<Mutex<State>>) {
    let mut watch = storage.subscribe();

    std::thread::spawn(move || loop {
        print!("Message: ");
        std::io::stdout().flush().unwrap();

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();
        let buf = buf.trim().to_string();

        state
            .lock()
            .unwrap()
            .addresses
            .values()
            .for_each(|address| {
                address.connected_peers.values().for_each(|tx| {
                    tx.blocking_send((rand::random(), Data::Message(buf.clone())))
                        .unwrap();
                })
            });
    });

    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open("alice_log")
        .unwrap();

    loop {
        watch.changed().await.unwrap();

        let packet = match watch.borrow().as_ref() {
            Some(n) => n.to_owned(),
            None => continue,
        };

        let data = match packet {
            ClientPacket::ConnectionEstablished { peer_key, host_key } => {
                format!(
                    "Connection established from peer `{}` to address `{}`\n",
                    String::from_utf8_lossy(&peer_key),
                    String::from_utf8_lossy(&host_key)
                )
            }
            ClientPacket::DataReceived {
                peer_key,
                host_key,
                data,
            } => {
                format!(
                    "Data received from peer `{}` to address `{}`: {:?}\n",
                    String::from_utf8_lossy(&peer_key),
                    String::from_utf8_lossy(&host_key),
                    data
                )
            }
            ClientPacket::SendDataConfirmation { token } => {
                format!("Send data confirmation with token `{:?}`\n", token)
            }
            _ => {
                unreachable!()
            }
        };

        log.write_all(data.as_bytes()).unwrap();
    }
}

async fn test_bob(
    alice: String,
    from: String,
    outgoing_tx: Sender<([u8; 56], [u8; 56], ExpandedSecretKey, Sender<Result<()>>)>,
    storage: Arc<Storage>,
    state: Arc<Mutex<State>>,
) {
    let mut watch = storage.subscribe();

    let alice: [u8; 56] = alice.as_bytes().try_into().unwrap();
    let from: [u8; 56] = from.as_bytes().try_into().unwrap();
    let secret_key = ExpandedSecretKey::from_bytes(
        &state
            .lock()
            .unwrap()
            .addresses
            .get(&crate::connections::decode_onion(&from).unwrap())
            .unwrap()
            .onion
            .secret_key
            .to_bytes(),
    )
    .unwrap();

    let (tx, mut rx) = channel(1);

    outgoing_tx
        .send((alice, from, secret_key, tx))
        .await
        .map_err(|_| {})
        .unwrap();

    println!("Connecting to Alice");
    rx.recv().await.unwrap().unwrap();
    println!("Connected to Alice");

    std::thread::spawn(move || loop {
        print!("Message: ");
        std::io::stdout().flush().unwrap();

        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();
        let buf = buf.trim().to_string();

        state
            .lock()
            .unwrap()
            .addresses
            .values()
            .for_each(|address| {
                address.connected_peers.values().for_each(|tx| {
                    tx.blocking_send((rand::random(), Data::Message(buf.clone())))
                        .unwrap();
                })
            });
    });

    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .open("bob_log")
        .unwrap();

    loop {
        watch.changed().await.unwrap();

        let packet = match watch.borrow().as_ref() {
            Some(n) => n.to_owned(),
            None => continue,
        };

        let data = match packet {
            ClientPacket::ConnectionEstablished { peer_key, host_key } => {
                format!(
                    "Connection established from peer `{}` to address `{}`\n",
                    String::from_utf8_lossy(&peer_key),
                    String::from_utf8_lossy(&host_key)
                )
            }
            ClientPacket::DataReceived {
                peer_key,
                host_key,
                data,
            } => {
                format!(
                    "Data received from peer `{}` to address `{}`: {:?}\n",
                    String::from_utf8_lossy(&peer_key),
                    String::from_utf8_lossy(&host_key),
                    data
                )
            }
            ClientPacket::SendDataConfirmation { token } => {
                format!("Send data confirmation with token `{:?}`\n", token)
            }
            _ => {
                unreachable!()
            }
        };

        log.write_all(data.as_bytes()).unwrap();
    }
}
