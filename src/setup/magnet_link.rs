use std::{io::ErrorKind, ops::Deref, str::FromStr};

#[derive(Clone, Debug)]
pub struct InfoHash {
    hash: [u8; 20],
}

impl InfoHash {
    pub fn try_new_from_str(s: &str) -> Result<Self, std::io::Error> {
        if s.len() != 40 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!("Info hash must be 20 bytes: got {s} ({} bytes)", s.len()/2),
            ));
        }

        let hash = hex::decode(s).unwrap().try_into().unwrap();

        Ok(Self { hash })
    }

    pub fn try_new_from_bytes(bytes: &[u8]) -> Result<Self, std::io::Error> {
        if bytes.len() != 20 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!("Info hash must be 20 bytes: got {} bytes", bytes.len()),
            ));
        }

        let hash = bytes.try_into().unwrap();

        Ok(Self { hash })
    }
}

impl Deref for InfoHash {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}

pub struct MagnetLink {
    pub info_hash: InfoHash,
    pub name: Option<String>,
    pub trackers: Vec<String>,
}

impl FromStr for MagnetLink {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const PREFIX: &str = "magnet:?";
        const URN_PREFIX: &str = "urn:btih:";

        if !s.starts_with(PREFIX) {
            return Err(Self::Err::new(
                ErrorKind::InvalidInput,
                format!("Magnet must start with {PREFIX}"),
            ));
        }

        let mut info_hash = None;
        let mut name = None;
        let mut trackers = Vec::new();

        for (k, v) in s[PREFIX.len()..]
            .split('&')
            .map(|kv| kv.split_once('=').unwrap())
        {
            if k == "xt" {
                if !v.starts_with(URN_PREFIX) {
                    return Err(Self::Err::new(
                        ErrorKind::InvalidInput,
                        format!("Invalid format for xt field: {v}"),
                    ));
                }

                info_hash = Some(InfoHash::try_new_from_str(&v[URN_PREFIX.len()..])?)
            } else if k == "dn" {
                name = Some(v.to_string());
            } else if k == "tr" {
                let url = urlencoding::decode(v)
                    .map_err(|e| Self::Err::new(ErrorKind::InvalidInput, e))?
                    .to_string();
                trackers.push(url);
            }
        }

        let info_hash = info_hash.ok_or(Self::Err::new(
            ErrorKind::InvalidInput,
            "Failed to locate xt field",
        ))?;

        Ok(Self {
            info_hash,
            name,
            trackers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn parse_info_hash_correctly() {
        let info_hash_str = "D1AD4F4CCCC44E6227283BD334487E777EB88EDC";
        let info_hash = InfoHash::try_new_from_str(info_hash_str).ok();
        assert!(info_hash.is_some());
    }
}