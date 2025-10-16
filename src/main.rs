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

use torrent_info::{
    magnet_acquirer::MagnetAcquirer, tracker_acquirer::TrackerAcquirer, TorrentInfo,
    TorrentInfoAcquirer,
};

use crate::{
    client::manager::Manager,
    ui::controller::{run_controller_task, Controller},
};

/*
    TODO FOR NEXT TIME: Continue testing custom bencode key parser (write unit test for key extraction), maybe extract into separate utility. Test full metadata protocol.

    https://www.bittorrent.org/beps/bep_0010.html (extended)
    https://www.bittorrent.org/beps/bep_0009.html (metadata)


    [ ] ----> START WRITING acquire_metadata()
    [x] Create more versatile connection/read_task that allows arbitrary message trait
    [ ] Create PeerInfoFeed in MagnetAcquirer and return from new/getPeerInfoFeed
    [x] Update unvisited nodes to priority q
    [x] Organise magnet_acquirer
    [x] Add channels for communicating newly discovered peers out
    [ ] Write magnet metadata acquirer
    [ ] Can we simplify other bencoding code with emit_pair_with?
    [ ] Make logic for parsing metadata handshake into parsing utility (i.e. for extracting value of bencoded key/val)
    [ ] Remove extended from Message?
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log!("Started program");
    // let torrent_file = String::from("torrents/homeowners.torrent");
    let magnet_link = String::from("magnet:?xt=urn:btih:040fd66a35e4f8cde5813dd288f4fcc3f33dfb7b");
    let output_dir = String::from("downloads");

    let magnet_acquirer = MagnetAcquirer::new();

    let (md, init_peers, peers_chan) = match magnet_acquirer.acquire(magnet_link).await? {
        TorrentInfo {
            md,
            init_peers,
            peers_chan,
        } => (
            Arc::new(md),
            Arc::new(init_peers),
            peers_chan.and_then(|v| Some(Arc::new(v))),
        ),
    };

    // let info_acquirer = TrackerAcquirer {};
    // // TODO: await this in the manager function
    // let (md, init_peers, peers_chan) = match info_acquirer.acquire(torrent_file).await? {
    //     TorrentInfo {
    //         md,
    //         init_peers,
    //         peers_chan,
    //     } => (
    //         Arc::new(md),
    //         Arc::new(init_peers),
    //         peers_chan.and_then(|v| Some(Arc::new(v))),
    //     ),
    // };

    match file_builder::create(&md, &output_dir, true) {
        Ok(_) => {}
        Err(e) => {
            log!("{:?}", e)
        }
    }

    let (tx_progress, rx_progress) = watch::channel((0, 0));
    let (tx_in_progress_pieces, rx_in_progress_pieces) =
        watch::channel(vec![false; md.num_pieces()]);
    let (tx_downloaded_pieces, rx_downloaded_pieces) = watch::channel(vec![false; md.num_pieces()]);
    let (tx_speed, rx_speed) = watch::channel(0.0);

    let peer_manager = Manager::new(
        md.clone(),
        init_peers.clone(),
        &output_dir,
        tx_progress,
        tx_in_progress_pieces,
        tx_downloaded_pieces,
        tx_speed,
    )?;
    let ui_controller = Controller::new(
        md.clone(),
        init_peers.clone(),
        rx_progress,
        rx_in_progress_pieces,
        rx_downloaded_pieces,
        rx_speed,
    )
    .await;

    tokio::spawn(run_peer_manager_task(peer_manager));
    run_controller_task(ui_controller).await;

    log!("Closed");

    Ok(())
}
