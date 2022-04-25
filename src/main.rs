mod client;
mod config;
mod connections;
mod crypto;
mod error;
mod handler;
mod model;
mod secure;
mod state;
mod tor;

use futures::future::try_join3;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::connections::{incoming, outgoing};
use crate::state::State;

#[tokio::main]
async fn main() {
    let config = Config::load();
    let control = tor::spawn_tor(&config).expect("Failed to spawn Tor process");

    let state = Arc::new(Mutex::new(
        State::new(&config).expect("Failed to initialize state"),
    ));

    let a = tor::handle_tor(control);
    let b = incoming::start_incoming(&config, state.clone());
    let c = outgoing::start_outgoing(&config);

    if let Err(e) = try_join3(a, b, c).await {}
}
