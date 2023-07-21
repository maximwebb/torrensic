use std::net::Ipv4Addr;

use bendy::{
    decoding::{Error as DecError, FromBencode, ResultExt},
    encoding::{Error as EncError, ToBencode},
};
use byteorder::{BigEndian, ReadBytesExt};

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

impl TrackerInfo {
    pub fn from_raw(raw: Vec<u8>) -> Result<Self, ()> {
        let mut interval = &raw[8..12];
        let mut leechers = &raw[12..16];
        let mut seeders = &raw[16..20];

        // TODO: include leecher/seeder info in TrackerInfo
        let interval = interval.read_u32::<BigEndian>().unwrap();
        let leechers = leechers.read_u32::<BigEndian>().unwrap();
        let seeders = seeders.read_u32::<BigEndian>().unwrap();

        let raw_peers: Vec<_> = raw[20..]
            .to_vec()
            .chunks_exact(6)
            .map(|v| v.to_vec())
            .collect();

        let mut peers: Vec<PeerInfo> = Vec::new();

        for chunk in raw_peers {
            let (mut ip, mut port) = chunk.split_at(4);
            let ip = Ipv4Addr::from(ip.read_u32::<BigEndian>().unwrap()).to_string();
            let port = port.read_u16::<BigEndian>().unwrap();

            peers.push(PeerInfo {
                ip,
                port,
                peer_id: None,
            });
        }

        Ok(TrackerInfo {
            interval,
            peers,
            tracker_id: None,
        })
    }
}

pub(crate) struct PeerInfo {
    pub peer_id: Option<Vec<u8>>,
    pub ip: String,
    pub port: u16,
}

impl FromBencode for PeerInfo {
    const EXPECTED_RECURSION_DEPTH: usize = 5;

    fn decode_bencode_object(object: bendy::decoding::Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut peer_id: Option<Vec<u8>> = None;
        let mut ip: Option<String> = None;
        let mut port: Option<u16> = None;

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
                    port = u16::decode_bencode_object(val).context("port").ok();
                }
                _ => {
                    continue;
                }
            }
        }

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
            match &self.peer_id {
                None => {}
                Some(id) => e.emit_pair(b"peer id", id)?,
            };
            e.emit_pair(b"port", self.port)
        })?;
        Ok(())
    }
}

impl PeerInfo {
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}