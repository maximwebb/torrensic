use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use urlencoding::decode_binary;

use crate::parser::{
    metadata::{get_info_hash, Metadata},
    trackerinfo::PeerInfo,
};

pub(crate) async fn handshake(
    peer: &PeerInfo,
    md: &Metadata,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", peer.ip, peer.port);
    println!("Connecting to {addr}");
    let socket = TcpStream::connect(addr).await?;
    let (mut rd, mut wr) = tokio::io::split(socket);

    let pstr: Vec<u8> = b"BitTorrent protocol".to_vec();
    let pstrlen: Vec<u8> = vec![pstr.len().try_into().unwrap()];
    let reserved: Vec<u8> = vec![0; 8];
    let info_hash: Vec<u8> = get_info_hash(&md).unwrap().as_bytes().to_vec();
    let info_hash = decode_binary(&info_hash).to_vec();
    let peer_id: Vec<u8> = b"-TO0000-0123456789AB".to_vec();

    let msg: Vec<u8> = [
        pstrlen.as_slice(),
        pstr.as_slice(),
        reserved.as_slice(),
        info_hash.as_slice(),
        peer_id.as_slice(),
    ]
    .concat();

    wr.write_all(&msg).await?;

    let mut buf = vec![0; 256];
    let n = rd.read(&mut buf).await?;
    if n != 0 {
        println!("Non-empty response");
    }

    let msg = String::from_utf8_lossy(&buf.to_vec())
        .trim_matches(char::from(0))
        .to_string();

    println!("Received: {msg}");

    Ok(())
}
