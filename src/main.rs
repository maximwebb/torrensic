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
    TODO FOR NEXT TIME: We seem to receive all the bytes for our metadata now, and can receive multiple messages from a peer. 
    We now need to finally parse the Metadata object - investigate why this is not working

    https://www.bittorrent.org/beps/bep_0010.html (extended)
    https://www.bittorrent.org/beps/bep_0009.html (metadata)


    [ ] ----> START WRITING acquire_metadata()
    [ ] Fix MetadataResponse deserialisation - use decoder.next_object(), determine # bytes consumed, and parse remainder of slice as binary data
    [ ] Create PeerInfoFeed in MagnetAcquirer and return from new/getPeerInfoFeed
    [ ] Can we simplify other bencoding code with emit_pair_with?
    [ ] Make logic for parsing metadata handshake into parsing utility (i.e. for extracting value of bencoded key/val)
    [ ] Remove extended from Message?
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log!("Started program");
    // let torrent_file = String::from("torrents/homeowners.torrent");
    // $ transmission-cli -w . wordle.torrent
    // let magnet_link = String::from("magnet:?xt=urn:btih:3ac100e71c570bcc6a88cc7acd89dacedf0b5558"); // stew
    let magnet_link = String::from("magnet:?xt=urn:btih:4a6b46d36598207dcd863153b112d149e13143da"); // wordle
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
