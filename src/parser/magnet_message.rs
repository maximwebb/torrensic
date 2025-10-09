use std::net::{Ipv4Addr, SocketAddrV4};

use bendy::{
    decoding::{Error as DecError, FromBencode, ResultExt},
    encoding::{SingleItemEncoder, ToBencode},
};
use byteorder::{BigEndian, ReadBytesExt};

use crate::{
    client::peer_handler::connection::{Deserialisable, Serialisable},
    log, log_err, utils,
};

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

#[derive(PartialEq, Debug)]
pub enum MetadataMessage {
    MetadataHandshake(MetadataHandshake),
    MetadataRequest(MetadataRequest),
    MetadataResponse(MetadataResponse),
}

impl Serialisable for MetadataMessage {
    fn serialise(&self) -> Vec<u8> {
        return match &self {
            MetadataMessage::MetadataHandshake(v) => {
                let payload = v.to_bencode().unwrap();
                let len: u32 = (payload.len() + 2).try_into().unwrap();
                let len_prefix = len.to_be_bytes().to_vec();

                let id: u8 = 20;

                return [len_prefix, vec![id, 0], payload].concat();
            }
            MetadataMessage::MetadataRequest(v) => v.to_bencode().unwrap(),
            MetadataMessage::MetadataResponse(v) => {
                let mut msg = v.to_bencode().unwrap();
                msg.extend(&v.data);
                msg
            }
        };
    }
}

/*
[0, 0, 0, 213, 20, 0, 100, 49, 50, 58, 99, 111, 109, 112, 108, 101, 116, 101, 95, 97, 103, 111, 105, 49, 49, 54, 101, 49, 58,
 109, 100, 49, 49, 58, 108, 116, 95, 100, 111, 110, 116, 104, 97, 118, 101, 105, 55, 101, 49, 48, 58, 115, 104, 97, 114, 101,
 95, 109, 111, 100, 101, 105, 56, 101, 49, 49, 58, 117, 112, 108, 111, 97, 100, 95, 111, 110, 108, 121, 105, 51, 101, 49, 50,
 58, 117, 116, 95, 104, 111, 108, 101, 112, 117, 110, 99, 104, 105, 52, 101, 49, 49, 58, 117, 116, 95, 109, 101, 116, 97, 100,
 97, 116, 97, 105, 50, 101, 54, 58, 117, 116, 95, 112, 101, 120, 105, 49, 101, 101, 49, 51, 58, 109, 101, 116, 97, 100, 97, 116,
 97, 95, 115, 105, 122, 101, 105, 50, 52, 51, 48, 54, 101, 52, 58, 114, 101, 113, 113, 105, 53, 48, 48, 101, 49, 49, 58, 117,
 112, 108, 111, 97, 100, 95, 111, 110, 108, 121, 105, 49, 101, 49, 58, 118, 49, 55, 58, 113, 66, 105, 116, 116, 111, 114, 114,
 101, 110, 116, 47, 52, 46, 54, 46, 51, 54, 58, 121, 111, 117, 114, 105, 112, 52, 58, 80, 1, 160, 54, 101, 0, 0, 0, 149, 5, 255,
 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 224]
*/

