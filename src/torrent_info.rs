use crate::parser::{metadata::Metadata, tracker_info::PeerInfo};

pub mod tracker_acquirer;
pub mod magnet_acquirer;

pub(crate) struct TorrentInfo
{
    pub md: Metadata,
    pub peers: Vec<PeerInfo>
}

pub(crate) trait TorrentInfoAcquirer
{
    async fn acquire(&self, torrent: String) -> Result<TorrentInfo, Box<dyn std::error::Error>>;
}