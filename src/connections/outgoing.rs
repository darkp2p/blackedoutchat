use std::{path::PathBuf, sync::Arc};

use ed25519_dalek::ExpandedSecretKey;
use futures::{SinkExt, StreamExt, TryFutureExt};
use tokio::{
    net::UnixStream,
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
};
use tokio_socks::tcp::Socks5Stream;

use crate::{
    config::Config,
    error::{BlackedoutError, Result},
    secure::SecureStream,
    state::State,
    storage::Storage,
    types::PublicKey,
};

use super::model::{Authenticate, BlackPacket};

pub async fn start_outgoing(
    config: &Config,
    state: &Arc<Mutex<State>>,
    storage: &Arc<Storage>,
    mut rx: Receiver<(PublicKey, PublicKey, Sender<Result<()>>)>,
) -> Result<()> {
    while let Some((peer_public_key, host_public_key, reply)) = rx.recv().await {
        reply
            .send(handle_request(config, state, storage, peer_public_key, host_public_key).await)
            .await
            .ok();
    }

    Ok(())
}

async fn handle_request(
    _config: &Config,
    state: &Arc<Mutex<State>>,
    storage: &Arc<Storage>,
    peer_public_key: PublicKey,
    host_public_key: PublicKey,
) -> Result<()> {
    let target_addr = format!("{}:21761", peer_public_key.to_onion_address());
    let host_secret_key = ExpandedSecretKey::from_bytes(
        &state
            .lock()
            .await
            .addresses
            .get(&host_public_key)
            .unwrap()
            .onion
            .secret_key
            .to_bytes(),
    )
    .unwrap();

    let mut stream = UnixStream::connect(PathBuf::new().join("data").join("tor.sock"))
        .map_err(Into::into)
        .and_then(|socket| Socks5Stream::connect_with_socket(socket, target_addr))
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
            return Err(BlackedoutError::WrongPacketType(
                "Expected an Authenticate::Token packet".to_string(),
            ))
        }
    };

    let signature = host_public_key.sign(&token, &host_secret_key);

    stream
        .send(BlackPacket::Authenticate(Authenticate::OnionAndSig {
            pub_key: host_public_key,
            sig: signature.to_bytes(),
        }))
        .await?;

    super::connection_loop(
        state.clone(),
        storage.clone(),
        stream,
        peer_public_key,
        host_public_key,
    )
    .await;

    Ok(())
}