impl Deserialisable for MetadataMessage {
    fn deserialise(raw: &Vec<u8>) -> Result<(Option<Self>, Vec<u8>), ()>
    where
        Self: Sized,
    {
        if raw[0] != b'd' {
            // Attempt to parse handshake message
            let mut len_prefix: &[u8] = &raw[0..4];
            let len_prefix: usize = len_prefix
                .read_u32::<BigEndian>()
                .unwrap()
                .try_into()
                .unwrap();

            if len_prefix + 4 > raw.len() {
                return Ok((None, raw.to_vec()));
            }

            // Length prefix itself is 4B (and excluded from the message length), so offset end by 4
            let end: usize = len_prefix + 4;

            // Split into bencoded message and remaining bytes
            let msg_bytes = &raw[6..end];
            let rem = &raw[end..];

            return match MetadataHandshake::deserialise(&msg_bytes.to_vec()) {
                Ok((Some(v), r)) => {
                    log!("Got handshake!");
                    Ok((Some(MetadataMessage::MetadataHandshake(v)), rem.to_vec()))},
                Ok((None, _)) => {
                    log!("Did not parse handshake");
                    Ok((None, raw.to_vec()))},
                Err(e) => {
                    log_err!(
                        "{:?}\nMessage: {:?}\nUTF8: {}\n",
                        e,
                        msg_bytes,
                        String::from_utf8_lossy(&msg_bytes)
                    );
                    Err(())
                }
            };
        }

        let bencode_end = match raw
            .windows(2)
            .enumerate()
            .find(|(_, s)| b"ee" == s)
            .map(|(idx, _)| idx)
        {
            Some(v) => v + 2,
            None => return Err(()),
        };

        return match MetadataResponse::from_bencode(raw) {
            Ok(mut v) => {
                let total_size: usize = v.total_size.try_into().unwrap();
                v.data = raw[bencode_end..total_size].to_vec();
                let rem = raw[total_size..].to_vec();

                Ok((Some(MetadataMessage::MetadataResponse(v)), rem))
            }
            Err(e) => {
                log_err!(
                    "{:?}\nMessage: {:?}\nUTF8: {}\n",
                    e,
                    raw,
                    String::from_utf8_lossy(&raw)
                );
                Err(())
            }
        };
        // TODO: handle other cases
    }
}

// Handshake send by peer to indicate size of metadata info hash
#[derive(PartialEq, Debug)]
pub struct MetadataHandshake {
    pub msg_code: u32,
    pub size: u32,
}

// TODO: Make this a parse utility
impl Deserialisable for MetadataHandshake {
    fn deserialise(raw: &Vec<u8>) -> Result<(Option<Self>, Vec<u8>), ()>
    where
        Self: Sized,
    {
        let key = b"metadata_size";
        let s = String::from_utf8_lossy(raw);
        log!("{:?}", s);
        let md_size_key = match raw.windows(key.len()).position(|w| w == key) {
            Some(v) => v + key.len(), // Add key.len() to search from after the key
            None => return Err(()),
        };

        let md_size_start = match raw[md_size_key..].iter().position(|&v| v == b'i') {
            Some(v) => v + md_size_key, // Add md_size_key offset to account for relative position
            None => return Err(()),
        };
        let md_size_end = match raw[md_size_key..].iter().position(|&v| v == b'e') {
            Some(v) => v + md_size_key,
            None => return Err(()),
        };

        let md_size = match String::from_utf8((&raw[md_size_start + 1..md_size_end]).to_vec()) {
            Ok(s) => match s.parse::<u32>() {
                Ok(v) => v,
                Err(_) => return Err(()),
            },
            Err(_) => return Err(()),
        };

        Ok((
            Some(Self {
                msg_code: 0,
                size: md_size,
            }),
            Vec::new(),
        ))
    }
    // fn decode_bencode_object(object: bendy::decoding::Object) -> Result<Self, DecError>
    // where
    //     Self: Sized,
    // {
    //     let mut msg_code: Option<u32> = None;
    //     let mut size: Option<u32> = None;

    //     let mut dict = object.try_into_dictionary()?;
    //     let mut errs = 0;
    //     while errs < 10 {
    //         let pair = dict.next_pair();
    //         let pair = match pair {
    //             Ok(Some(val)) => val,
    //             Ok(None) => break,
    //             Err(e) => {
    //                 log_err!("Got: {:?}, skipping", e);
    //                 errs += 1;
    //                 continue;
    //             },
    //         };
    //         match pair {
    //             (b"metadata_size", v) => {
    //                 size = u32::decode_bencode_object(v)
    //                     .context("metadata_size")
    //                     .map(Some)?;
    //             }
    //             (b"m", v) => {
    //                 // Parse inner dictionary of extensions
    //                 let mut ext_dict = v.try_into_dictionary()?;
    //                 while let Some(pair_) = ext_dict.next_pair()? {
    //                     match pair_ {
    //                         (b"ut_metadata", val) => {
    //                             msg_code = u32::decode_bencode_object(val)
    //                                 .context("ut_metadata: msg_code")
    //                                 .map(Some)?;
    //                             break;
    //                         }
    //                         _ => continue,
    //                     }
    //                 }
    //             }
    //             _ => continue,
    //         }
    //     }

