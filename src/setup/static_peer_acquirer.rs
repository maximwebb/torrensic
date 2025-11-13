use std::net::SocketAddrV4;

use super::PeerAcquirer;

pub struct StaticPeerAcquirer {
    peers: Vec<SocketAddrV4>
}

impl StaticPeerAcquirer {
    pub fn new(peers: Vec<SocketAddrV4>) -> Self {
        Self {
            peers
        }
    }
}

impl PeerAcquirer for StaticPeerAcquirer {
    async fn try_get_peers(&mut self) -> Option<Vec<SocketAddrV4>> {
        Some(self.peers.clone())
    }

    async fn get_peers(&mut self) -> Vec<SocketAddrV4> {
        todo!()
    }
}