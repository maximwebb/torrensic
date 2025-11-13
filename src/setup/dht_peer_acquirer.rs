use std::{collections::HashSet, net::{Ipv4Addr, SocketAddrV4}, sync::Arc};

use bendy::{decoding::FromBencode, encoding::ToBencode};
use priority_queue::PriorityQueue;
use tokio::sync::{mpsc, oneshot};

use crate::{log, log_err, parser::magnet_message::{Endpoint, GetPeers, GetPeersResponse, MagnetMessage, Ping}, setup::{PeerAcquirer, dht_peer_acquirer::utils::compute_node_id}};
use crate::{client::ProtocolError::TorrentInfoAcquireFailed};

mod utils;
mod messages;

pub struct DhtPeerAcquirer {
    rx_peers: mpsc::Receiver<SocketAddrV4>,
}

impl PeerAcquirer for DhtPeerAcquirer {
    async fn try_get_peers(&mut self) -> Option<Vec<SocketAddrV4>> {
        let mut buffer = Vec::new();
        self.rx_peers.recv_many(&mut buffer, 128).await;
        if buffer.is_empty() { None } else { Some(buffer) }
    }

    async fn get_peers(&mut self) -> Vec<SocketAddrV4> {
        todo!()
    }
}

impl DhtPeerAcquirer {
    fn new(info_hash: Vec<u8>) -> Self {
        let endpoints = vec!["127.0.0.1:51413"];
        
        let bootstrap_nodes = 
                endpoints
                    .into_iter()
                    .map(|endpoint| endpoint.parse().unwrap())
                    .collect();

        let (tx_peers, mut rx_peers) = mpsc::channel::<SocketAddrV4>(1024);
        
        let _ = tokio::spawn(async move {
            Self::acquire_peers_task(bootstrap_nodes, info_hash, tx_peers).await
        });

        DhtPeerAcquirer { rx_peers}
    }
}

impl DhtPeerAcquirer{
    async fn acquire_node_hash(bootstrap_nodes: &Vec<SocketAddrV4>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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
        info_hash: Vec<u8>,
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

        // TODO REMOVE: start off with a known good IP
        let init_ip1: SocketAddrV4 = "127.0.0.1:51413".parse().unwrap();
        // let init_ip2: SocketAddrV4 = "201.92.174.163:15375".parse().unwrap();
        // let init_ip2: SocketAddrV4 = "94.2.212.131:10982".parse().unwrap();
        let _ = tx_peers.send(init_ip1).await;

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
            .send(messages::AdminMessage::NodeAddressRequest(messages::NodeAddressRequest {
                chan: tx,
            }))
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