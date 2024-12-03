use std::{collections::HashSet, net::SocketAddrV4};

use tokio::sync::mpsc;

use crate::parser::{metadata::Metadata, tracker_info::PeerInfo};

pub mod magnet_acquirer;
pub mod peer_info_feed;
pub mod tracker_acquirer;

// TODO: should we change this to hashset?
pub type PeerSet = Vec<SocketAddrV4>;

pub(crate) struct TorrentInfo {
    pub md: Metadata,
    pub init_peers: Vec<PeerInfo>,
    pub peers_chan: Option<mpsc::Receiver<PeerSet>>,
}

pub(crate) trait TorrentInfoAcquirer {
    async fn acquire(&self, torrent: String) -> Result<TorrentInfo, Box<dyn std::error::Error>>;
}
