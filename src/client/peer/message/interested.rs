use super::PeerWireMessage;

pub struct Interested {}

impl PeerWireMessage for Interested {
    fn id(&self) -> Option<u8> {
        Some(2)
    }

    fn name(&self) -> String {
        String::from("interested")
    }
}