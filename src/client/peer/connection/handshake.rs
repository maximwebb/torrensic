use std::error::Error;
use std::io::{Error as IOError, ErrorKind};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};

use crate::parser::{metadata::Metadata, trackerinfo::PeerInfo};

pub(crate) async fn handshake(
    md: &Metadata,
    rd: &mut ReadHalf<TcpStream>,
    wr: &mut WriteHalf<TcpStream>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let pstr: Vec<u8> = b"BitTorrent protocol".to_vec();
    let pstrlen: Vec<u8> = vec![pstr.len().try_into().unwrap()];
    let reserved: Vec<u8> = vec![0; 8];
    let info_hash = &md.info_hash;
    let peer_id: Vec<u8> = b"-TO0000-0123456789AB".to_vec();

    let msg = [
        pstrlen.as_slice(),
        pstr.as_slice(),
        reserved.as_slice(),
        info_hash.as_slice(),
        peer_id.as_slice(),
    ]
    .concat();

    let _ = wr.write_all(&msg).await?;

    let mut buf = vec![0; 1024];
    let n = rd.read(&mut buf).await?;
    if n == 0 {
        return Err(Box::new(IOError::new(
            ErrorKind::InvalidData,
            "Empty handshake",
        )));
    }

    let remaining: Vec<u8> = buf[68..n].to_vec();

    Ok(remaining)
}
