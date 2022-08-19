use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use crate::{connections::model::Data, types::PublicKey};

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum ClientPacket {
    Connect(PeerHostPair),
    ConnectionEstablished(PeerHostPair),
    Initialize(Initialize),
    Disconnected(PeerHostPair),
    DataReceived {
        #[serde(flatten)]
        pair: PeerHostPair,
        data: Data,
    },
    SendData {
        #[serde_as(as = "Base64")]
        token: [u8; 12],
        #[serde(flatten)]
        pair: PeerHostPair,
        data: Data,
    },
    SendDataConfirmation {
        #[serde_as(as = "Base64")]
        token: [u8; 12],
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PeerHostPair {
    pub peer_public_key: PublicKey,
    pub host_public_key: PublicKey,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Initialize {
    pub connected_peers: HashMap<PublicKey, Vec<PublicKey>>,
}
