use super::PeerWireMessage;

pub struct Request {
    pub index: u32,
    pub begin: u32,
    pub length: u32,
}

impl PeerWireMessage for Request {
    fn id(&self) -> Option<u8> {
        Some(6)
    }

    fn payload(&self) -> Vec<u8> {
        [self.index, self.begin, self.length]
            .map(u32::to_be_bytes)
            .concat()
    }

    fn name(&self) -> String {
        String::from("request")
    }
}
