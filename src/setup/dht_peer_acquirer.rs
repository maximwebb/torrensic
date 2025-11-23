use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use async_trait::async_trait;
use bendy::{decoding::FromBencode, encoding::ToBencode};
use priority_queue::PriorityQueue;
use rand::{seq::SliceRandom, thread_rng};
use tokio::sync::{mpsc, oneshot};

use crate::{
    client::ProtocolError::TorrentInfoAcquireFailed,
    setup::{magnet_link::InfoHash, PeerList},
};
use crate::{
    log, log_err,
    parser::magnet_message::{Endpoint, GetPeers, GetPeersResponse, MagnetMessage, Ping},
    setup::{dht_peer_acquirer::utils::compute_node_id, PeerAcquirer},
};

mod messages;
mod utils;

pub struct DhtPeerAcquirer {
    rx_peers: mpsc::Receiver<SocketAddrV4>,
}

#[async_trait]
impl PeerAcquirer for DhtPeerAcquirer {
    async fn try_get_peers(&mut self) -> Option<PeerList> {
        let mut buffer = Vec::new();
        self.rx_peers.recv_many(&mut buffer, 128).await;
        if buffer.is_empty() {
            None
        } else {
            Some(PeerList(buffer))
        }
    }

    async fn get_peers(&mut self) -> PeerList {
        todo!()
    }
}

impl DhtPeerAcquirer {
    pub fn new(info_hash: InfoHash) -> Self {
        let mut endpoints = [
            "86.6.8.99:45074",
            "88.227.79.197:15229",
            "102.208.186.56:56549",
            "109.189.232.148:21349",
            "109.243.69.96:6881",
            "146.70.10.41:53325",
            "154.242.158.113:62423",
            "193.118.38.73:33781",
            "195.123.225.54:61598",
            "198.16.155.97:59809",
            "213.44.109.247:61598",
            "222.153.178.36:1094",
            "2.56.190.167:41592",
            "5.194.215.132:6881",
            "35.153.138.183:6881",
            "49.37.51.27:54907",
            "66.56.80.3:39448",
            "66.51.132.43:48783",
            "65.183.152.68:51413",
            "64.58.194.69:53443",
            "62.182.98.139:6881",
            "60.225.4.206:18236",
            "49.227.208.201:36387",
            "49.49.220.77:51413",
            "49.43.107.17:35295",
            "46.232.211.234:58255",
            "46.229.247.26:23714",
            "46.11.133.67:22914",
            "45.248.77.158:39932",
            "45.180.149.225:59225",
            "45.144.115.58:40634",
            "45.144.113.143:2969",
            "45.132.225.177:41858",
            "45.92.33.163:26821",
            "45.88.190.183:26603",
            "45.83.220.219:36542",
            "41.209.3.74:6881",
            "41.75.173.34:6881",
            "41.56.189.25:17033",
            "41.13.182.220:44541",
            "38.248.90.52:32294",
            "38.162.215.40:30706",
            "37.248.151.232:6881",
            "37.140.254.117:46001",
            "37.120.219.246:41185",
            "37.111.206.235:45007",
            "37.106.33.33:35511",
            "37.60.43.238:65525",
            "37.40.91.78:35036",
            "37.36.34.85:20005",
            "37.26.70.11:6881",
            "37.19.213.209:18012",
            "36.255.112.139:32545",
            "31.183.145.7:37111",
            "31.22.88.111:6929",
            "24.235.196.128:62311",
            "24.50.205.190:54243",
            "23.234.92.93:44624",
            "2.58.72.82:6881",
            "213.152.161.56:65170",
            "213.152.161.35:16877",
            "213.110.144.47:58534",
            "209.141.57.194:61904",
            "206.45.197.197:14082",
            "203.190.14.168:51413",
            "202.172.96.41:28656",
            "202.51.199.91:6881",
            "198.44.159.34:23433",
            "198.44.138.146:51413",
            "197.232.144.99:49909",
            "197.184.68.152:6881",
            "197.155.205.174:4000",
            "197.82.161.55:17203",
            "196.188.225.166:54731",
            "194.88.96.50:3000",
            "192.0.136.104:6881",
            "191.96.36.84:51604",
            "189.203.34.20:30704",
            "188.143.105.128:56800",
            "188.129.80.244:51413",
            "188.27.152.240:19741",
            "187.190.141.169:44423",
            "186.122.91.207:51413",
            "186.121.162.196:50000",
        ];
        endpoints.shuffle(&mut thread_rng());

        let bootstrap_nodes = endpoints
            .into_iter()
            .map(|endpoint| endpoint.parse().unwrap())
            .collect();

        let (tx_peers, mut rx_peers) = mpsc::channel::<SocketAddrV4>(1024);

        let _ = tokio::spawn(async move {
            Self::acquire_peers_task(bootstrap_nodes, info_hash, tx_peers).await
        });

        DhtPeerAcquirer { rx_peers }
    }
}

