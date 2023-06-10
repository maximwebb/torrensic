use bendy::{
    decoding::{Decoder, Error as DecError, FromBencode, Object, ResultExt},
    encoding::{AsString, Error as EncError, SingleItemEncoder, ToBencode},
};

use sha1::{Digest, Sha1};
use urlencoding::encode_binary;

use super::fileinfo::FileInfo;

pub(crate) struct MetaData {
    pub files: Vec<FileInfo>,
    pub name: String,
    pub piece_length: u32,
    pub pieces: Vec<u8>,
    pub private: u32,
}

impl FromBencode for MetaData {
    const EXPECTED_RECURSION_DEPTH: usize = 4;

    fn decode_bencode_object(object: Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut files: Option<Vec<FileInfo>> = None;
        let mut name: Option<String> = None;
        let mut pieces: Option<Vec<u8>> = None;
        let mut piece_length: Option<u32> = None;
        let mut private: Option<u32> = None;

        let mut dict = match object.try_into_dictionary() {
            Ok(v) => v,
            Err(e) => {
                return Err(e);
            }
        };

        while let Some(pair) = dict.next_pair().unwrap() {
            match pair {
                (b"files", val) => {
                    files = Vec::decode_bencode_object(val).ok();
                }
                (b"name", val) => {
                    name = String::decode_bencode_object(val).ok();
                }
                (b"pieces", val) => {
                    pieces = val.try_into_bytes().map(Vec::from).map(Some)?;
                }
                (b"piece length", val) => {
                    piece_length = u32::decode_bencode_object(val)
                        .context("piece length")
                        .map(Some)?;
                }
                (b"private", val) => {
                    private = u32::decode_bencode_object(val)
                        .context("private")
                        .map(Some)?;
                }
                _ => {
                    continue;
                }
            }
        }

        let files = files.ok_or_else(|| DecError::missing_field("files"))?;
        let name = name.ok_or_else(|| DecError::missing_field("name"))?;
        let pieces = pieces.ok_or_else(|| DecError::missing_field("pieces"))?;
        let piece_length = piece_length.ok_or_else(|| DecError::missing_field("piece_length"))?;
        let private = private.ok_or_else(|| DecError::missing_field("private"))?;

        Ok(MetaData {
            files,
            name,
            piece_length,
            pieces,
            private,
        })
    }
}

impl ToBencode for MetaData {
    const MAX_DEPTH: usize = 4;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), EncError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"files", &self.files)?;
            e.emit_pair(b"name", &self.name)?;
            e.emit_pair(b"piece length", &self.piece_length)?;
            e.emit_pair(b"pieces", AsString(&self.pieces))?;
            e.emit_pair(b"private", &self.private)
        })?;

        Ok(())
    }
}

pub(crate) fn read_metadata(path: &String) -> Result<MetaData, ()> {
    let res = std::fs::read(path).expect("Failed to read file");

    let mut dec = Decoder::new(&res).with_max_depth(5);

    let mut dict = match dec.next_object().unwrap() {
        Some(Object::Dict(d)) => d,
        _ => return Err(()),
    };

    let info = loop {
        match dict.next_pair().unwrap() {
            Some((b"info", val)) => break MetaData::decode_bencode_object(val).unwrap(),
            Some(_) => {
                continue;
            }
            None => return Err(()),
        };
    };

    Ok(info)
}

pub(crate) fn get_info_hash(info: &MetaData) -> Result<String, ()> {
    let bytes = info.to_bencode().unwrap();

    let mut hasher: Sha1 = Sha1::new();
    hasher.update(bytes);
    let sha_hex: Vec<u8> = hasher.finalize().to_vec();
    let sha_url = encode_binary(&sha_hex);

    Ok(sha_url.to_string())
}
