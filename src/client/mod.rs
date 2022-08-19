pub mod model;

use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Extension, Json,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{any, get, get_service},
    Router, Server,
};
use futures::{
    future::{try_join_all, TryFutureExt},
    stream::{SplitSink, StreamExt},
    FutureExt, SinkExt,
};
use tokio::sync::{
    mpsc::{channel, Sender},
    Mutex,
};
use tower_http::{cors::CorsLayer, services::ServeDir};

use crate::{
    config::{Client, Config},
    error::{BlackedoutError, Result},
    state::State,
    storage::Storage,
    types::PublicKey,
};

use self::model::{ClientPacket, Initialize, PeerHostPair};

type OutgoingTx = Sender<(PublicKey, PublicKey, Sender<Result<()>>)>;
type FutureBoxed = Pin<Box<dyn Future<Output = Result<()>>>>;
type ConnectedClients = HashMap<[u8; 32], SplitSink<WebSocket, Message>>;

pub async fn start_clients(
    config: &Config,
    storage: &Arc<Storage>,
    state: &Arc<Mutex<State>>,
    outgoing_tx: OutgoingTx,
) -> Result<()> {
    let collection = config
        .clients
        .clients
        .iter()
        .map(|x| match x {
            Client::Tcp { address } => {
                let mut rx = storage.subscribe();

                let connected_clients = Arc::new(Mutex::new(ConnectedClients::new()));
                let conn_clients0 = connected_clients.clone();

                let a: FutureBoxed = Box::pin(async move {
                    while let Some(n) = rx.recv().await {
                        let n = serde_json::to_string(&n).unwrap();

                        for (_, client) in conn_clients0.lock().await.iter_mut() {
                            client.send(Message::Text(n.clone())).await.ok();
                        }
                    }

                    Ok(())
                });

                let b: FutureBoxed = Box::pin(
                    Server::bind(address)
                        .serve(
                            Router::new()
                                .route(
                                    "/",
                                    get_service(ServeDir::new("webapp")).handle_error(
                                        |e| async move {
                                            (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e))
                                        },
                                    ),
                                )
                                .route("/connect", any(connect_handler))
                                .route("/ws", get(ws_handler))
                                .layer(Extension(connected_clients))
                                .layer(Extension(state.clone()))
                                .layer(Extension(outgoing_tx.clone()))
                                .layer(CorsLayer::permissive())
                                .into_make_service(),
                        )
                        .map_err(Into::into),
                );

                vec![a, b]
            }
        })
        .flatten()
        .collect::<Vec<FutureBoxed>>();

    try_join_all(collection).await.map(|_| ())
}

async fn connect_handler(
    Json(packet): Json<PeerHostPair>,
    Extension(state): Extension<Arc<Mutex<State>>>,
    Extension(outgoing_txt): Extension<OutgoingTx>,
) -> std::result::Result<(), (StatusCode, Json<BlackedoutError>)> {
    let (tx, mut rx) = channel(1);

    outgoing_txt
        .send((packet.peer_public_key, packet.host_public_key, tx))
        .await
        .unwrap();

    rx.recv()
        .await
        .unwrap()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(e)))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    state: Extension<Arc<Mutex<State>>>,
    outgoing_tx: Extension<OutgoingTx>,
    connected_clients: Extension<Arc<Mutex<ConnectedClients>>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| ws_socket_handler(socket, state, outgoing_tx, connected_clients))
}

async fn ws_socket_handler(
    socket: WebSocket,
    Extension(state): Extension<Arc<Mutex<State>>>,
    Extension(outgoing_txt): Extension<OutgoingTx>,
    Extension(connected_clients): Extension<Arc<Mutex<ConnectedClients>>>,
) {
    let (mut tx, mut rx) = socket.split();

    let connected_peers = state
        .lock()
        .await
        .addresses
        .iter()
        .map(|(a, b)| (*a, b.connected_peers.keys().map(|x| *x).collect::<Vec<_>>()))
        .collect();

    tx.send(Message::Text(
        serde_json::to_string(&ClientPacket::Initialize(Initialize { connected_peers })).unwrap(),
    ))
    .await
    .unwrap();

    let id = rand::random();
    connected_clients.lock().await.insert(id, tx);

    while let Some(Ok(n)) = rx
        .next()
        .await
        .map(|x| x.map(|x| serde_json::from_slice::<ClientPacket>(&x.into_data())))
    {
        let n = match n {
            Ok(n) => n,
            Err(e) => {
                println!("Error deserializing client packet: {:?}", e);
                continue;
            }
        };

        if let Err(e) = match n {
            ClientPacket::SendData { token, pair, data } => {
                state
                    .lock()
                    .then(|state| async move {
                        state
                            .addresses
                            .get(&pair.host_public_key)
                            .ok_or(BlackedoutError::HostPublicKeyDoesNotExist)?
                            .connected_peers
                            .get(&pair.peer_public_key)
                            .ok_or(BlackedoutError::PeerPublicKeyDoesNotExist)?
                            .send((token, data))
                            .map_err(|_| BlackedoutError::Unexpected)
                            .await
                    })
                    .await
            }
            _ => Err(BlackedoutError::WrongPacketType(
                "Unexpected packet".to_string(),
            )),
        } {
            connected_clients
                .lock()
                .await
                .get_mut(&id)
                .unwrap()
                .send(Message::Text(serde_json::to_string(&e).unwrap()))
                .await
                .ok();
        }
    }

    // Disconnected
    connected_clients.lock().await.remove(&id);
}
