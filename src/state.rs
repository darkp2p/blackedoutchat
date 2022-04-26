use std::collections::HashMap;

use tokio::sync::mpsc::Sender;

use crate::{
    config::Config,
    connections::model::Data,
    error::Result,
    tor::onion::{get_onion_data, Onion},
};

pub struct State {
    pub addresses: HashMap<[u8; 32], AddressState>,
}

pub struct AddressState {
    pub onion: Onion,
    pub connected_peers: HashMap<[u8; 32], Sender<([u8; 12], Data)>>,
}

impl State {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(State {
            addresses: get_onion_data(config)?
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        AddressState {
                            onion: v,
                            connected_peers: Default::default(),
                        },
                    )
                })
                .collect(),
        })
    }
}
