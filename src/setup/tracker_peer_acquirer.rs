use std::{collections::HashSet, io::ErrorKind, net::{Ipv4Addr, SocketAddrV4}, str::FromStr, time::Duration};

use async_trait::async_trait;
use bendy::decoding::FromBencode;
use byteorder::{BigEndian, ReadBytesExt};
use reqwest::Client;
use tokio::{net::UdpSocket, time::timeout};
use urlencoding::encode_binary;

use crate::{
    log, log_err, log_warn,
    parser::{
        metadata::get_urlenc_info_hash,
        tracker_info::TrackerInfo,
    },
    setup::{magnet_link::InfoHash, PeerAcquirer, PeerList},
};

pub struct TrackerPeerAcquirer {
    announce_url_list: Vec<String>,
    info_hash: InfoHash,
}

#[async_trait]
impl PeerAcquirer for TrackerPeerAcquirer {
    async fn try_get_peers(&mut self) -> Option<PeerList> {
        let mut to_remove = HashSet::new();
        let mut res = None;
        for tracker in self.announce_url_list.iter() {
            log!("Requesting peers from {tracker}");
            let req = if tracker.starts_with("http") {
                self.req_http_tracker_info(tracker).await
            } else {
                self.req_udp_tracker_info(tracker).await
            };

            match req {
                Ok(tracker_info) => {
                    let endpoints : Vec<SocketAddrV4> = tracker_info
                        .peers
                        .iter()
                        .filter_map(|v| {
                            let ip = Ipv4Addr::from_str(&v.ip).ok().filter(|ip| !ip.is_unspecified())?;
                            Some(SocketAddrV4::new(ip, v.port))
                        })
                        .collect();
                    if !endpoints.is_empty() {
                        res = Some(PeerList(endpoints));
                        break;
                    }
                }
                Err(_) => {
                    to_remove.insert(tracker.clone());
                    log_warn!("Removing tracker {tracker}");
                }
            }
        }

        self.announce_url_list.retain(|v| !to_remove.contains(v));

        if res.is_none() {
            log_err!("Failed to retrieve tracker info");
        }

        return res;
    }

    async fn get_peers(&mut self) -> PeerList {
        todo!()
    }
}

impl TrackerPeerAcquirer {
    pub fn new(announce_url_list: Vec<String>, info_hash: InfoHash) -> Self {
        Self {
            announce_url_list,
            info_hash,
        }
    }

    async fn req_http_tracker_info(
        &self,
        tracker_url: &String,
    ) -> Result<TrackerInfo, Box<dyn std::error::Error>> {
        let hash = get_urlenc_info_hash(&self.info_hash).unwrap();
        let peer_id = encode_binary(b"-TO0000-0123456789AB");
        let port = String::from("3000");
        let url = format!("{tracker_url}?info_hash={hash}&peer_id={peer_id}");

        let client = Client::new();

        let res = client
            .get(url)
            .query(&[("port", &port)])
            .send()
            .await?
            .bytes()
            .await?;

        let tracker_info = TrackerInfo::from_bencode(&res).map_err(|_e| {
            std::io::Error::new(ErrorKind::InvalidInput, "Failed to parse tracker response")
        })?;

        Ok(tracker_info)
    }

    async fn req_udp_tracker_info(
        &self,
        tracker_url: &String,
    ) -> Result<TrackerInfo, Box<dyn std::error::Error>> {
        let url = url::Url::parse(tracker_url).unwrap();
        let addr = match url.socket_addrs(|| None) {
            Ok(v) => v[0],
            Err(_) => {
                return Err(Box::new(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "Invalid tracker url: {tracker_url}",
                )));
            }
        };

        let mut timeout_duration = 1000;

        let socket = UdpSocket::bind("0.0.0.0:3000").await?;
        socket.connect(addr).await?;

        // let mut rng = rand::thread_rng();
        // let trans_id: u32 = rng.gen();
        let trans_id = 618364956;
        let connect_msg = Self::connect_msg(trans_id);
        let _ = socket.send(&connect_msg).await?;

        let mut buf = [0; 16];
        let resp = socket.recv(&mut buf);

        match timeout(Duration::from_millis(timeout_duration), resp).await {
            Err(_) => {
                return Err(Box::new(std::io::Error::new(
                    ErrorKind::TimedOut,
                    "Timeout when attempting to perform UDP tracker handshake with {tracker_url} after {duration}ms",
                )));
            }
            Ok(fut) => fut?,
        };

        let mut action_recv = &buf[..4];
        let mut trans_id_recv = &buf[4..8];
        let mut conn_id_recv = &buf[8..];

        let action_recv = action_recv.read_u32::<BigEndian>()?;
        let trans_id_recv = trans_id_recv.read_u32::<BigEndian>()?;
        let conn_id_recv = conn_id_recv.read_u64::<BigEndian>()?;

        if action_recv != 0 || trans_id_recv != trans_id {
            return Err(Box::new(std::io::Error::new(
                ErrorKind::InvalidData,
                "Invalid response from server",
            )));
        }

        let announce_msg = self.announce_msg(conn_id_recv, trans_id, None);

        loop {
            let _ = socket.send(&announce_msg).await?;

            let mut buf = [0; 1024];
            let resp = socket.recv(&mut buf);
            match timeout(Duration::from_millis(timeout_duration), resp).await {
                Err(_) => {
                    if timeout_duration >= 20000 {
                        break;
                    }

                    log!("Failed to receive announce response from tracker after {timeout_duration}ms, retrying...");
                    timeout_duration *= 2;
                    continue;
                }
                Ok(fut) => fut?,
            };

            let res = TrackerInfo::from_raw(buf.to_vec()).unwrap();
            return Ok(res);
        }

        return Err(Box::new(std::io::Error::new(
            ErrorKind::NotConnected,
            "Failed to Connect",
        )));
    }

    fn connect_msg(trans_id: u32) -> Vec<u8> {
        let proto_id: u64 = 0x41727101980; // magic bytes
        let action: u32 = 0;

        [
            proto_id.to_be_bytes().to_vec(),
            action.to_be_bytes().to_vec(),
            trans_id.to_be_bytes().to_vec(),
        ]
        .concat()
    }

    fn announce_msg(&self, conn_id: u64, trans_id: u32, peer_id: Option<Vec<u8>>) -> Vec<u8> {
        let action: u32 = 1;
        let info_hash = &self.info_hash.to_vec();
        let peer_id = match peer_id {
            None => b"-TO0000-0123456789AB".to_vec(),
            Some(v) => v,
        };
        let downloaded: u64 = 0;

        // TODO MW: How to do this without metadata?
        // let _num_pieces: u64 = md.info.pieces.len().try_into().unwrap();
        // let left: u64 = (md.info.piece_length as u64 * _num_pieces).into();
        let left: u64 = 0;
        let uploaded: u64 = 0;
        let event: u32 = 0;
        let ip: u32 = 0;
        let key: u32 = 12345;
        let num_want: i32 = -1;
        let port: u16 = 3000;

        [
            conn_id.to_be_bytes().to_vec(),
            action.to_be_bytes().to_vec(),
            trans_id.to_be_bytes().to_vec(),
            info_hash.to_vec(),
            peer_id.to_vec(),
            downloaded.to_be_bytes().to_vec(),
            left.to_be_bytes().to_vec(),
            uploaded.to_be_bytes().to_vec(),
            event.to_be_bytes().to_vec(),
            ip.to_be_bytes().to_vec(),
            key.to_be_bytes().to_vec(),
            num_want.to_be_bytes().to_vec(),
            port.to_be_bytes().to_vec(),
        ]
        .concat()
    }
}
