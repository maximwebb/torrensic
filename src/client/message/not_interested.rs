use super::PeerWireMessage;

pub struct NotInterested {}

impl PeerWireMessage for NotInterested {
    fn id(&self) -> Option<u8> {
        Some(3)
    }

    fn name(&self) -> String {
        String::from("not interested")
    }
}