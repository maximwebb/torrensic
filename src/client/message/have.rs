use super::PeerWireMessage;

pub struct Have {
    pub piece_index: usize
}

impl PeerWireMessage for Have {
    fn id(&self) -> Option<u8> {
        Some(4)
    }

    fn payload(&self) -> Vec<u8> {
        self.piece_index.to_be_bytes().to_vec()
    }

    fn name(&self) -> String {
        String::from("have")
    }
}