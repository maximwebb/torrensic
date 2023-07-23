use std::sync::Arc;

use crate::parser::{metadata::Metadata, trackerinfo::PeerInfo};

use super::peer::Peer;

pub(crate) struct PeerManager {
    md: Arc<Metadata>,
    peers: Vec<Peer>,
    output_dir: Arc<str>,
}

impl PeerManager {
    // Start all threads on creation
    pub(crate) fn new(md: Metadata, peers: Vec<PeerInfo>, output_dir: &str) -> Self {
        let md_ref = Arc::new(md);
        let dir_ref: Arc<str> = Arc::from(output_dir);
        let peers = peers[..1]
            .iter()
            .map(|peer| Peer::new(&peer.to_string(), md_ref.clone(), dir_ref.clone()))
            .collect();
        PeerManager {
            md: md_ref,
            peers,
            output_dir: dir_ref,
        }
    }

    pub(crate) async fn run(&self) {
        loop {

        }
    }
}
