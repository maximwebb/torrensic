use std::{net::SocketAddrV4, ops::Deref};

use async_trait::async_trait;

use crate::setup::{
    dht_peer_acquirer::DhtPeerAcquirer, static_peer_acquirer::StaticPeerAcquirer,
    tracker_peer_acquirer::TrackerPeerAcquirer,
};

pub mod dht_peer_acquirer;
pub mod static_peer_acquirer;
pub mod tracker_peer_acquirer;

pub mod magnet_torrent_info_acquirer;

pub mod magnet_link;

pub(crate) struct PeerList(pub Vec<SocketAddrV4>);

impl Deref for PeerList {
    type Target = Vec<SocketAddrV4>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl Send for PeerList {}

// TODO MW: Figure out API for requesting new peers
#[async_trait]
pub trait PeerAcquirer {
    async fn try_get_peers(&mut self) -> Option<PeerList> {
        None
    }

    async fn get_peers(&mut self) -> PeerList {
        self.try_get_peers().await.unwrap()
    }
}

pub enum PeerAcquirerEnum {
    Tracker(TrackerPeerAcquirer),
    Dht(DhtPeerAcquirer),
    Static(StaticPeerAcquirer),
}

impl PeerAcquirerEnum {
    pub async fn try_get_peers(&mut self) -> Option<PeerList> {
        match self {
            PeerAcquirerEnum::Tracker(acquirer) => acquirer.try_get_peers().await,
            PeerAcquirerEnum::Dht(acquirer) => acquirer.try_get_peers().await,
            PeerAcquirerEnum::Static(acquirer) => acquirer.try_get_peers().await,
        }
    }
}
