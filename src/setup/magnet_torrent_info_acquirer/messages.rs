
use std::net::SocketAddrV4;

use bendy::{
    decoding::{Error as DecError, FromBencode, Object, ResultExt},
    encoding::{SingleItemEncoder, ToBencode},
};
use byteorder::{BigEndian, ReadBytesExt};

use crate::utils;

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
                _ => continue,
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

                    let res = raw[..len]
                        .chunks_exact(6)
                        .map(|c| utils::addr_from_bytes(c).unwrap())
                        .collect::<Vec<SocketAddrV4>>();
                    nodes = res;
                }
                (b"values", val) => {
                    let mut list = val.try_into_list()?;

                    while let Some(item) = list.next_object()? {
                        if let bendy::decoding::Object::Bytes(raw) = item {
                            peers.push(utils::addr_from_bytes(raw).unwrap());
                        } else {
                            continue;
                        }
                    }
                }
                _ => continue,
            }
        }

        let id = id.ok_or_else(|| DecError::missing_field("id"))?;
        if nodes.len() == 0 && peers.len() == 0 {
            return Err(DecError::missing_field("endpoints"));
        }

        Ok(GetPeersResponse { id, nodes, peers })
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
                _ => continue,
            }
        }
        let ip = ip.ok_or_else(|| DecError::missing_field("ip"))?;
        let port = port.ok_or_else(|| DecError::missing_field("port"))?;
        Ok(Endpoint { ip, port })
    }
}

////////////////////////
// Metadata Protocol

pub fn serialise_magnet_msg(msg: &impl ToBencode, msg_code: u8) -> Vec<u8> {
    let payload = msg.to_bencode().unwrap();
    let id: u8 = 20;
    let len: u32 = (payload.len() + 2).try_into().unwrap();
    let len_prefix = len.to_be_bytes().to_vec();
    vec![len_prefix, vec![id, msg_code], payload].concat()
}

// Handshake send by peer to indicate size of metadata info hash
#[derive(PartialEq, Debug)]
pub struct MetadataHandshake {
    pub msg_code: u8,
    pub size: u32,
}

impl FromBencode for MetadataHandshake {
    fn decode_bencode_object(object: Object) -> Result<Self, DecError> {
        let mut msg_code: Option<u8> = None;
        let mut size: Option<u32> = None;

        let mut dict = object.try_into_dictionary()?;
        while let Some((key, value)) = dict.next_pair()? {
            match key {
                b"m" => {
                    let mut m_dict = value.try_into_dictionary()?;
                    while let Some((k, v)) = m_dict.next_pair()? {
                        if k == b"ut_metadata" {
                            msg_code = u8::decode_bencode_object(v)
                                .context("ut_metadata: msg_code")
                                .map(Some)?;
                        }
                    }
                }
                b"metadata_size" => {
                    size = u32::decode_bencode_object(value)
                        .context("metadata_size")
                        .map(Some)?;
                }
                _ => {
                    continue;
                }
            }
        }

        Ok(MetadataHandshake {
            msg_code: msg_code.unwrap_or(0),
            size: size.unwrap_or(0),
        })
    }
}

impl ToBencode for MetadataHandshake {
    const MAX_DEPTH: usize = 4;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair_with(b"m", |e| {
                e.emit_dict(|mut e| e.emit_pair(b"ut_metadata", self.msg_code))
            })?;
            e.emit_pair(b"metadata_size", self.size)?;
            Ok(())
        })?;

        Ok(())
    }
}

// Request a piece of metadata
#[derive(PartialEq, Debug)]
pub struct MetadataRequest {
    pub piece_index: u32,
    pub msg_code: u8,
}

impl FromBencode for MetadataRequest {
    fn decode_bencode_object(object: bendy::decoding::Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut piece_index: Option<u32> = None;

        let mut dict = object.try_into_dictionary()?;

        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"piece", v) => {
                    piece_index = u32::decode_bencode_object(v)
                        .context("piece index")
                        .map(Some)?;
                }
                _ => continue,
            }
        }

        let piece_index = piece_index.ok_or_else(|| DecError::missing_field("piece"))?;

        Ok(Self {
            piece_index,
            msg_code: 0,
        })
    }
}

impl ToBencode for MetadataRequest {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"msg_type", 0)?;
            e.emit_pair(b"piece", self.piece_index)?;
            Ok(())
        })?;

        Ok(())
    }
}

// Consists solely of the bencoded dictionary at the start of the metadata response
#[derive(PartialEq, Debug)]
pub(crate) struct MetadataResponseHeader {
    pub(crate) piece: u32,
    pub(crate) total_size: u32,
}

// Metadata response, including the actual byte data
#[derive(PartialEq, Debug)]
pub struct MetadataResponse {
    pub header: MetadataResponseHeader,
    pub payload: Vec<u8>,
}

impl FromBencode for MetadataResponseHeader {
    fn decode_bencode_object(object: bendy::decoding::Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut piece: Option<u32> = None;
        let mut total_size: Option<u32> = None;

        let mut dict = object.try_into_dictionary()?;

        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"piece", v) => {
                    piece = u32::decode_bencode_object(v).context("piece").map(Some)?;
                }
                (b"total_size", v) => {
                    total_size = u32::decode_bencode_object(v)
                        .context("total size")
                        .map(Some)?;
                }
                _ => continue,
            }
        }

        let piece = piece.ok_or_else(|| DecError::missing_field("piece"))?;
        let total_size = total_size.ok_or_else(|| DecError::missing_field("total_size"))?;

        Ok(Self { piece, total_size })
    }
}

impl ToBencode for MetadataResponseHeader {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"msg_type", 1)?;
            e.emit_pair(b"piece", self.piece)?;
            e.emit_pair(b"total_size", self.total_size)?;
            Ok(())
        })?;
        Ok(())
    }
}

impl MetadataResponse {
    pub(crate) fn deserialise(raw: &Vec<u8>) -> Result<Self, DecError> {
        let bencode_end = raw
            .windows(2)
            .enumerate()
            .find(|(_, v)| v == b"ee")
            .map(|(idx, _)| idx + 2)
            .ok_or_else(|| DecError::missing_field("Could not find bencoded dict end"))?;

        let header = MetadataResponseHeader::from_bencode(&raw[..bencode_end])?;

        let payload = raw[bencode_end..].to_vec();
        Ok(MetadataResponse { header, payload })
    }
}

#[cfg(test)]
mod test {
}
