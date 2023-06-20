use bendy::{
    decoding::{Error as DecError, FromBencode, ResultExt},
    encoding::{Error as EncError, ToBencode, Encoder},
};

use sha1::{Digest, Sha1};
use urlencoding::encode_binary;

use super::info::Info;

pub(crate) struct Metadata {
    pub announce: String,
    pub announce_list: Vec<Vec<String>>,
    pub info: Info,
    pub info_hash: Vec<u8>,
}

impl FromBencode for Metadata {
    const EXPECTED_RECURSION_DEPTH: usize = 5;

    fn decode_bencode_object(object: bendy::decoding::Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut announce: Option<String> = None;
        let mut announce_list: Option<Vec<Vec<String>>> = None;
        let mut info: Option<Info> = None;
        let mut info_hash: Option<Vec<u8>> = None;

        let mut dict = object.try_into_dictionary()?;

        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"announce", val) => {
                    announce = String::decode_bencode_object(val).context("announce").ok();
                }
                (b"announce-list", val) => {
                    announce_list = Vec::decode_bencode_object(val)
                        .context("announce-list")
                        .ok();
                }
                (b"info", val) => {
                    let raw = val.try_into_dictionary()?.into_raw()?;
                    let mut hasher: Sha1 = Sha1::new();
                    hasher.update(raw);
                    info_hash = Some(hasher.finalize().to_vec());

                    info = Info::from_bencode(raw)
                        .context("info")
                        .ok();
                }
                _ => {
                    continue;
                }
            }
        }

        let announce = announce.ok_or_else(|| DecError::missing_field("announce"))?;
        let announce_list =
            announce_list.ok_or_else(|| DecError::missing_field("announce-list"))?;
        let info = info.ok_or_else(|| DecError::missing_field("info"))?;
        let info_hash = info_hash.ok_or_else(|| DecError::missing_field("info_hash"))?;

        Ok(Metadata {
            announce,
            announce_list,
            info,
            info_hash
        })
    }
}

impl ToBencode for Metadata {
    const MAX_DEPTH: usize = 5;

    fn encode(&self, encoder: bendy::encoding::SingleItemEncoder) -> Result<(), EncError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"announce", &self.announce)?;
            e.emit_pair(b"announce-list", &self.announce_list)?;
            e.emit_pair(b"info", &self.info)
        })?;

        Ok(())
    }
}

pub(crate) fn read_metadata(path: &String) -> Result<Metadata, DecError> {
    let res = std::fs::read(path).expect("Failed to read file");
    let metadata = Metadata::from_bencode(&res)?;

    Ok(metadata)
}

//TODO: Directly compute info hash when computing metadata.
pub(crate) fn get_urlenc_info_hash(metadata: &Metadata) -> Result<String, EncError> {
    let bytes = metadata.info.to_bencode()?;

    let mut hasher: Sha1 = Sha1::new();
    hasher.update(bytes);
    let sha_hex: Vec<u8> = hasher.finalize().to_vec();
    let sha_url = encode_binary(&sha_hex);

    Ok(sha_url.to_string())
}