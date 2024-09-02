mod builder;
mod client;
mod parser;
mod ui;

use std::sync::Arc;

use builder::file_builder;
use client::manager::run_peer_manager_task;
use tokio::{self, sync::watch};

use parser::bootstrap_info::read_metadata;

use crate::{
    client::manager::Manager,
    ui::controller::{run_controller_task, Controller}, parser::torrent_info::{get_torrent_info, TorrentType},
};


/*
    TODO FOR NEXT TIME: 
    [] Finish the get_torrent_info function
    [] Use this data to retrieve peer_info from tracker
    [] Also rewrite peer info so it contains list of endpoints

*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent_info = get_torrent_info(String::from("torrents/airfryer.torrent"))?;
    // let md = read_metadata(&torrent_file).unwrap();
    
    let (file_info, peer_info) = match torrent_info {
        TorrentType::File(bootstrap_info, file_info) => {
            todo!()
        }
        TorrentType::Magnet(bootstrap_info) => {
            todo!()
        }
    };

    let output_dir = String::from("downloads");
    match file_builder::create(&file_info, &output_dir, true) {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e)
        }
    }

    let file_info = Arc::new(file_info);

    let tracker_info = client::tracker::req_tracker_info(&file_info).await?;
    let peers = Arc::new(tracker_info.peers);

    let (tx_progress, rx_progress) = watch::channel((0, 0));
    let (tx_in_progress_pieces, rx_in_progress_pieces) =
        watch::channel(vec![false; file_info.num_pieces()]);
    let (tx_downloaded_pieces, rx_downloaded_pieces) = watch::channel(vec![false; file_info.num_pieces()]);
    let (tx_speed, rx_speed) = watch::channel(0.0);

    let peer_manager = Manager::new(
        file_info.clone(),
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
