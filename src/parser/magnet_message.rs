use std::net::{Ipv4Addr, SocketAddrV4};

use bendy::{
    decoding::{Error as DecError, FromBencode},
    encoding::{SingleItemEncoder, ToBencode},
};
use byteorder::{BigEndian, ReadBytesExt};

pub(crate) trait MagnetTopic {
    fn topic() -> String;
}

pub(crate) struct MagnetMessage<T: Clone> {
    pub payload: T,
}

impl<T: MagnetTopic + ToBencode + Clone> ToBencode for MagnetMessage<T> {
    const MAX_DEPTH: usize = 3;

    fn encode(
        &self,
        encoder: bendy::encoding::SingleItemEncoder,
    ) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"a", self.payload.clone())?;
            e.emit_pair(b"q", T::topic())?;
            e.emit_pair(b"t", "aa")?;
            e.emit_pair(b"y", "q")
        })?;

        Ok(())
    }
}

impl<T: FromBencode + Clone> FromBencode for MagnetMessage<T> {
    const EXPECTED_RECURSION_DEPTH: usize = 5;

    fn decode_bencode_object(
        object: bendy::decoding::Object,
    ) -> Result<Self, bendy::decoding::Error>
    where
        Self: Sized,
    {
        let mut payload: Option<T> = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"r", val) => {
                    let raw = val.try_into_dictionary()?.into_raw()?;
                    payload = T::from_bencode(raw).ok();
                }
                _ => {
                    continue;
                }
            }
        }
        let payload = payload.ok_or_else(|| DecError::missing_field("r"))?;
        Ok(MagnetMessage::<T> { payload })
    }
}

///////////////////////
// Ping Message

#[derive(Clone)]
pub(crate) struct Ping {
    pub id: Vec<u8>,
}

impl MagnetTopic for Ping {
    fn topic() -> String {
        String::from("ping")
    }
}

impl ToBencode for Ping {
    const MAX_DEPTH: usize = 2;

    fn encode(
        &self,
        encoder: bendy::encoding::SingleItemEncoder,
    ) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair_with(b"id", |e| e.emit_bytes(&self.id))?;
            Ok(())
        })?;

        Ok(())
    }
}

////////////////////////
// Get Peers Messages

#[derive(Clone)]
pub(crate) struct GetPeers {
    pub id: Vec<u8>,
    pub info_hash: Vec<u8>,
}

impl MagnetTopic for GetPeers {
    fn topic() -> String {
        String::from("get_peers")
    }
}

impl ToBencode for GetPeers {
    const MAX_DEPTH: usize = 2;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair_with(b"id", |e| e.emit_bytes(&self.id))?;
            e.emit_pair_with(b"info_hash", |e| e.emit_bytes(&self.info_hash))?;
            Ok(())
        })?;

        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct GetPeersResponse {
    pub id: Vec<u8>,
    pub nodes: Vec<SocketAddrV4>,
    pub peers: Vec<SocketAddrV4>,
}

impl FromBencode for GetPeersResponse {
    fn decode_bencode_object(
        object: bendy::decoding::Object,
    ) -> Result<Self, bendy::decoding::Error>
    where
        Self: Sized,
    {
        let mut id: Option<Vec<u8>> = None;
        let mut nodes = Vec::<SocketAddrV4>::new();
        let mut peers = Vec::<SocketAddrV4>::new();

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"id", val) => {
                    let raw = val.try_into_bytes()?;
                    id = Some(raw.to_vec());
                }
                (b"nodes", val) => {
                    let raw = val.try_into_bytes()?;
                    let len = (raw.len() / 6) * 6;

                    // TODO: write parsing utility for IP/port
                    let res = raw[..len]
                        .chunks_exact(6)
                        .map(|c| {
                            let mut ip_raw = &c[..4];
                            let mut port_raw = &c[4..6];
                            let ip = ip_raw.read_u32::<BigEndian>().unwrap();
                            let port = port_raw.read_u16::<BigEndian>().unwrap();
                            SocketAddrV4::new(Ipv4Addr::from_bits(ip), port)
                        })
                        .collect::<Vec<SocketAddrV4>>();
                    nodes = res;
                }
                (b"values", val) => {
                    let mut list = val.try_into_list()?;

                    while let Some(item) = list.next_object()? {
                        if let bendy::decoding::Object::Bytes(raw) = item {
                            let mut ip_raw = &raw[..4];
                            let mut port_raw = &raw[4..6];
                            let ip = ip_raw.read_u32::<BigEndian>().unwrap();
                            let port = port_raw.read_u16::<BigEndian>().unwrap();
                            peers.push(SocketAddrV4::new(Ipv4Addr::from_bits(ip), port));
                        } else {
                            continue;
                        }
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        let id = id.ok_or_else(|| DecError::missing_field("id"))?;
        if nodes.len() == 0 && peers.len() == 0
        {
            return Err(DecError::missing_field("endpoints"));
        }

        Ok(GetPeersResponse {
            id,
            nodes,
            peers
        })
    }
}

////////////////////////
// Endpoint Message

#[derive(Clone)]
pub(crate) struct Endpoint {
    pub ip: u32,
    pub port: u16,
}

impl FromBencode for Endpoint {
    const EXPECTED_RECURSION_DEPTH: usize = 2;

    fn decode_bencode_object(
        object: bendy::decoding::Object,
    ) -> Result<Self, bendy::decoding::Error>
    where
        Self: Sized,
    {
        let mut ip: Option<u32> = None;
        let mut port: Option<u16> = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"ip", val) => {
                    let raw = val.try_into_bytes()?;
                    let mut ip_raw = &raw[..4];
                    let mut port_raw = &raw[4..6];
                    ip = Some(ip_raw.read_u32::<BigEndian>()?);
                    port = Some(port_raw.read_u16::<BigEndian>()?);
                }
                _ => {
                    continue;
                }
            }
        }
        let ip = ip.ok_or_else(|| DecError::missing_field("ip"))?;
        let port = port.ok_or_else(|| DecError::missing_field("port"))?;
        Ok(Endpoint { ip, port })
    }
}
