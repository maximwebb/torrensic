mod client;
mod parser;

use tokio;

use parser::metadata::read_metadata;

use client::message::{
    have::Have, interested::Interested, keep_alive::KeepAlive, not_interested::NotInterested,
    request::Request, PeerWireMessage,
};

use crate::client::message::{parse, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent_file: String = String::from("torrents/test.torrent");
    let md = &read_metadata(&torrent_file).unwrap();

    let tracker_info = client::tracker::req_tracker_info(md).await?;

    println!("Found {} peers.", { tracker_info.peers.len() });

    // for peer in tracker_info.peers {
    //     client::peer_wire::handshake(&peer, md).await?;
    // }

    let have = Have { piece_index: 420 };
    let interested = Interested {};
    let keep_alive = KeepAlive {};
    let not_interested = NotInterested {};
    let request = Request {
        index: 42,
        begin: 33,
        length: 99,
    };

    let have_res = parse(have.serialise()).unwrap();
    let interested_res = parse(interested.serialise()).unwrap();
    let keep_alive_res = parse(keep_alive.serialise()).unwrap();
    let not_interested_res = parse(not_interested.serialise()).unwrap();
    let request_res = parse(request.serialise()).unwrap();

    let res = [
        have_res,
        interested_res,
        keep_alive_res,
        not_interested_res,
        request_res,
    ];
    for msg in res {
        println!("{}", msg.print())
    }

    // for msg in res {
    //     match msg {
    //         Message::KA(v) => println!("{}", v.print()),
    //         Message::I(v) => println!("{}", v.print()),
    //         Message::NI(v) => println!("{}", v.print()),
    //         Message::H(v) => println!("{}", v.print()),
    //         Message::R(v) => println!("{}", v.print()),
    //     };
    // }

    Ok(())
}
