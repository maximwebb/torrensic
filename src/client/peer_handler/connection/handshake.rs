use std::error::Error;
use std::io::{Error as IOError, ErrorKind};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};

use crate::client::handshake_message::get_handshake_bytes;

pub(crate) async fn handshake(
    info_hash: &Vec<u8>,
    rd: &mut ReadHalf<TcpStream>,
    wr: &mut WriteHalf<TcpStream>,
    _req_metadata: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let msg = get_handshake_bytes(info_hash);
    let _ = wr.write_all(&msg).await?;

    let mut buf = vec![0; 1024];
    let n = rd.read(&mut buf).await?;
    if n == 0 {
        return Err(Box::new(IOError::new(
            ErrorKind::InvalidData,
            "Empty handshake",
        )));
    }

    let remaining: Vec<u8> = if n > 68 {
        buf[68..n].to_vec()
    } else {
        Vec::new()
    };

    Ok(remaining)
}
