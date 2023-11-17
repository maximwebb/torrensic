use super::PeerWireMessage;

pub struct Choke {}

impl PeerWireMessage for Choke {
    fn id(&self) -> Option<u8> {
        Some(0)
    }

    fn name(&self) -> String {
        String::from("choke")
    }
}