use std::{collections::HashSet, net::SocketAddrV4};

use tokio::sync::{mpsc, oneshot, watch};

pub type PeerSet = HashSet<SocketAddrV4>;

// Used to facilitate peer info updates
#[derive(Clone)]
pub(crate) struct PeerInfoFeed {
    rx_new_peers: watch::Receiver<PeerSet>,
    tx_req_all_peers: mpsc::Sender<oneshot::Sender<PeerSet>>,
}

impl PeerInfoFeed {
    pub(crate) fn new() -> (
        Self,
        watch::Sender<PeerSet>,
        mpsc::Receiver<oneshot::Sender<PeerSet>>,
    ) {
        let (tx_new_peers, rx_new_peers) = watch::channel(HashSet::new());
        let (tx_req_all_peers, rx_req_all_peers) = mpsc::channel(8);
        (
            Self {
                rx_new_peers,
                tx_req_all_peers,
            },
            tx_new_peers,
            rx_req_all_peers,
        )
    }

    pub(crate) async fn get_new_peers(&mut self) -> Option<PeerSet> {
        if self.rx_new_peers.changed().await.is_ok() {
            Some(self.rx_new_peers.borrow_and_update().clone())
        } else {
            None
        }
    }

    pub(crate) async fn get_all_peers(&self) -> Result<PeerSet, Box<dyn std::error::Error>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx_req_all_peers.send(tx).await?;

        return Ok(rx.await?);
    }
}
