use std::{
    sync::{Arc, Mutex},
    thread,
};

use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::client::model::ClientPacket;

pub struct Storage {
    storage_tx: Sender<ClientPacket>,
    subscribers: Arc<Mutex<Vec<Sender<ClientPacket>>>>,
}

impl Storage {
    pub fn new() -> Self {
        let (storage_tx, mut storage_rx): (Sender<ClientPacket>, _) = channel(1);
        let subscribers = Arc::new(Mutex::new(Vec::new()));
        let sub0 = subscribers.clone();

        thread::spawn(move || {
            while let Some(n) = storage_rx.blocking_recv() {
                // Send to subscribed clients
                sub0.lock()
                    .unwrap()
                    .clone()
                    .into_iter()
                    .for_each(|x: Sender<ClientPacket>| {
                        x.blocking_send(n.clone()).ok();
                    });

                // TODO: Store in database
            }
        });

        Storage {
            storage_tx,
            subscribers,
        }
    }

    pub async fn send_packet(&self, packet: ClientPacket) {
        self.storage_tx.send(packet).await.unwrap();
    }

    pub fn subscribe(&self) -> Receiver<ClientPacket> {
        let (tx, rx) = channel(1);
        self.subscribers.lock().unwrap().push(tx);
        rx
    }
}
