mod builder;
mod client;
mod parser;

use builder::file_builder;
use tokio;

use parser::metadata::read_metadata;

use crate::client::peer_manager::PeerManager;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent_file = String::from("torrents/test_folder.torrent");
    let output_dir = String::from("downloads");
    let md = read_metadata(&torrent_file).unwrap();

    match file_builder::create(&md, &output_dir, true) {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e)
        }
    }

    let tracker_info = client::tracker::req_tracker_info(&md).await?;
    println!("Found {} peers.", { tracker_info.peers.len() });

    let peer_manager = PeerManager::new(md, tracker_info.peers, &output_dir);
    let _ = peer_manager.run().await;

    Ok(())
}