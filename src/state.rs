use std::collections::HashMap;

use futures::stream::SplitSink;
use tokio::net::UnixStream;

use crate::{model::BlackPacket, secure::SecureStream};

pub struct State {
    connected_peers: HashMap<[u8; 32], SplitSink<SecureStream<UnixStream>, BlackPacket>>,
}
