use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::parser::metadata::Metadata;

use self::wire_protocol_task::{WireProtocolTask, run_proto_task};

mod connection;
mod message;
mod wire_protocol_task;

//TODO: get basic task spawning working
pub struct Peer {
    sender: mpsc::Sender<Command>,
}

impl Peer {
    pub(crate) fn new(addr: &str, md: Arc<Metadata>, output_dir: &String) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let proto_task = WireProtocolTask::new(receiver);
        tokio::spawn(run_proto_task(proto_task, addr, md, output_dir));

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
