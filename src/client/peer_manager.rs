use std::sync::Arc;

use bitvec::{prelude::Msb0, vec::BitVec};
use tokio::sync::Mutex;

use crate::{
    builder::file_builder,
    parser::{metadata::Metadata, trackerinfo::PeerInfo},
};

use super::peer::Peer;

pub(crate) type BitVecMutex = Arc<Mutex<BitVec<u8, Msb0>>>;

pub(crate) struct PeerManager {
    md: Arc<Metadata>,
    peers: Vec<Peer>,
    output_dir: Arc<str>,
    client_pieces: BitVecMutex,
}

impl PeerManager {
    // Start all threads on creation
    pub(crate) fn new(
        md: Metadata,
        peers: Vec<PeerInfo>,
        output_dir: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let client_pieces: BitVec<u8, Msb0> = file_builder::load_bitfield(&md, &output_dir)?;
        let client_pieces_ref = Arc::new(Mutex::new(client_pieces));

        let md_ref = Arc::new(md);
        let dir_ref: Arc<str> = Arc::from(output_dir);

        let peers = peers[..]
            .iter()
            .map(|peer| {
                Peer::new(
                    &peer.to_string(),
                    md_ref.clone(),
                    dir_ref.clone(),
                    client_pieces_ref.clone(),
                )
            })
            .collect();

        Ok(PeerManager {
            md: md_ref,
            peers,
            output_dir: dir_ref,
            client_pieces: client_pieces_ref,
        })
    }

    pub(crate) async fn run(&self) {
        loop {}
    }
}
