mod builder;
mod client;
mod parser;

use builder::file_builder;
use tokio;

use parser::metadata::read_metadata;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent_file = String::from("torrents/test_folder.torrent");
    let output_dir = &String::from("downloads");
    let md = &read_metadata(&torrent_file).unwrap();

    match file_builder::create(md, output_dir, false) {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e)
        }
    }

    let tracker_info = client::tracker::req_tracker_info(md).await?;
    println!("Found {} peers.", { tracker_info.peers.len() });

    for peer in &tracker_info.peers {
        match client::peer_wire::run(&peer, md, output_dir).await {
            Ok(_) => {
                println!("Torrent download complete!");
                return Ok(())
            },
            Err(e) => println!("{:?}", e),
        }
    }

    Ok(())
}