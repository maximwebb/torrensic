use std::net::SocketAddrV4;

use crate::parser::metadata::Metadata;

pub trait PeerAcquirer {
    async fn try_get_peers(&mut self) -> Option<Vec<SocketAddrV4>>;

    async fn get_peers(&mut self) -> Vec<SocketAddrV4>;
}

pub trait MetadataAcquirer {
    fn get_metadata(&self) -> Metadata;
}

pub mod tracker_peer_acquirer;
pub mod dht_peer_acquirer;