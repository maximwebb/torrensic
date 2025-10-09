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
    let magnet_link = String::from("magnet:?xt=urn:btih:730FEBFEA86CCC7934854817181F040B52D4A267&dn=Kendrick+Lamar+-+GNX+%282024%29+Mp3+320kbps+%5BPMEDIA%5D+%E2%AD%90%EF%B8%8F&tr=http%3A%2F%2Fp4p.arenabg.com%3A1337%2Fannounce&tr=udp%3A%2F%2F47.ip-51-68-199.eu%3A6969%2Fannounce&tr=udp%3A%2F%2F9.rarbg.me%3A2780%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2710%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2730%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2920%2Fannounce&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce&tr=udp%3A%2F%2Fopentracker.i2p.rocks%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.coppersurfer.tk%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.cyberia.is%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.dler.org%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.internetwarriors.net%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.leechers-paradise.org%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.openbittorrent.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337&tr=udp%3A%2F%2Ftracker.pirateparty.gr%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.tiny-vps.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451%2Fannounce");
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
