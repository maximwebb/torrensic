use crate::parser::magnet_message::MetadataHandshake;

use super::PeerWireMessage;

pub struct Extended {
    pub inner: MetadataHandshake,
}

impl PeerWireMessage for Extended {
    fn id(&self) -> Option<u8> {
        Some(20)
    }

    fn payload(&self) -> Vec<u8> {
        return Vec::new();
    }

    fn name(&self) -> String {
        String::from("extended")
    }
}
