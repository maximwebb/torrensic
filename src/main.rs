mod builder;
mod client;
mod parser;
mod setup;
mod ui;
mod utils;

use std::{net::SocketAddrV4, str::FromStr, sync::Arc};

use builder::file_builder;

use crate::setup::{
    dht_peer_acquirer::DhtPeerAcquirer, magnet_link::MagnetLink,
    magnet_torrent_info_acquirer::MagnetTorrentInfoAcquirer,
    tracker_peer_acquirer::TrackerPeerAcquirer, PeerAcquirer, PeerAcquirerEnum,
};

/*
    TODO for next time:
    - [x] Add richer parsing from Magnet links (create MagnetLink struct with optional name/tracker fields)
    - [x] Automatically determine peer acquisition method based on this
    - [ ] Add custom parser for unordered bencoded dicts
    - [ ] Update tracker peer acquirer to act as iterator (i.e. don't keep trying the same tracker over and over - and also filter out all-0 IP addresses)
*/
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log!("Started program");
    // let torrent_file = String::from("torrents/homeowners.torrent");
    // let md = read_metadata(&torrent_file).unwrap();
    // let mut peer_acquirer = TrackerPeerAcquirer::new(md.announce_list.clone(), md.info_hash.clone());

    // $ transmission-cli -w . wordle.torrent
    // let magnet_link = String::from("magnet:?xt=urn:btih:3ac100e71c570bcc6a88cc7acd89dacedf0b5558"); // stew
    // let magnet_link = String::from("magnet:?xt=urn:btih:4a6b46d36598207dcd863153b112d149e13143da"); // wordle
    let output_dir = String::from("downloads");

    let magnet_link_str = String::from("magnet:?xt=urn:btih:D1AD4F4CCCC44E6227283BD334487E777EB88EDC&dn=American.Psycho.2000.Remastered.1080p.BluRay.X264.AC3.Wi&tr=http%3A%2F%2Fp4p.arenabg.com%3A1337%2Fannounce&tr=udp%3A%2F%2F47.ip-51-68-199.eu%3A6969%2Fannounce&tr=udp%3A%2F%2F9.rarbg.me%3A2780%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2710%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2730%2Fannounce&tr=udp%3A%2F%2F9.rarbg.to%3A2920%2Fannounce&tr=udp%3A%2F%2Fopen.stealth.si%3A80%2Fannounce&tr=udp%3A%2F%2Fopentracker.i2p.rocks%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.coppersurfer.tk%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.cyberia.is%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.dler.org%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.internetwarriors.net%3A1337%2Fannounce&tr=udp%3A%2F%2Ftracker.leechers-paradise.org%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.openbittorrent.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337&tr=udp%3A%2F%2Ftracker.pirateparty.gr%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.tiny-vps.com%3A6969%2Fannounce&tr=udp%3A%2F%2Ftracker.torrent.eu.org%3A451%2Fannounce");
    let magnet_link = MagnetLink::from_str(&magnet_link_str)?;

    if let Some(name) = magnet_link.name.as_ref() {
        log!("Acquiring metadata for {name}");
    }

    let mut peer_acquirer = if magnet_link.trackers.is_empty() {
        PeerAcquirerEnum::Dht(DhtPeerAcquirer::new(magnet_link.info_hash.clone()))
    } else {
        // let mut peer_acquirer = StaticPeerAcquirer::new(vec!["127.0.0.1:51413".parse().unwrap()]);
        PeerAcquirerEnum::Tracker(TrackerPeerAcquirer::new(
            magnet_link.trackers.clone(),
            magnet_link.info_hash.clone(),
        ))
    };

    let torrent_info_acquirer = MagnetTorrentInfoAcquirer::new(magnet_link.info_hash.clone());

    let torrent_info = torrent_info_acquirer
        .get_torrent_info(&mut peer_acquirer)
        .await
        .unwrap();

    log!("Got torrent info!");

    // let peers = peer_acquirer.try_get_peers().await;
    // log!("Got peers: {peers:?}");

    // let magnet_acquirer = MagnetAcquirer::new();

    // let (md, init_peers, peers_chan) = match magnet_acquirer.acquire(magnet_link).await? {
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

    // match file_builder::create(&md, &output_dir, true) {
    //     Ok(_) => {}
    //     Err(e) => {
    //         log!("{:?}", e)
    //     }
    // }

    // let (tx_progress, rx_progress) = watch::channel((0, 0));
    // let (tx_in_progress_pieces, rx_in_progress_pieces) =
    //     watch::channel(vec![false; md.num_pieces()]);
    // let (tx_downloaded_pieces, rx_downloaded_pieces) = watch::channel(vec![false; md.num_pieces()]);
    // let (tx_speed, rx_speed) = watch::channel(0.0);

    // let peer_manager = Manager::new(
    //     md.clone(),
    //     init_peers.clone(),
    //     &output_dir,
    //     tx_progress,
    //     tx_in_progress_pieces,
    //     tx_downloaded_pieces,
    //     tx_speed,
    // )?;
    // let ui_controller = Controller::new(
    //     md.clone(),
    //     init_peers.clone(),
    //     rx_progress,
    //     rx_in_progress_pieces,
    //     rx_downloaded_pieces,
    //     rx_speed,
    // )
    // .await;

    // tokio::spawn(run_peer_manager_task(peer_manager));
    // run_controller_task(ui_controller).await;

    log!("Closed");

    Ok(())
}
