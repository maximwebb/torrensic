mod builder;
mod client;
mod parser;
mod ui;

use std::sync::Arc;

use bitvec::{prelude::Msb0, vec::BitVec};
use builder::file_builder;
use client::manager::run_peer_manager_task;
use tokio::{self, sync::watch};

use parser::metadata::read_metadata;

use crate::{
    client::manager::Manager,
    ui::controller::{run_controller_task, Controller},
};

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

    let tracker_info = client::tracker::req_tracker_info(&md).await?;
    // println!("Found {} peers.", { tracker_info.peers.len() });

    let (tx_progress, rx_progress) = watch::channel((0, 0));
    let (tx_pieces, rx_pieces) = watch::channel(BitVec::<u8, Msb0>::repeat(false, md.num_pieces()));
    let (tx_speed, rx_speed) = watch::channel(0.0);
    let md = Arc::new(md);

    let peer_manager = Manager::new(
        md.clone(),
        tracker_info.peers,
        &output_dir,
        tx_progress,
        tx_pieces,
        tx_speed,
    )?;
    let ui_controller = Controller::new(md.clone(), rx_progress, rx_pieces, rx_speed);

    tokio::spawn(run_peer_manager_task(peer_manager));
    run_controller_task(ui_controller).await;

    println!("Closed");

    Ok(())
}
