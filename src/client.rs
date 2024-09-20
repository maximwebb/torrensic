use std::{error::Error, fmt};

pub mod admin_message;
pub mod manager;
mod peer_handler;
mod strategy;

#[derive(Debug)]
pub(crate) enum ProtocolError {
    TorrentInfoAcquireFailed(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ProtocolError::TorrentInfoAcquireFailed(ref msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl Error for ProtocolError {}
