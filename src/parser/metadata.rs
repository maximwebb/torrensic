use bendy::{
    decoding::{Error as DecError, FromBencode, ResultExt},
    encoding::{Error as EncError, ToBencode},
};

use sha1::{Digest, Sha1};
use urlencoding::encode_binary;

use super::file_info::FileInfo;

pub(crate) struct Metadata {
    pub announce: Option<String>,
    pub announce_list: Vec<Vec<String>>,
    pub info: FileInfo,
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
        let mut info: Option<FileInfo> = None;
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

                    info = FileInfo::from_bencode(raw).context("info").ok();
                }
                _ => {
                    continue;
                }
            }
        }

        let announce_list =
            announce_list.ok_or_else(|| DecError::missing_field("announce-list"))?;
        let info = info.ok_or_else(|| DecError::missing_field("info"))?;
        let info_hash = info_hash.ok_or_else(|| DecError::missing_field("info_hash"))?;

        Ok(Metadata {
            announce,
            announce_list,
            info,
            info_hash,
        })
    }
}

impl ToBencode for Metadata {
    const MAX_DEPTH: usize = 5;

    fn encode(&self, encoder: bendy::encoding::SingleItemEncoder) -> Result<(), EncError> {
        encoder.emit_dict(|mut e| {
            match &self.announce {
                Some(announce) => e.emit_pair(b"announce", announce)?,
                None => {}
            };
            e.emit_pair(b"announce-list", &self.announce_list)?;
            e.emit_pair(b"info", &self.info)
        })?;

        Ok(())
    }
}

impl Metadata {
    pub fn num_pieces(&self) -> usize {
        return self.info.pieces.len() / 20;
    }

    pub fn num_blocks(&self) -> u32 {
        return (self.info.piece_length / (2 << 13)).try_into().unwrap();
    }

    pub fn block_len(&self, index: u32, block_index: u32) -> u32 {
        if index + 1 < self.num_pieces().try_into().unwrap() || block_index + 1 < self.num_blocks()
        {
            return 2 << 13;
        } else {
            let total_size = self.info.files.iter().map(|f| f.length).sum::<u32>();
            let block_size = (index * self.info.piece_length) + (self.num_blocks() - 1) * (2 << 13);
            return total_size - block_size;
        }
    }
}

pub(crate) fn read_metadata(path: &String) -> Result<Metadata, DecError> {
    let res = std::fs::read(path).expect("Failed to read file");
    let metadata = Metadata::from_bencode(&res)?;

    Ok(metadata)
}

pub(crate) fn get_urlenc_info_hash(metadata: &Metadata) -> Result<String, EncError> {
    let bytes = metadata.info.to_bencode()?;

    let mut hasher: Sha1 = Sha1::new();
    hasher.update(bytes);
    let sha_hex: Vec<u8> = hasher.finalize().to_vec();
    let sha_url = encode_binary(&sha_hex);

    Ok(sha_url.to_string())
}