    //     let size = size.ok_or_else(|| DecError::missing_field("size"))?;
    //     let msg_code = msg_code.ok_or_else(|| DecError::missing_field("msg_code"))?;

    //     Ok(Self { msg_code, size })
    // }
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

        Ok(Self { piece_index })
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

// Response message from a metadata request
#[derive(PartialEq, Debug)]
pub struct MetadataResponse {
    pub piece_index: u32,
    pub total_size: u32,
    pub data: Vec<u8>,
}

// The From/ToBencode implementations here solely deal with the bencoded part of the message - the full (de)serialisation
// is carried out in MetadataMessage
impl FromBencode for MetadataResponse {
    fn decode_bencode_object(object: bendy::decoding::Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut piece_index: Option<u32> = None;
        let mut total_size: Option<u32> = None;

        let mut dict = object.try_into_dictionary()?;

        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"piece", v) => {
                    piece_index = u32::decode_bencode_object(v)
                        .context("piece index")
                        .map(Some)?;
                }
                (b"total_size", v) => {
                    total_size = u32::decode_bencode_object(v)
                        .context("total size")
                        .map(Some)?;
                }
                _ => continue,
            }
        }

        let piece_index = piece_index.ok_or_else(|| DecError::missing_field("piece"))?;
        let total_size = total_size.ok_or_else(|| DecError::missing_field("piece"))?;

        Ok(Self {
            piece_index,
            total_size,
            data: Vec::new(),
        })
    }
}

