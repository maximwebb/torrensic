mod client;
mod parser;
mod utils;

use tokio;

use parser::metadata::read_metadata;

use client::message::{
    choke::Choke, have::Have, interested::Interested, keep_alive::KeepAlive,
    not_interested::NotInterested, parse, request::Request, Message, PeerWireMessage,
};

use crate::client::message::{bitfield::Bitfield, cancel::Cancel, piece::Piece, unchoke::Unchoke};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent_file: String = String::from("torrents/airfryer.torrent");
    let md = &read_metadata(&torrent_file).unwrap();

    let tracker_info = client::tracker::req_tracker_info(md).await?;

    println!("Found {} peers.", { tracker_info.peers.len() });

    for peer in tracker_info.peers {
        let _ = client::peer_wire::run(&peer, md).await;
    }

    Ok(())
}

fn test_messages() {
    let keep_alive = Message::from(KeepAlive {});
    let choke = Message::from(Choke {});
    let unchoke = Message::from(Unchoke {});
    let interested = Message::from(Interested {});
    let not_interested = Message::from(NotInterested {});
    let have = Message::from(Have { piece_index: 420 });
    let bitfield = Message::from(Bitfield {
        bitfield: vec![1, 2, 3, 4, 5, 6],
    });
    let request = Message::from(Request {
        index: 42,
        begin: 33,
        length: 99,
    });
    let piece = Message::from(Piece {
        index: 69,
        begin: 420,
        block: vec![3, 1, 4, 1, 5, 9],
    });
    let cancel = Message::from(Cancel {
        index: 43,
        begin: 34,
        length: 100,
    });

    let res = [
        keep_alive,
        choke,
        unchoke,
        interested,
        not_interested,
        have,
        bitfield,
        request,
        piece,
        cancel,
    ]
    .map(|msg| parse(msg.serialise()).unwrap());
    for msg in res {
        println!("{}", msg.print())
    }
}
