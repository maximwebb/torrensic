use bendy::{
    decoding::{Error as DecError, FromBencode, Object, ResultExt},
    encoding::{AsString, Error as EncError, SingleItemEncoder, ToBencode},
};

use super::file_path_info::FilePathInfo;

// TODO: Should this be combined with fileinfo?
pub(crate) struct FileInfo {
    pub files: Vec<FilePathInfo>,
    pub name: String,
    pub piece_length: u32,
    pub pieces: Vec<u8>,
    pub private: Option<u32>,
}

impl FileInfo {
    pub(crate) fn get_from_file(path: &String) -> Result<Self, DecError> {
        let bytes = std::fs::read(path).expect("Failed to read file");
        return FileInfo::from_bencode(&bytes);
    }
}

impl FromBencode for FileInfo {
    const EXPECTED_RECURSION_DEPTH: usize = 4;

    fn decode_bencode_object(object: Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut files: Option<Vec<FilePathInfo>> = None;
        let mut name: Option<String> = None;
        let mut pieces: Option<Vec<u8>> = None;
        let mut piece_length: Option<u32> = None;
        let mut private: Option<u32> = None;

        let mut dict = object.try_into_dictionary()?;

        // Locate info dictionary in torrent file
        let mut info_dict = None;
        while let Some(pair) = dict.next_pair().unwrap() {
            match pair {
                (b"info", val) => {
                    info_dict = match val.try_into_dictionary() {
                        Ok(v) => Some(v),
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                _ => continue
            }
        };

        let mut info_dict = info_dict.expect("Unable to find info field");

        while let Some(pair) = info_dict.next_pair().unwrap() {
            match pair {
                (b"files", val) => {
                    files = Vec::decode_bencode_object(val).ok();
                }
                (b"name", val) | (b"display-name", val) => {
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
                _ => continue
            }
        }

        let files = files.ok_or_else(|| DecError::missing_field("files"))?;
        let name = name.ok_or_else(|| DecError::missing_field("name"))?;
        let pieces = pieces.ok_or_else(|| DecError::missing_field("pieces"))?;
        let piece_length = piece_length.ok_or_else(|| DecError::missing_field("piece_length"))?;

        Ok(FileInfo {
            files,
            name,
            piece_length,
            pieces,
            private,
        })
    }
}

impl ToBencode for FileInfo {
    const MAX_DEPTH: usize = 4;

    fn encode(&self, encoder: SingleItemEncoder) -> Result<(), EncError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"files", &self.files)?;
            e.emit_pair(b"name", &self.name)?;
            e.emit_pair(b"piece length", &self.piece_length)?;
            e.emit_pair(b"pieces", AsString(&self.pieces))?;
            match &self.private {
                Some(private) => e.emit_pair(b"private", private),
                None => Ok({}),
            }
        })?;

        Ok(())
    }
}

