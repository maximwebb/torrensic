use std::sync::Arc;

use crate::parser::{trackerinfo::PeerInfo, metadata::Metadata};

use super::peer::Peer;

struct PeerManager {
    md: Arc<Metadata>,
    peers: Vec<Peer>,
    output_dir: Arc<String>,
}

impl PeerManager {
    // TODO: change addr and output_dir to Arc's
    fn new(md: Metadata, peers: Vec<PeerInfo>, output_dir: String) -> Self {
        let md_ref = Arc::new(md);
        let dir_ref = Arc::new(output_dir);
        let peers = peers.iter().map(|peer| Peer::new(&peer.to_string(), md_ref.clone(), dir_ref.clone())).collect();
        PeerManager{md: md_ref, peers, output_dir: dir_ref}
    }
}