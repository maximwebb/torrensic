use std::sync::Arc;

use bitvec::{vec::BitVec, prelude::Msb0};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::parser::metadata::Metadata;

use self::wire_protocol_task::{WireProtocolTask, run_proto_task};

use super::peer_manager::BitVecMutex;

mod connection;
mod message;
mod wire_protocol_task;

pub struct Peer {
    sender: mpsc::Sender<Command>,
}

impl Peer {
    pub(crate) fn new(addr: &str, md: Arc<Metadata>, output_dir: Arc<str>, client_pieces: BitVecMutex) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let proto_task = WireProtocolTask::new(receiver, md, addr, output_dir, client_pieces);
        tokio::spawn(run_proto_task(proto_task));

        Peer {
            sender,
        }
    }
}

pub(crate) struct PeerState {
    client_choked: bool,
    client_interested: bool,
    peer_choked: bool,
    peer_interested: bool,
}

pub(crate) enum Command {
    SetClientState {
        choke: bool,
        interested: bool,
    },
    GetPeerState {
        respond_chan: oneshot::Sender<PeerState>,
    },
}
