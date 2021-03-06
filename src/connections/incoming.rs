use std::{fs::remove_file, os::unix::net::UnixListener, path::PathBuf, sync::Arc};

use ed25519_dalek::Signature;
use futures::{
    future::{ready, TryFutureExt},
    stream::{poll_fn, select_all, StreamExt},
    SinkExt,
};
use tokio::{
    net::{UnixListener as AsyncUnixListener, UnixStream},
    sync::Mutex,
};

use crate::{
    config::Config,
    error::{BlackedoutError, Result},
    secure::SecureStream,
    state::State,
    storage::Storage,
    types::PublicKey,
};

use super::model::{Authenticate, BlackPacket};

pub async fn start_incoming(
    _config: &Config,
    state: &Arc<Mutex<State>>,
    storage: &Arc<Storage>,
) -> Result<()> {
    let listeners = state
        .lock()
        .await
        .addresses
        .iter()
        .map(|(key, addr)| (*key, addr.onion.name.clone()))
        .map(|(key, addr)| {
            let path = PathBuf::new()
                .join("data")
                .join("incoming")
                .join(&addr)
                .join("incoming.sock");

            if path.exists() {
                remove_file(&path)?;
            }

            UnixListener::bind(path)
                .and_then(|listener| {
                    listener
                        .set_nonblocking(true)
                        .and_then(|_| AsyncUnixListener::from_std(listener))
                        .and_then(|listener| {
                            Ok(poll_fn(move |cx| {
                                listener
                                    .poll_accept(cx)
                                    .map(|x| Some(x.map(|(x, _)| (x, key))))
                                    .map_err(|e| BlackedoutError::from(e))
                            }))
                        })
                })
                .map_err(Into::into)
        })
        .collect::<Result<Vec<_>>>()?;

    select_all(listeners)
        .for_each(|x| handle_connection(x, &state, &storage))
        .await;

    Ok(())
}

async fn handle_connection(
    stream: Result<(UnixStream, PublicKey)>,
    state: &Arc<Mutex<State>>,
    storage: &Arc<Storage>,
) {
    let (mut stream, host_public_key, token) = match ready(stream)
        .and_then(|(stream, addr)| {
            SecureStream::new(stream, true).map_ok(move |stream| (stream, addr))
        })
        .and_then(|(mut stream, addr)| async move {
            // Send 32-byte token for the peer to sign
            let token = rand::random();
            let res = stream
                .send(BlackPacket::Authenticate(Authenticate::Token(token)))
                .await;

            res.map(|_| (stream, addr, token))
        })
        .await
    {
        Ok(n) => n,
        Err(_e) => {
            // TODO: Error handling
            return;
        }
    };

    let peer_public_key = match stream
        .next()
        .await
        .ok_or(BlackedoutError::ConnectionClosed)
        .and_then(|x| x)
        .and_then(|x| verify_sign(x, token))
    {
        Ok(x) => x,
        Err(_e) => {
            // TODO: Error handler
            return;
        }
    };

    super::connection_loop(
        state.clone(),
        storage.clone(),
        stream,
        peer_public_key,
        host_public_key,
    )
    .await;
}

fn verify_sign(packet: BlackPacket, token: [u8; 32]) -> Result<PublicKey> {
    let (pub_key, sig) = match packet {
        BlackPacket::Authenticate(auth) => match auth {
            Authenticate::OnionAndSig { pub_key, sig } => (pub_key, sig),
            _ => {
                return Err(BlackedoutError::WrongPacketType(
                    "Expected an Authenticate::OnionAndSig packet".to_string(),
                ));
            }
        },
        _ => {
            return Err(BlackedoutError::WrongPacketType(
                "Expected an Authenticate packet".to_string(),
            ));
        }
    };

    Signature::from_bytes(&sig)
        .map_err(|_| BlackedoutError::BadSignature)
        .and_then(|signature| pub_key.verify(&token, &signature).map(|_| pub_key))
}
