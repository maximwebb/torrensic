use std::time::Duration;

use tokio::time::timeout;
use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::client::message::bitfield::Bitfield;
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

    let mut peer_state = PeerState {
        client_choked: true,
        client_interested: false,
        peer_choked: true,
        peer_interested: false,
    };

    //TODO: rename Bitfield to avoid name clash.
    let mut peer_pieces = Bitfield{bits: Vec::new(), bitfield: todo!() }

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
            Message::Choke(_) => peer_state.client_choked = true,
            Message::Unchoke(_) => peer_state.client_choked = false,
            Message::Interested(_) => peer_state.peer_interested = true,
            Message::NotInterested(_) => peer_state.peer_interested = false,
            _ => continue,
        }
    }
}

struct PeerState {
    client_choked: bool,
    client_interested: bool,
    peer_choked: bool,
    peer_interested: bool,
}