impl DhtPeerAcquirer {
    async fn acquire_node_hash(
        bootstrap_nodes: &Vec<SocketAddrV4>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        log!("Acquiring own node hash");

        let id = String::from("abcdefghij0123456789");
        let ping = MagnetMessage::<Ping> {
            payload: Ping { id: id.into() },
        };
        let ping_bytes = ping.to_bencode().unwrap();

        for addr in bootstrap_nodes.iter() {
            let resp_bytes = match utils::make_req(&ping_bytes, addr).await? {
                Some(v) => v,
                None => continue,
            };

            let ip = match Endpoint::from_bencode(&resp_bytes) {
                Ok(v) => {
                    log!(
                        "Got own endpoint: {:?}:{:?} from peer: {:?}",
                        Ipv4Addr::from(v.ip),
                        v.port,
                        addr
                    );
                    v.ip
                }
                Err(e) => {
                    log_err!("{e}");
                    continue;
                }
            };

            return Ok(compute_node_id(ip));
        }
        Err(Box::new(TorrentInfoAcquireFailed(
            "Could not determine our node ID from bootstrap nodes".to_owned(),
        )))
        // Ok(compute_node_id(1380408026 as u32))
    }

    async fn acquire_peers_task(
        nodes: Vec<SocketAddrV4>,
        info_hash: InfoHash,
        tx_peers: mpsc::Sender<SocketAddrV4>,
    ) -> Result<(), ()> {
        let node_hash = Self::acquire_node_hash(&nodes).await.unwrap();

        let mut unvisited_nodes = PriorityQueue::new();
        for node in nodes {
            unvisited_nodes.push(node, 200);
        }
        let mut visited_nodes = HashSet::<SocketAddrV4>::new();
        // TODO MW: Do we still need to store these?
        let mut peers = HashSet::<SocketAddrV4>::new();
        let max_peers = 1000;
        let num_workers = 20;

        let (tx_admin_message, mut rx_admin_message) = mpsc::channel(128);

        let get_peers = MagnetMessage::<GetPeers> {
            payload: GetPeers {
                id: node_hash,
                info_hash: info_hash.clone(),
            },
        };
        let get_peers_bytes = Arc::new(get_peers.to_bencode().unwrap());

        for _ in 0..num_workers {
            let bytes = get_peers_bytes.clone();
            let tx = tx_admin_message.clone();
            tokio::spawn(async {
                match Self::acquire_peers_inner(bytes, tx).await {
                    Ok(_) => {}
                    Err(e) => {
                        log!("Got err: {}", e);
                    }
                }
            });
        }

        loop {
            tokio::select! {
                admin_message = rx_admin_message.recv() => {
                    match admin_message.expect("Error receiving message") {
                        messages::AdminMessage::NodeAddressRequest(req) => {
                            let val = unvisited_nodes.pop();
                            let addr = val.map(|(v, _)| v);
                            if let Some(v) = addr {
                                visited_nodes.insert(v);
                            }
                            let _ = req.chan.send(addr);
                        },
                        messages::AdminMessage::AddressList(req) => {
                            if !req.peers.is_empty() {
                                let endpoints = HashSet::<SocketAddrV4>::from_iter(req.peers);
                                let endpoints: HashSet<_> = endpoints.difference(&peers).cloned().collect();
                                log!(
                                    "Got {} new peer endpoints (total: {})",
                                    endpoints.len(),
                                    endpoints.len() + peers.len()
                                );
                                for endpoint in endpoints.clone() {
                                    let _ = tx_peers.send(endpoint).await;
                                }
                                peers.extend(endpoints);

                                if peers.len() >= max_peers {
                                    break;
                                }
                            }
                            if !req.nodes.is_empty() {
                                let pre_len = unvisited_nodes.len();
                                for node in req.nodes {
                                    if !visited_nodes.contains(&node) {
                                        unvisited_nodes.push(node, utils::fuzzy_xor_distance(&req.id, &info_hash));
                                    }
                                }
                                log!(
                                    "Got {} new node endpoints (total: {}, xor distance: {})",
                                    unvisited_nodes.len() - pre_len,
                                    unvisited_nodes.len(),
                                    utils::fuzzy_xor_distance(&info_hash, &req.id)
                                );
                            }
                            let _ = req.ack.send(());
                        },
                    }
                }
            }
        }

        if peers.is_empty() {
            log!("Error: failed to acquire peers from DHT, stopping...");
            return Err(());
        }

        Ok(())
    }

    async fn acquire_peers_inner(
        msg_bytes: Arc<Vec<u8>>,
        tx_admin_message: mpsc::Sender<messages::AdminMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let (tx, rx) = oneshot::channel();
            let _ = tx_admin_message
                .send(messages::AdminMessage::NodeAddressRequest(
                    messages::NodeAddressRequest { chan: tx },
                ))
                .await;

            let addr = match rx.await? {
                Some(v) => v,
                None => {
                    // log!("Got None when requesting node address, exiting");
                    break;
                }
            };

            let resp = utils::make_req(&msg_bytes, &addr).await?;

            let resp = match resp {
                Some(v) => v,
                None => continue,
            };

            let GetPeersResponse { peers, id, nodes } =
                match MagnetMessage::<GetPeersResponse>::from_bencode(&resp) {
                    Ok(v) => v.payload,
                    Err(e) => {
                        log_err!("Error parsing response: {}", e.to_string());
                        continue;
                    }
                };

            let (tx, rx) = oneshot::channel();
            let _ = tx_admin_message
                .send(messages::AdminMessage::AddressList(messages::AddressList {
                    ack: tx,
                    peers,
                    nodes,
                    id,
                }))
                .await;

            let _ = rx.await?;
        }

        Ok(())
    }
}
