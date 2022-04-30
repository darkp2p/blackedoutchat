use serde::{Deserialize, Serialize};

use crate::{connections::model::Data, types::PublicKey};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum ClientPacket {
    Connect {
        peer_public_key: PublicKey,
        host_public_key: PublicKey,
    },
    ConnectionEstablished {
        peer_public_key: PublicKey,
        host_public_key: PublicKey,
    },
    Disconnected {
        peer_public_key: PublicKey,
        host_public_key: PublicKey,
    },
    DataReceived {
        peer_public_key: PublicKey,
        host_public_key: PublicKey,
        data: Data,
    },
    SendData {
        token: [u8; 12],
        peer_public_key: PublicKey,
        host_public_key: PublicKey,
        data: Data,
    },
    SendDataConfirmation {
        token: [u8; 12],
    },
}
