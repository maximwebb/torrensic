use bendy::{
    decoding::{Error as DecError, FromBencode, ResultExt},
    encoding::{Error as EncError, ToBencode},
};

pub(crate) struct TrackerInfo {
    pub interval: u32,
    pub tracker_id: Option<String>,
    pub peers: Vec<PeerInfo>,
}

impl FromBencode for TrackerInfo {
    const EXPECTED_RECURSION_DEPTH: usize = 5;

    fn decode_bencode_object(object: bendy::decoding::Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut interval: Option<u32> = None;
        let mut tracker_id: Option<String> = None;
        let mut peers: Option<Vec<PeerInfo>> = None;

        let mut dict = object.try_into_dictionary()?;

        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"interval", val) => {
                    interval = u32::decode_bencode_object(val).context("interval").ok();
                }
                (b"tracker id", val) => {
                    tracker_id = String::decode_bencode_object(val)
                        .context("tracker id")
                        .ok();
                }
                (b"peers", val) => {
                    peers = Vec::decode_bencode_object(val).context("peers").ok();
                }
                _ => {
                    continue;
                }
            }
        }

        let interval = interval.ok_or_else(|| DecError::missing_field("interval"))?;
        let peers = peers.ok_or_else(|| DecError::missing_field("peers"))?;

        Ok(TrackerInfo {
            interval,
            tracker_id,
            peers,
        })
    }
}

impl ToBencode for TrackerInfo {
    const MAX_DEPTH: usize = 5;

    fn encode(&self, encoder: bendy::encoding::SingleItemEncoder) -> Result<(), EncError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"interval", self.interval)?;
            match &self.tracker_id {
                Some(id) => e.emit_pair(b"tracker id", id)?,
                None => {}
            };
            e.emit_pair(b"peers", &self.peers)
        })?;
        Ok(())
    }
}

pub(crate) struct PeerInfo {
    pub peer_id: Vec<u8>,
    pub ip: String,
    pub port: u32,
}

impl FromBencode for PeerInfo {
    const EXPECTED_RECURSION_DEPTH: usize = 5;

    fn decode_bencode_object(object: bendy::decoding::Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut peer_id: Option<Vec<u8>> = None;
        let mut ip: Option<String> = None;
        let mut port: Option<u32> = None;

        let mut dict = object.try_into_dictionary()?;

        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"ip", val) => {
                    ip = String::decode_bencode_object(val)
                        .context("tracker id")
                        .ok();
                }
                (b"peer id", val) => {
                    let a = val.try_into_bytes();
                    let a = a.map(Vec::from).ok();
                    peer_id = a;
                }
                (b"port", val) => {
                    port = u32::decode_bencode_object(val).context("port").ok();
                }
                _ => {
                    continue;
                }
            }
        }

        let peer_id = peer_id.ok_or_else(|| DecError::missing_field("peer id"))?;
        let port = port.ok_or_else(|| DecError::missing_field("port"))?;
        let ip: String = ip.ok_or_else(|| DecError::missing_field("ip"))?;

        Ok(PeerInfo { peer_id, ip, port })
    }
}

impl ToBencode for PeerInfo {
    const MAX_DEPTH: usize = 5;

    fn encode(&self, encoder: bendy::encoding::SingleItemEncoder) -> Result<(), EncError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"ip", &self.ip)?;
            e.emit_pair(b"peer id", &self.peer_id)?;
            e.emit_pair(b"port", self.port)
        })?;
        Ok(())
    }
}