impl ToBencode for MetadataResponse {
    const MAX_DEPTH: usize = 3;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), bendy::encoding::Error> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"msg_type", 1)?;
            e.emit_pair(b"piece", self.piece_index)?;
            e.emit_pair(b"total_size", self.total_size)?;
            Ok(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Verifies that serialising a message, then parsing it from the bytes perserves the original message
    fn serialise_then_deserialise_preserves(msg: &MetadataMessage, msg_type: &str) {
        let bytes = msg.serialise();
        log!("{:?}", bytes);

        let res = MetadataMessage::deserialise(&bytes);

        let (parsed_msg, rem) = res.expect(&format!("Failed to deserialise {} message", msg_type));

        let parsed_msg = parsed_msg.expect(&format!("Got None when parsing {} message", msg_type));

        assert_eq!(
            msg, &parsed_msg,
            "Got different result when deserialising {} message",
            msg_type
        );
        assert!(
            rem.is_empty(),
            "Got non-empty remainder when deserialising {} message",
            msg_type
        );
    }

    // Verifies that parsing a message, then serialising it preserves the original bytes
    fn deserialise_then_serialise_preserves(raw: &Vec<u8>, msg_type: &str) {
        let (msg, rem) = MetadataMessage::deserialise(&raw)
            .expect(&format!("Failed to deserialise {} message", msg_type));

        let msg = msg.expect(&format!("Got None when parsing {} message", msg_type));

        let msg_bytes = msg.serialise();

        let all_bytes = [msg_bytes, rem].concat();

        assert_eq!(
            raw, &all_bytes,
            "Got different result when serialising {} message",
            msg_type
        );
    }

    #[test]
    fn serialise_then_parse_preserves_handshake() {
        let handshake = MetadataMessage::MetadataHandshake(MetadataHandshake {
            msg_code: 4,
            size: 42,
        });

        serialise_then_deserialise_preserves(&handshake, "handshake");
    }

    #[test]
    pub fn parse_then_serialise_handshake() {
        let raw: Vec<u8> = vec![
            0, 0, 0, 46, 20, 0, 100, 49, 58, 109, 100, 49, 49, 58, 117, 116, 95, 109, 101, 116, 97,
            100, 97, 116, 97, 105, 52, 101, 101, 49, 51, 58, 109, 101, 116, 97, 100, 97, 116, 97,
            95, 115, 105, 122, 101, 105, 52, 50, 101, 101, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
            13, 14, 15, 16, 17, 18, 19, 20,
        ];

        deserialise_then_serialise_preserves(&raw, "handshake");
    }

    #[test]
    pub fn parse_then_serialise_other() {
        let raw: Vec<u8> = vec![
            0, 0, 1, 1, 20, 0, 100, 49, 58, 101, 105, 48, 101, 52, 58, 105, 112, 118, 52, 52, 58,
            70, 70, 33, 242, 52, 58, 105, 112, 118, 54, 49, 54, 58, 38, 4, 61, 8, 148, 130, 213, 0,
            0, 0, 0, 0, 0, 0, 84, 194, 49, 50, 58, 99, 111, 109, 112, 108, 101, 116, 101, 95, 97,
            103, 111, 105, 49, 48, 101, 49, 58, 109, 100, 49, 49, 58, 117, 112, 108, 111, 97, 100,
            95, 111, 110, 108, 121, 105, 51, 101, 49, 49, 58, 108, 116, 95, 100, 111, 110, 116,
            104, 97, 118, 101, 105, 55, 101, 49, 50, 58, 117, 116, 95, 104, 111, 108, 101, 112,
            117, 110, 99, 104, 105, 52, 101, 49, 49, 58, 117, 116, 95, 109, 101, 116, 97, 100, 97,
            116, 97, 105, 50, 101, 54, 58, 117, 116, 95, 112, 101, 120, 105, 49, 101, 49, 48, 58,
            117, 116, 95, 99, 111, 109, 109, 101, 110, 116, 105, 54, 101, 101, 49, 51, 58, 109,
            101, 116, 97, 100, 97, 116, 97, 95, 115, 105, 122, 101, 105, 49, 55, 50, 56, 53, 101,
            49, 58, 112, 105, 50, 51, 50, 56, 51, 101, 52, 58, 114, 101, 113, 113, 105, 50, 53, 53,
            101, 49, 58, 118, 49, 53, 58, 206, 188, 84, 111, 114, 114, 101, 110, 116, 32, 51, 46,
            52, 46, 50, 50, 58, 121, 112, 105, 52, 53, 55, 54, 56, 101, 54, 58, 121, 111, 117, 114,
            105, 112, 52, 58, 80, 1, 160, 54, 101, 0, 0, 0, 105, 5, 255, 223, 191, 255, 253, 255,
            255, 255, 239, 223, 255, 255, 223, 255, 255, 255, 255, 255, 255, 127, 255, 255, 255,
            255, 255, 246, 255, 255, 255, 255, 255, 255, 239, 255, 255, 253, 255, 255, 255, 255,
            255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 239, 255, 255, 123, 255, 255, 251, 255, 255, 255, 239, 255,
            255, 255, 223, 255, 255, 255, 255, 255, 255, 255, 239, 251, 255, 127, 253, 255, 255,
            255, 255, 255, 255, 255, 254, 255, 255, 255, 239, 255, 255, 128, 0, 0, 0, 5, 4, 0, 0,
            0, 74, 0, 0, 0, 5, 4, 0, 0, 1, 30, 0, 0, 0, 5, 4, 0, 0, 3, 7, 0, 0, 0, 5, 4, 0, 0, 0,
            204, 0, 0, 0, 5, 4, 0, 0, 0, 67, 0, 0, 0, 5, 4, 0, 0, 1, 243, 0, 0, 0, 5, 4, 0, 0, 3,
            35, 0, 0, 0, 5, 4, 0, 0, 1, 87, 0, 0, 0, 5, 4, 0, 0, 2, 8, 0, 0, 0, 5, 4, 0, 0, 2, 163,
            0, 0, 0, 5, 4, 0, 0, 2, 173, 0, 0, 0, 5, 4, 0, 0, 0, 38, 0, 0, 0, 5, 4, 0, 0, 2, 184,
            0, 0, 0, 5, 4, 0, 0, 1, 3, 0, 0, 0, 5, 4, 0, 0, 0, 10, 0, 0, 0, 5, 4, 0, 0, 0, 152, 0,
            0, 0, 5, 4, 0, 0, 2, 37, 0, 0, 0, 5, 4, 0, 0, 2, 98, 0, 0, 0, 5, 4, 0, 0, 2, 67, 0, 0,
            0, 5, 4, 0, 0, 0, 207, 0, 0, 0, 5, 4, 0, 0, 0, 98, 0, 0, 0, 5, 4, 0, 0, 2, 13, 0, 0, 0,
            5, 4, 0, 0, 0, 17, 0, 0, 0, 5, 4, 0, 0, 2, 198,
        ];

        deserialise_then_serialise_preserves(&raw, "other");
    }
}
