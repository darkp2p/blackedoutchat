use std::thread;

use tokio::sync::{
    mpsc::{channel, Sender},
    watch::{channel as watch, Receiver as WatchReceiver},
};

use crate::client::model::ClientPacket;

pub struct Storage {
    storage_tx: Sender<ClientPacket>,
    subscribe_rx: WatchReceiver<Option<ClientPacket>>,
}

impl Storage {
    pub fn new() -> Self {
        let (storage_tx, mut storage_rx) = channel(1);
        let (subscribe_tx, subscribe_rx) = watch(None);

        thread::spawn(move || {
            while let Some(n) = storage_rx.blocking_recv() {
                // Send to subscribed clients
                subscribe_tx.send(Some(n)).unwrap();

                // TODO: Store in database
            }
        });

        Storage {
            storage_tx,
            subscribe_rx,
        }
    }

    pub async fn send_packet(&self, packet: ClientPacket) {
        self.storage_tx.send(packet).await.unwrap();
    }

    pub fn subscribe(&self) -> WatchReceiver<Option<ClientPacket>> {
        self.subscribe_rx.clone()
    }
}
