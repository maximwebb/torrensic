use bendy::{
    decoding::{Error as DecError, FromBencode, Object, ResultExt},
    encoding::{AsString, Error as EncError, SingleItemEncoder, ToBencode},
};

use super::fileinfo::FileInfo;

// TODO: Should this be combined with fileinfo?
pub(crate) struct Info {
    pub files: Vec<FileInfo>,
    pub name: String,
    pub piece_length: u32,
    pub pieces: Vec<u8>,
    pub private: u32,
}

impl FromBencode for Info {
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

        Ok(Info {
            files,
            name,
            piece_length,
            pieces,
            private,
        })
    }
}

impl ToBencode for Info {
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
