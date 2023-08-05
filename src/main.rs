mod builder;
mod client;
mod parser;
mod ui;

use std::future;

use builder::file_builder;
use client::peer_manager::run_peer_manager_task;
use tokio::{self, join, sync::{mpsc, watch}};

use parser::metadata::read_metadata;

use crate::{
    client::peer_manager::PeerManager,
    ui::controller::{run_controller_task, Controller},
};

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
    // println!("Found {} peers.", { tracker_info.peers.len() });

    let (tx_progress, rx_progress) = watch::channel((0, 0));

    let peer_manager = PeerManager::new(md, tracker_info.peers, &output_dir, tx_progress)?;
    let ui_controller = Controller::new(rx_progress);

    tokio::spawn(run_peer_manager_task(peer_manager));
    run_controller_task(ui_controller).await;

    println!("Closed");

    Ok(())
}
