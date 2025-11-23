use std::net::SocketAddrV4;

use async_trait::async_trait;

use crate::setup::PeerList;

use super::PeerAcquirer;

pub struct StaticPeerAcquirer {
    peers: Vec<SocketAddrV4>,
}

impl StaticPeerAcquirer {
    pub fn new(peers: Vec<SocketAddrV4>) -> Self {
        Self { peers }
    }
}

#[async_trait]
impl PeerAcquirer for StaticPeerAcquirer {
    async fn try_get_peers(&mut self) -> Option<PeerList> {
        Some(PeerList(self.peers.clone()))
    }

    async fn get_peers(&mut self) -> PeerList {
        PeerList(self.peers.clone())
    }
}
