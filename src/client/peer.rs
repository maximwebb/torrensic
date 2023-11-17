use std::sync::Arc;

use bitvec::{vec::BitVec, prelude::Msb0};
use tokio::sync::{mpsc, oneshot};

use crate::parser::metadata::Metadata;

use self::wire_protocol_task::{run_proto_task, WireProtocolTask};

use super::peer_manager::BitVecMutex;

mod connection;
mod message;
mod wire_protocol_task;

//TODO is this class even needed? Could the wire_protocol_task be renamed to this?
pub struct Peer {}

impl Peer {
    pub(crate) fn new(
        addr: &str,
        md: Arc<Metadata>,
        output_dir: Arc<str>,
        client_pieces: BitVecMutex,
        tx_piece_index_req: mpsc::Sender<PieceIndexRequest>,
        tx_peer_disconnect: mpsc::Sender<PeerDisconnect>,
    ) -> Self {
        let proto_task = WireProtocolTask::new(
            md,
            addr,
            output_dir,
            client_pieces,
            tx_piece_index_req,
            tx_peer_disconnect,
        );
        tokio::spawn(run_proto_task(proto_task));

        Peer {}
    }
}

pub(crate) struct PieceIndexRequest {
    pub chan: oneshot::Sender<Option<u32>>,
    pub peer_bitfield: BitVec<u8, Msb0>,
}

pub(crate) struct PeerDisconnect {
    pub addr: Arc<str>,
}

pub(crate) struct PeerState {
    client_choked: bool,
    client_interested: bool,
    peer_choked: bool,
    peer_interested: bool,
}
