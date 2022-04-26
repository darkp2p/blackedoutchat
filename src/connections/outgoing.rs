use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ed25519_dalek::{ExpandedSecretKey, PublicKey};
use futures::{SinkExt, StreamExt, TryFutureExt};
use tokio::{
    net::UnixStream,
    sync::mpsc::{Receiver, Sender},
};
use tokio_socks::tcp::Socks5Stream;

use crate::{
    config::Config,
    error::{BlackedoutError, Result},
    secure::SecureStream,
    state::State,
    storage::Storage,
};

use super::model::{Authenticate, BlackPacket};

pub async fn start_outgoing(
    config: &Config,
    state: &Arc<Mutex<State>>,
    storage: &Arc<Storage>,
    mut rx: Receiver<([u8; 56], [u8; 56], ExpandedSecretKey, Sender<Result<()>>)>,
) -> Result<()> {
    while let Some((hostname, addr, secret_key, reply)) = rx.recv().await {
        reply
            .send(handle_request(config, state, storage, hostname, addr, secret_key).await)
            .await
            .ok();
    }

    Ok(())
}

async fn handle_request(
    _config: &Config,
    state: &Arc<Mutex<State>>,
    storage: &Arc<Storage>,
    hostname: [u8; 56],
    address: [u8; 56],
    secret_key: ExpandedSecretKey,
) -> Result<()> {
    let peer_key = super::decode_onion(&hostname)?;
    let hostname = String::from_utf8(hostname.into())
        .map_err(|_| BlackedoutError::BadHostname)
        .map(|mut x| {
            x.push_str(".onion");
            x
        })?;

    let mut stream = UnixStream::connect(PathBuf::new().join("data").join("tor.sock"))
        .map_err(Into::into)
        .and_then(|socket| Socks5Stream::connect_with_socket(socket, hostname))
        .map_err(Into::into)
        .and_then(|stream| SecureStream::new(stream, false))
        .await?;

    let token = match stream
        .next()
        .await
        .ok_or(BlackedoutError::ConnectionClosed)??
    {
        BlackPacket::Authenticate(Authenticate::Token(n)) => n,
        _ => {
            return Err(BlackedoutError::WrongPacketType {
                description: "Expected an Authenticate::Token packet".to_string(),
            })
        }
    };

    let public_key = PublicKey::from(&secret_key);
    let signature = secret_key.sign(&token, &public_key);

    stream
        .send(BlackPacket::Authenticate(Authenticate::OnionAndSig {
            onion: address,
            sig: signature.to_bytes(),
        }))
        .await?;

    let address = super::decode_onion(&address)?;

    super::connection_loop(
        state.clone(),
        storage.clone(),
        stream,
        address,
        peer_key,
        *public_key.as_bytes(),
    )
    .await;

    Ok(())
}
