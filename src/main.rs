mod builder;
mod client;
mod parser;
mod ui;

use std::sync::Arc;

use builder::file_builder;
use client::manager::run_peer_manager_task;
use tokio::{self, sync::watch};

use parser::metadata::read_metadata;

use crate::{
    client::manager::Manager,
    ui::controller::{run_controller_task, Controller},
};


/*
    TODO FOR NEXT TIME: 
    [ ] Unfinished changes are in magnet-links branch
    [ ] Create trait from Metadata, allowing you to use varying levels of information at different stages
    [ ] Make piece_strategy and peer_handler generic - so it can be used for both acquiring files and metadata. What sort of customisation points are required?
    [ ] Move above into torrent_protocol.rs - and flesh out protocol.rs and magnet_protocol.rs

*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent_file = String::from("torrents/airfryer.torrent");
    let output_dir = String::from("downloads");
    let md = read_metadata(&torrent_file).unwrap();

    match file_builder::create(&md, &output_dir, true) {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e)
        }
    }

    let md = Arc::new(md);

    let tracker_info = client::tracker::req_tracker_info(&md).await?;
    let peers = Arc::new(tracker_info.peers);

    let (tx_progress, rx_progress) = watch::channel((0, 0));
    let (tx_in_progress_pieces, rx_in_progress_pieces) =
        watch::channel(vec![false; md.num_pieces()]);
    let (tx_downloaded_pieces, rx_downloaded_pieces) = watch::channel(vec![false; md.num_pieces()]);
    let (tx_speed, rx_speed) = watch::channel(0.0);

    let peer_manager = Manager::new(
        md.clone(),
        peers.clone(),
        &output_dir,
        tx_progress,
        tx_in_progress_pieces,
        tx_downloaded_pieces,
        tx_speed,
    )?;
    let ui_controller = Controller::new(
        md.clone(),
        peers.clone(),
        rx_progress,
        rx_in_progress_pieces,
        rx_downloaded_pieces,
        rx_speed,
    ).await;

    tokio::spawn(run_peer_manager_task(peer_manager));
    run_controller_task(ui_controller).await;

    println!("Closed");

    Ok(())
}
