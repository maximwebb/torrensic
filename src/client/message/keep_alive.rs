use super::PeerWireMessage;

pub struct KeepAlive {}

// Contains no message ID or payload.
impl PeerWireMessage for KeepAlive {
    fn name(&self) -> String {
        String::from("keep-alive")
    }
}
