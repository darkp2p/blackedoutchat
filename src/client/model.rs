use serde::{Deserialize, Serialize};

use crate::connections::model::Data;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientPacket {
    ConnectionEstablished {
        peer_key: [u8; 32],
        host_key: [u8; 32],
    },
    DataReceived {
        peer_key: [u8; 32],
        host_key: [u8; 32],
        data: Data,
    },
    SendData {
        token: [u8; 12],
        peer_key: [u8; 32],
        host_key: [u8; 32],
        data: Data,
    },
    SendDataConfirmation {
        token: [u8; 12],
    },
}
