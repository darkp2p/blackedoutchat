pub mod incoming;
pub mod model;
pub mod outgoing;

use std::sync::Arc;

use futures::{
    future::{select, Either},
    stream::StreamExt,
    SinkExt,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{
        mpsc::{channel, Receiver},
        Mutex,
    },
};
use tokio_stream::wrappers::ReceiverStream;

use crate::{
    client::model::{ClientPacket, PeerHostPair},
    error::BlackedoutError,
    secure::SecureStream,
    state::State,
    storage::Storage,
    types::PublicKey,
};

use self::model::{BlackPacket, Data};

pub async fn connection_loop<S>(
    state: Arc<Mutex<State>>,
    storage: Arc<Storage>,
    stream: SecureStream<S>,
    peer_public_key: PublicKey,
    host_public_key: PublicKey,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (tx, rx): (_, Receiver<([u8; 12], Data)>) = channel(1);

    state
        .lock()
        .await
        .addresses
        .get_mut(&host_public_key)
        .unwrap()
        .connected_peers
        .insert(peer_public_key, tx);

    let (mut stream_tx, stream_rx) = stream.split();

    let mut to_peer = ReceiverStream::new(rx);
    let mut from_peer = stream_rx.map(|x| {
        x.and_then(|x| match x {
            BlackPacket::Data(n) => Ok(n),
            _ => Err(BlackedoutError::WrongPacketType(
                "Expected a Data packet".to_string(),
            )),
        })
    });

    storage
        .send_packet(ClientPacket::ConnectionEstablished(PeerHostPair {
            peer_public_key,
            host_public_key,
        }))
        .await;

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
                    pair: PeerHostPair {
                        peer_public_key,
                        host_public_key,
                    },
                    data,
                },
            };

            storage.send_packet(packet).await;
        }

        state
            .lock()
            .await
            .addresses
            .get_mut(&host_public_key)
            .unwrap()
            .connected_peers
            .remove(&peer_public_key);

        to_peer.into_inner().close();
        stream_tx.close().await.ok();

        storage
            .send_packet(ClientPacket::Disconnected(PeerHostPair {
                peer_public_key,
                host_public_key,
            }))
            .await;
    });
}
