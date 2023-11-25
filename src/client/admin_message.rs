use std::sync::Arc;

use tokio::sync::oneshot;

pub(crate) enum AdminMessage {
    PeerBitfield(PeerBitfield),
    PieceIndexRequest(PieceIndexRequest),
    PieceDownload(PieceDownload),
    PeerDisconnect(PeerDisconnect),
}

pub(crate) struct PeerBitfield {
    pub ack: oneshot::Sender<()>,
    pub addr: Arc<str>,
    pub peer_bitfield: Vec<bool>,
}

pub(crate) struct PieceIndexRequest {
    pub chan: oneshot::Sender<Option<u32>>,
    pub addr: Arc<str>,
}

pub(crate) struct PieceDownload {
    pub index: u32,
}

pub(crate) struct PeerDisconnect {
    pub addr: Arc<str>,
}
