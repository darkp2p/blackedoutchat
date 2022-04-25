use std::{
    os::unix::net::UnixListener,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use data_encoding::BASE32;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use futures::{
    future::{ready, select, Either, TryFutureExt},
    stream::{poll_fn, select_all, StreamExt},
    SinkExt,
};
use tokio::{
    net::{UnixListener as AsyncUnixListener, UnixStream},
    sync::mpsc::{channel, Receiver},
};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    config::{Address, Config},
    error::{BlackedoutError, Result},
    model::{Authenticate, BlackPacket, Data},
    secure::SecureStream,
    state::State,
};

pub async fn start_incoming(config: &Config, state: Arc<Mutex<State>>) -> Result<()> {
    let listeners = config
        .addresses
        .addresses
        .clone()
        .into_iter()
        .map(|addr| {
            UnixListener::bind(
                PathBuf::new()
                    .join("data")
                    .join("incoming")
                    .join(&addr.name)
                    .with_extension("sock"),
            )
            .and_then(|listener| {
                listener
                    .set_nonblocking(true)
                    .and_then(|_| AsyncUnixListener::from_std(listener))
                    .and_then(|listener| {
                        Ok(poll_fn(move |cx| {
                            listener
                                .poll_accept(cx)
                                .map(|x| Some(x.map(|(x, _)| (x, addr.clone()))))
                                .map_err(|e| BlackedoutError::from(e))
                        }))
                    })
            })
            .map_err(Into::into)
        })
        .collect::<Result<Vec<_>>>()?;

    select_all(listeners)
        .for_each(|x| handle_connection(x, &state))
        .await;

    Ok(())
}

async fn handle_connection(stream: Result<(UnixStream, Address)>, state: &Arc<Mutex<State>>) {
    let (mut stream, addr, token) = match ready(stream)
        .and_then(|(stream, addr)| SecureStream::new(stream, true).map_ok(|stream| (stream, addr)))
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

    let state = state.clone();
    let (tx, rx): (_, Receiver<Data>) = channel(1);

    let peer_pub = match stream
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

    state
        .lock()
        .unwrap()
        .addresses
        .get_mut(&addr.name)
        .unwrap()
        .connected_peers
        .insert(peer_pub, tx);

    let (mut stream_tx, stream_rx) = stream.split();

    let mut to_peer = ReceiverStream::new(rx);
    let mut from_peer = stream_rx.map(|x| {
        x.and_then(|x| match x {
            BlackPacket::Data(n) => Ok(n),
            _ => Err(BlackedoutError::WrongPacketType {
                description: "Expected a Data packet".to_string(),
            }),
        })
    });

    tokio::spawn(async move {
        loop {
            let (data, send_to_peer) = match select(to_peer.next(), from_peer.next()).await {
                Either::Left((n, _)) => match n {
                    Some(n) => (n, true),
                    None => break,
                },
                Either::Right((n, _)) => match n {
                    Some(Ok(n)) => (n, false),
                    Some(Err(_)) => {
                        // TODO: Error handling
                        // The peer sent a wrong packet type so disconnect it here
                        break;
                    }
                    None => break,
                },
            };

            if send_to_peer {
                match stream_tx.send(BlackPacket::Data(data)).await {
                    Ok(_) => {}
                    Err(_e) => {
                        // TODO: Error handling
                        continue;
                    }
                }
            }
        }

        state
            .lock()
            .unwrap()
            .addresses
            .get_mut(&addr.name)
            .unwrap()
            .connected_peers
            .remove(&peer_pub);

        to_peer.into_inner().close();
        stream_tx.close().await.ok();
    });
}

fn verify_sign(packet: BlackPacket, token: [u8; 32]) -> Result<[u8; 32]> {
    let (onion, sig) = match packet {
        BlackPacket::Authenticate(auth) => match auth {
            Authenticate::OnionAndSig { onion, sig } => (onion, sig),
            _ => {
                return Err(BlackedoutError::WrongPacketType {
                    description: "Expected an Authenticate::OnionAndSig packet".to_string(),
                });
            }
        },
        _ => {
            return Err(BlackedoutError::WrongPacketType {
                description: "Expected an Authenticate packet".to_string(),
            });
        }
    };

    let decoded_onion = BASE32
        .decode(&onion)
        .map_err(|e| BlackedoutError::Base32Error(e))?;

    if decoded_onion.len() < 32 {
        return Err(BlackedoutError::BadHostname);
    }

    PublicKey::from_bytes(&decoded_onion[..32])
        .map_err(|_| BlackedoutError::BadPublicKey)
        .and_then(|public_key| {
            Signature::from_bytes(&sig)
                .map_err(|_| BlackedoutError::BadSignature)
                .and_then(|signature| {
                    public_key
                        .verify(&token, &signature)
                        .map(|_| decoded_onion[..32].try_into().unwrap())
                        .map_err(|_| BlackedoutError::SignatureVerificationFailed)
                })
        })
}
