pub mod incoming;
pub mod model;
pub mod outgoing;

use std::sync::{Arc, Mutex};

use data_encoding::BASE32;
use ed25519_dalek::PublicKey;
use futures::{
    future::{select, Either},
    stream::StreamExt,
    SinkExt,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc::{channel, Receiver},
};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    client::model::ClientPacket,
    error::{BlackedoutError, Result},
    secure::SecureStream,
    state::State,
    storage::Storage,
};

use self::model::{BlackPacket, Data};

pub async fn connection_loop<S>(
    state: Arc<Mutex<State>>,
    storage: Arc<Storage>,
    stream: SecureStream<S>,
    addr: [u8; 32],
    peer_key: [u8; 32],
    host_key: [u8; 32],
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (tx, rx): (_, Receiver<([u8; 12], Data)>) = channel(1);

    state
        .lock()
        .unwrap()
        .addresses
        .get_mut(&addr)
        .unwrap()
        .connected_peers
        .insert(peer_key, tx);

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
            let (data, token) = match select(to_peer.next(), from_peer.next()).await {
                Either::Left((n, _)) => match n {
                    Some((token, data)) => (data, Some(token)),
                    None => break,
                },
                Either::Right((n, _)) => match n {
                    Some(Ok(n)) => (n, None),
                    Some(Err(_)) => {
                        // TODO: Error handling
                        // The peer sent a wrong packet type so disconnect it here
                        break;
                    }
                    None => break,
                },
            };

            let packet = match token {
                Some(token) => {
                    match stream_tx.send(BlackPacket::Data(data.clone())).await {
                        Ok(_) => {}
                        Err(_e) => {
                            // TODO: Error handling
                            continue;
                        }
                    }

                    ClientPacket::SendDataConfirmation { token }
                }
                None => ClientPacket::DataReceived {
                    peer_key,
                    host_key,
                    data,
                },
            };

            storage.send_packet(packet).await;
        }

        state
            .lock()
            .unwrap()
            .addresses
            .get_mut(&addr)
            .unwrap()
            .connected_peers
            .remove(&peer_key);

        to_peer.into_inner().close();
        stream_tx.close().await.ok();
    });
}

pub fn decode_onion(onion: &[u8; 56]) -> Result<[u8; 32]> {
    let decoded_onion = BASE32
        .decode(onion)
        .map_err(|e| BlackedoutError::Base32Error(e))?;

    if decoded_onion.len() < 32 {
        return Err(BlackedoutError::BadHostname);
    }

    PublicKey::from_bytes(&decoded_onion[..32])
        .map_err(|_| BlackedoutError::BadPublicKey)
        .map(|public_key| *public_key.as_bytes())
}
