mod admin_message;
mod magnet_socket_handler;
mod peer_acquirer;

use core::num;
use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

use bendy::{decoding::FromBencode, encoding::ToBencode};
use priority_queue::PriorityQueue;
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::{io, net::UdpSocket, select, sync::mpsc, time::timeout};

use crate::{
    client::{
        handshake_message::get_handshake_bytes,
        peer_handler::connection::{Deserialisable, Serialisable},
        ProtocolError::TorrentInfoAcquireFailed,
    },
    log, log_err,
    parser::{
        magnet_message::{
            Endpoint, GetPeers, GetPeersResponse, MagnetMessage, MetadataHandshake,
            MetadataMessage, MetadataRequest, Ping,
        },
        metadata::Metadata,
    },
    torrent_info::magnet_acquirer::magnet_socket_handler::MagnetSocketHandler,
    utils,
};

use super::{TorrentInfo, TorrentInfoAcquirer};

async fn make_req(
    msg_bytes: &Vec<u8>,
    addr: &SocketAddrV4,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    match socket.connect(addr).await {
        Ok(_) => {}
        Err(e) => {
            log_err!("Got error during connection: {e}");
            return Ok(None);
        }
    };

    let _len = match socket.send(&msg_bytes).await {
        Ok(_len) => _len,
        Err(e) => {
            log_err!("Got error while sending: {e}");
            return Ok(None);
        }
    };

    let mut buf = [0; 4096];
    let resp = socket.recv(&mut buf);

    let len = match timeout(Duration::from_millis(500), resp).await {
        Err(_) => {
            // log!("Timeout when attempting to perform UDP tracker handshake with {addr} after 500ms");
            return Ok(None);
        }
        Ok(fut) => match fut {
            Ok(len) => len,
            Err(e) => {
                log_err!("Got error while receiving: {e}");
                return Ok(None);
            }
        },
    };
    Ok(Some(buf[..len].to_vec()))
}

fn compute_node_id(ip: u32) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(42);
    let rand: u8 = rng.gen();
    let r: u32 = (rand & 0x7).into();

    let bytes = (ip & 0x03_0f_3f_ff) | (r << 29);
    let bytes = bytes.to_be_bytes();

    let hash = crc32c::crc32c(&bytes).to_be_bytes();

    let mut node_id = hash[..3].to_vec();
    node_id.extend_from_slice(&rng.gen::<[u8; 16]>());
    node_id.push(rand);

    node_id
}

