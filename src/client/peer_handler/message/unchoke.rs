use super::PeerWireMessage;

pub struct Unchoke {}

impl PeerWireMessage for Unchoke {
    fn id(&self) -> Option<u8> {
        Some(1)
    }

    fn name(&self) -> String {
        String::from("unchoke")
    }
}