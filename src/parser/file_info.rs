use bendy::{
    decoding::{Error as DecError, FromBencode, Object, ResultExt},
    encoding::{AsString, Error as EncError, SingleItemEncoder, ToBencode},
};

pub(crate) struct FilePathInfo {
    pub length: u32,
    pub path: Vec<String>,
}

pub(crate) struct FileInfo {
    pub files: Vec<FilePathInfo>,
    pub name: String,
    pub piece_length: u32,
    pub pieces: Vec<u8>,
    pub private: Option<u32>,
}

/////////////////
// Decoding

impl FromBencode for FilePathInfo {
    const EXPECTED_RECURSION_DEPTH: usize = 4;

    fn decode_bencode_object(object: Object) -> Result<Self, DecError>
    where
        Self: Sized,
    {
        let mut length: Option<u32> = None;
        let mut path: Option<Vec<String>> = None;

        let mut dict = object.try_into_dictionary()?;

        while let Some(pair) = dict.next_pair()? {
            match pair {
                (b"length", val) => {
                    length = u32::decode_bencode_object(val)
                        .context("length")
                        .map(Some)?;
                }
                (b"path", val) => {
                    path = Vec::decode_bencode_object(val).context("path").map(Some)?;
                }
                _ => {
                    continue;
                }
            }
        }

        let length = length.ok_or_else(|| DecError::missing_field("length"))?;
        let path = path.ok_or_else(|| DecError::missing_field("path"))?;

        Ok(FilePathInfo { length, path })
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
                _ => {
                    continue;
                }
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

/////////////////
// Encoding

impl ToBencode for FilePathInfo {
    const MAX_DEPTH: usize = 4;

    fn encode(&self, encoder: bendy::encoding::SingleItemEncoder) -> Result<(), EncError> {
        encoder.emit_dict(|mut e| {
            e.emit_pair(b"length", &self.length)?;
            e.emit_pair(b"path", &self.path)
        })?;

        Ok(())
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
