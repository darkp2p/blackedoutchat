use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlackPacket {
    Authenticate(Authenticate),
    Data(Data),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Authenticate {
    #[serde(with = "BigArray")]
    Token([u8; 32]),
    OnionAndSig {
        #[serde(with = "BigArray")]
        onion: [u8; 56],
        #[serde(with = "BigArray")]
        sig: [u8; 64],
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Data {
    Message(String),
}
