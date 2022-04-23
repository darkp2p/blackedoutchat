mod client;
mod config;
mod crypto;
mod error;
mod handler;
mod secure;
mod state;
mod tor;

use futures::{
    future::{ready, select_all},
    FutureExt,
};

use crate::config::Config;
use crate::handler::Handler;
use crate::tor::handler::TorHandler;

#[tokio::main]
async fn main() {
    let config = Config::load();

    if config.addresses.addresses.len() > 1 {
        panic!("Blackedoutchat only supports 1 receiving address for now");
    }

    let handlers = vec![
        ready(tor::spawn_tor(&config).expect("Failed to start Tor process"))
            .then(TorHandler::new)
            .await
            .unwrap()
            .listen(),
    ];

    select_all(handlers).await.0.ok();
}
