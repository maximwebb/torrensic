use std::net::SocketAddrV4;

use crate::{log_err, parser::metadata::Metadata};

pub mod dht_peer_acquirer;
pub mod static_peer_acquirer;
pub mod tracker_peer_acquirer;

pub mod magnet_torrent_info_acquirer;

// TODO MW: Figure out API for requesting new peers
pub trait PeerAcquirer {
    async fn try_get_peers(&mut self) -> Option<Vec<SocketAddrV4>>;

    async fn get_peers(&mut self) -> Vec<SocketAddrV4>;
}

pub fn parse_info_hash(link: &str) -> Option<Vec<u8>> {
    if !link.starts_with("magnet:?") {
        return None;
    }

    let pairs = link[8..].split('&');

    for pair in pairs {
        let mut splitter = pair.splitn(2, '=');
        if splitter.next().unwrap() != "xt" {
            continue;
        }

        let v = splitter.next().unwrap();
        if !v.starts_with("urn:btih:") {
            log_err!("Error: got unexpected value for xt in magnet link: {}", v);
            continue;
        }

        let info_hash = v[9..].to_string();

        if info_hash.len() != 40 {
            log_err!(
                "Error: got unexpected info hash length in magnet link: {}",
                info_hash
            );
            continue;
        }

        let info_hash = hex::decode(info_hash).expect("Error: Invalid info hash");
        return Some(info_hash);
    }

    return None;
}
