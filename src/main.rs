mod builder;
mod client;
mod parser;
mod torrent_info;
mod ui;
mod utils;

use std::sync::Arc;

use builder::file_builder;
use client::manager::run_peer_manager_task;
use tokio::{self, sync::watch};

use torrent_info::{magnet_acquirer::MagnetAcquirer, tracker_acquirer::TrackerAcquirer, TorrentInfo, TorrentInfoAcquirer};

use crate::{
    client::manager::Manager,
    ui::controller::{run_controller_task, Controller},
};

/*
    TODO FOR NEXT TIME:
    [x] Parse ping response into ip/port - does it send on connect or on ping?
    [ ] Implement node hash calculation
    [ ] Write other magnet messages
    [x] Try pinging some other nodes
    [ ] Start implementing protocol
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let torrent_file = String::from("torrents/airfryer.torrent");
    let output_dir = String::from("downloads");

    let magnet_acquirer = MagnetAcquirer::new();
    magnet_acquirer.acquire(torrent_file.clone()).await?;


    let info_acquirer = TrackerAcquirer {};
    let (md, peers) = match info_acquirer.acquire(torrent_file).await? {
        TorrentInfo { md, peers } => (Arc::new(md), Arc::new(peers)),
    };

    match file_builder::create(&md, &output_dir, true) {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e)
        }
    }

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
    )
    .await;

    tokio::spawn(run_peer_manager_task(peer_manager));
    run_controller_task(ui_controller).await;

    println!("Closed");

    Ok(())
}
