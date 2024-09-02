use bendy::{
    decoding::{Error as DecError, FromBencode, Object, ResultExt},
    encoding::{Error as EncError, ToBencode},
};

pub(crate) struct FilePathInfo {
    pub length: u32,
    pub path: Vec<String>,
}

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
