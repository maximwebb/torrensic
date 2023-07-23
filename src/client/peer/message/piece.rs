use super::PeerWireMessage;

pub struct Piece {
    pub index: u32,
    pub begin: u32,
    pub block: Vec<u8>,
}

impl PeerWireMessage for Piece {
    fn id(&self) -> Option<u8> {
        Some(7)
    }

    fn payload(&self) -> Vec<u8> {
        [
            self.index.to_be_bytes().to_vec(),
            self.begin.to_be_bytes().to_vec(),
            self.block.clone(),
        ]
        .concat()
    }

    fn name(&self) -> String {
        String::from("piece")
    }
}