fn parse_info_hash(link: &str) -> Option<Vec<u8>> {
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
            log!("Error: got unexpected value for xt in magnet link: {}", v);
            continue;
        }

        let info_hash = v[9..].to_string();

        if info_hash.len() != 40 {
            log!(
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

#[derive(Clone)]
pub(crate) struct MagnetAcquirer {
    bootstrap_nodes: Arc<Vec<SocketAddrV4>>,
}

impl MagnetAcquirer {
    pub(crate) fn new() -> Self {
        let endpoints = vec!["127.0.0.1:51413"];

        return MagnetAcquirer {
            bootstrap_nodes: Arc::new(
                endpoints
                    .into_iter()
                    .map(|endpoint| endpoint.parse().unwrap())
                    .collect(),
            ),
        };
    }

    async fn acquire_node_hash(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let id = String::from("abcdefghij0123456789");
        let ping = MagnetMessage::<Ping> {
            payload: Ping { id: id.into() },
        };
        let ping_bytes = ping.to_bencode().unwrap();

        for addr in self.bootstrap_nodes.iter() {
            let resp_bytes = match make_req(&ping_bytes, addr).await? {
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
        // Err(Box::new(TorrentInfoAcquireFailed(
        //     "Could not determine our node ID from bootstrap nodes".to_owned(),
        // )))
        Ok(compute_node_id(1380408026 as u32))
    }

    async fn acquire_peers(
        &self,
        id: Vec<u8>,
        info_hash: Vec<u8>,
        tx_peers: mpsc::Sender<Vec<SocketAddrV4>>,
    ) -> Result<HashSet<SocketAddrV4>, ()> {
        let nodes: Vec<SocketAddrV4> = self.bootstrap_nodes.to_vec();
        let mut unvisited_nodes = PriorityQueue::new();
        for node in nodes {
            unvisited_nodes.push(node, 200);
        }
        let mut visited_nodes = HashSet::<SocketAddrV4>::new();
        let mut peers = HashSet::<SocketAddrV4>::new();
        let max_peers = 1000;
        let num_workers = 20;

        let (tx_admin_message, mut rx_admin_message) = mpsc::channel(128);

        let get_peers = MagnetMessage::<GetPeers> {
            payload: GetPeers {
                id,
                info_hash: info_hash.clone(),
            },
        };
        let get_peers_bytes = Arc::new(get_peers.to_bencode().unwrap());

        // TODO REMOVE: start off with a known good IP
        let init_ip1: SocketAddrV4 = "127.0.0.1:51413".parse().unwrap();
        // let init_ip2: SocketAddrV4 = "201.92.174.163:15375".parse().unwrap();
        // let init_ip2: SocketAddrV4 = "94.2.212.131:10982".parse().unwrap();
        let _ = tx_peers.send(vec![init_ip1]).await;

        for _ in 0..num_workers {
            let bytes = get_peers_bytes.clone();
            let tx = tx_admin_message.clone();
            tokio::spawn(async {
                match peer_acquirer::run(bytes, tx).await {
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
                        admin_message::AdminMessage::NodeAddressRequest(req) => {
                            let val = unvisited_nodes.pop();
                            let addr = val.map(|(v, _)| v);
                            if let Some(v) = addr {
                                visited_nodes.insert(v);
                            }
                            let _ = req.chan.send(addr);
                        },
                        admin_message::AdminMessage::AddressList(req) => {
                            if !req.peers.is_empty() {
                                let endpoints = HashSet::<SocketAddrV4>::from_iter(req.peers);
                                let endpoints: HashSet<_> = endpoints.difference(&peers).cloned().collect();
                                log!(
                                    "Got {} new peer endpoints (total: {})",
                                    endpoints.len(),
                                    endpoints.len() + peers.len()
                                );
                                let _ = tx_peers.send(endpoints.clone().into_iter().collect()).await;
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

        Ok(peers)
    }

    async fn acquire_metadata(
        &self,
        info_hash: Vec<u8>,
        rx_peers: &mut mpsc::Receiver<Vec<SocketAddrV4>>,
    ) -> io::Result<Metadata> {
        loop {
            let peers = rx_peers
                .recv()
                .await
                .expect("Received invalid peer message");

            for addr in peers {
                let (tx_cancel, mut rx_cancel) = mpsc::channel::<()>(1);

                let mut sock_handler = MagnetSocketHandler::try_new(&addr.to_string()).await?;

                log!("[{}] Connected", addr);

                let handshake_resp = sock_handler.handshake(&info_hash).await?;

                log!(
                    "[{}] Completed handshake - got: {}",
                    addr,
                    String::from_utf8_lossy(&handshake_resp)
                );

                let ext_handshake_resp_bytes = sock_handler.read().await?;
                log!("{}", String::from_utf8_lossy(&ext_handshake_resp_bytes));
                let ext_handshake =
                    MetadataHandshake::from_bencode(&ext_handshake_resp_bytes).unwrap();

                log!("[{}] Got extension handshake: {:?}", addr, ext_handshake);

                let size = ext_handshake.size;
                let msg_code = ext_handshake.msg_code;
                let mut piece_index: u32 = 0;
                let mut received_bytes: u32 = 0;

                loop {
                    log!("[{}] Requesting piece {}", addr, piece_index);

                    let req_bytes =
                        MetadataMessage::MetadataRequest(MetadataRequest { piece_index, msg_code })
                            .serialise();
                    sock_handler.write(&req_bytes).await?;

                    let msg_bytes = tokio::select! {
                        v = sock_handler.read() => {
                            match v {
                                Ok(v) => v,
                                Err(e) => {
                                    log!("Error: {}", e);
                                    break;
                                },
                            }
                        }
                        _ = rx_cancel.recv() => {
                            break;
                        }
                    };

                    let Ok((Some(MetadataMessage::MetadataResponse(response)), _)) = MetadataMessage::deserialise(&msg_bytes) else {
                        log!("[{}] Got unknown message: {}", addr, String::from_utf8_lossy(&msg_bytes));
                        continue;
                    };                

                    log!("[{}] Got response {:?}", addr, response);
                    piece_index += 1;
                    received_bytes += response.total_size;

                    if received_bytes >= size {
                        log!("[{}] Finished acquiring metadata", addr);
                        break;
                    }
                }

            }
        }

        todo!()
    }
}

// Responsive IP: 41.133.89.199:6881
impl TorrentInfoAcquirer for MagnetAcquirer {
    async fn acquire(&self, torrent: String) -> Result<TorrentInfo, Box<dyn std::error::Error>> {
        let id = self.acquire_node_hash().await?;
        let info_hash = parse_info_hash(&torrent).ok_or(Box::new(TorrentInfoAcquireFailed(
            "Failed to parse magnet link".to_owned(),
        )))?;

        let (tx_peers, mut rx_peers) = mpsc::channel::<Vec<SocketAddrV4>>(8);

        let acquirer = self.clone();
        let info_hash_copy = info_hash.clone();
        tokio::spawn(async move { acquirer.acquire_peers(id, info_hash_copy, tx_peers).await });

        // log!(
        //     "Got peers!: {}",
        //     peers
        //         .iter()
        //         .map(|x| x.to_string())
        //         .collect::<Vec<String>>()
        //         .join(", ")
        // );

        let md = self.acquire_metadata(info_hash, &mut rx_peers).await?;

        Ok(TorrentInfo {
            md,
            init_peers: Vec::new(),
            peers_chan: Some(rx_peers),
        })
    }
}
