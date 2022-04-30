use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Deserialize, Serialize)]
pub struct Clients {
    #[serde(rename = "client")]
    pub clients: Vec<Client>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Client {
    Tcp { address: SocketAddr },
}

impl super::ConfigTrait for Clients {
    fn name() -> &'static str {
        "clients"
    }
}

impl Default for Clients {
    fn default() -> Self {
        Clients {
            clients: vec![Client::Tcp {
                address: "127.0.0.1:8080".parse().unwrap(),
            }],
        }
    }
}
