mod parser;
mod client;

use tokio;

use parser::metadata::read_metadata;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent_file: String = String::from("torrents/test.torrent");
    let md = &read_metadata(&torrent_file).unwrap();

    let tracker_info = client::tracker::req_tracker_info(md).await?;

    println!("Found {} peers.", { tracker_info.peers.len() });

    for peer in tracker_info.peers {
        client::peer_wire::handshake(&peer, md).await?;
    }


    Ok(())
}
