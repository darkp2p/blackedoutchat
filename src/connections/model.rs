use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::types::PublicKey;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum BlackPacket {
    Authenticate(Authenticate),
    Data(Data),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum Authenticate {
    Token([u8; 32]),
    OnionAndSig {
        pub_key: PublicKey,
        #[serde(with = "BigArray")]
        sig: [u8; 64],
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "data")]
pub enum Data {
    Message(String),
}
