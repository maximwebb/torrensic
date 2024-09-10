use crate::parser::{metadata::Metadata, tracker_info::PeerInfo};

pub mod tracker_acquirer;

pub(crate) struct TorrentInfo
{
    pub md: Metadata,
    pub peers: Vec<PeerInfo>
}

pub(crate) trait TorrentInfoAcquirer
{
    async fn acquire(&self, torrent_file: String) -> Result<TorrentInfo, Box<dyn std::error::Error>>;
}

// pub(crate) struct MagnetTorrentInfoAcquirer
// {

// }