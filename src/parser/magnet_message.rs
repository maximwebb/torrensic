use std::net::Ipv4Addr;

use bendy::{
    decoding::{Error as DecError, FromBencode},
    encoding::{SingleItemEncoder, ToBencode},
};
use byteorder::{BigEndian, ReadBytesExt};

pub(crate) trait MagnetTopic {
    fn topic() -> String;
}

pub(crate) struct MagnetMessage<T: MagnetTopic + ToBencode + Clone> {
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
