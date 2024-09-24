use std::net::SocketAddrV4;

use tokio::sync::oneshot;

pub(crate) enum AdminMessage {
    NodeAddressRequest(NodeAddressRequest),
    AddressList(AddressList),
}

pub(crate) struct NodeAddressRequest {
    pub chan: oneshot::Sender<Option<SocketAddrV4>>,
}

pub(crate) struct AddressList {
    pub ack: oneshot::Sender<()>,
    pub peers: Vec<SocketAddrV4>,
    pub nodes: Vec<SocketAddrV4>,
    pub id: Vec<u8>,
}
