use super::PeerWireMessage;

pub struct Bitfield {
    pub bitfield: Vec<u8>
}

impl PeerWireMessage for Bitfield {
    fn id(&self) -> Option<u8> {
        Some(5)
    }

    fn payload(&self) -> Vec<u8> {
        self.bitfield.clone()
    }

    fn name(&self) -> String {
        String::from("bitfield")
    }
}