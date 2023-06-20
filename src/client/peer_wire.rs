use std::time::Duration;

use tokio::time::timeout;
use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::client::message::{Message, PeerWireMessage};
use crate::{
    client::{handshake::handshake, message::parse},
    parser::{metadata::Metadata, trackerinfo::PeerInfo},
};

pub(crate) async fn run(peer: &PeerInfo, md: &Metadata) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", peer.ip, peer.port);
    println!("Connecting to {addr}");
    let socket = TcpStream::connect(addr);
    let socket = match timeout(Duration::from_millis(1000), socket).await {
        Ok(v) => match v {
            Ok(v) => v,
            Err(e) => return Err(Box::new(e)),
        },
        Err(e) => return Err(Box::new(e)),
    };

    let (mut rd, mut wr) = tokio::io::split(socket);

    handshake(peer, md, &mut rd, &mut wr).await?;

    loop {
        let mut buf = vec![0; 256];
        let n = rd.read(&mut buf).await?;
        let buf = buf[0..n].to_vec();

        let msg = match parse(buf) {
            Ok(v) => v,
            Err(_) => {
                println!("Invalid message");
                return Ok(());
            }
        };
        println!("{}", msg.print());
        match msg {
            Message::Cancel(_) => return Ok(()),
            Message::Choke(_) => return Ok(()),
            _ => continue,
        }
    }
}
