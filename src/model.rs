use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

#[derive(Deserialize, Serialize)]
pub enum BlackPacket {
    Authenticate(Authenticate),
    Data(Data),
}

#[derive(Deserialize, Serialize)]
pub enum Authenticate {
    #[serde(with = "BigArray")]
    Init([u8; 56]),
    ReturnToken([u8; 32]),
    #[serde(with = "BigArray")]
    ReturnSig([u8; 64]),
}

#[derive(Deserialize, Serialize)]
pub enum Data {
    Message(String),
}
