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
mod types;

use std::sync::Arc;

use futures::future::try_join4;
use tokio::sync::{mpsc::channel, Mutex};

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

    let a = tor::handle_tor(control);
    let b = incoming::start_incoming(&config, &state, &storage);
    let c = outgoing::start_outgoing(&config, &state, &storage, outgoing_rx);
    let d = client::start_clients(&config, &storage, &state, outgoing_tx);

    println!("Start listening");

    if let Err(_e) = try_join4(a, b, c, d).await {}
}
