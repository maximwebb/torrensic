mod peer_acquirer;
mod admin_message;

use core::num;
use std::{
    collections::HashSet, net::{Ipv4Addr, SocketAddrV4}, sync::Arc, time::Duration
};

use bendy::{decoding::FromBencode, encoding::ToBencode};
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::{net::UdpSocket, sync::mpsc, time::timeout};

use crate::{
    client::ProtocolError::TorrentInfoAcquireFailed,
    parser::magnet_message::{Endpoint, GetPeers, GetPeersResponse, MagnetMessage, Ping},
};

use super::TorrentInfoAcquirer;

async fn make_req(
    msg_bytes: &Vec<u8>,
    addr: &SocketAddrV4,
) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    match socket.connect(addr).await {
        Ok(_) => {}
        Err(e) => {
            println!("Got error during connection: {e}");
            return Ok(None);
        }
    };

    let _len = match socket.send(&msg_bytes).await {
        Ok(_len) => _len,
        Err(e) => {
            println!("Got error while sending: {e}");
            return Ok(None);
        }
    };

    let mut buf = [0; 4096];
    let resp = socket.recv(&mut buf);

    let len = match timeout(Duration::from_millis(500), resp).await {
        Err(_) => {
            // println!("Timeout when attempting to perform UDP tracker handshake with {addr} after 500ms");
            return Ok(None);
        }
        Ok(fut) => match fut {
            Ok(len) => len,
            Err(e) => {
                println!("Got error while receiving: {e}");
                return Ok(None);
            }
        },
    };
    Ok(Some(buf[..len].to_vec()))
}

pub(crate) struct MagnetAcquirer {
    bootstrap_nodes: Vec<SocketAddrV4>,
}

impl MagnetAcquirer {
    pub(crate) fn new() -> Self {
        let endpoints = vec![
            "37.14.4.86:33167",
            "45.155.42.24:28402",
            "54.234.214.165:6881",
            "61.147.235.2:37461",
            "80.1.160.54:3000",
            "103.1.212.150:23756",
            "102.212.228.48:26567",
            "102.176.101.69:20756",
            "102.70.12.105:8973",
            "102.69.226.80:6881",
            "101.189.203.117:54134",
            "101.115.153.211:51207",
            "94.140.11.119:45394",
            "94.59.173.236:59848",
            "93.65.240.70:65136",
            "92.119.18.19:8999",
            "91.196.221.245:45580",
            "91.167.237.44:6881",
            "90.203.191.149:50034",
            "89.213.77.128:10648",
            "89.169.8.178:16091",
            "89.147.101.131:26089",
            "89.134.5.17:21027",
            "89.40.206.132:44545",
            "88.250.196.37:64890",
            "88.17.63.33:22123",
            "86.153.198.245:6881",
            "86.104.22.100:23343",
            "86.94.235.186:16881",
            "86.48.10.240:28174",
            "86.5.131.87:62496",
            "85.228.140.153:51413",
            "85.139.196.218:51413",
            "85.12.30.131:45406",
            "84.239.41.6:18680",
            "84.39.151.118:52338",
            "84.17.51.94:41758",
            "83.233.4.167:5846",
            "83.109.18.89:64062",
            "82.114.64.28:63949",
            "80.6.38.219:6881",
            "79.118.191.35:55567",
            "79.117.225.135:54146",
            "78.190.220.4:10502",
            "78.131.65.239:64602",
            "73.247.177.92:41880",
            "73.65.4.123:44623",
            "69.160.121.150:6881",
            "66.54.123.0:6881",
            "64.64.116.140:6881",
            "58.167.166.57:6881",
            "58.38.71.27:52000",
            "50.64.48.102:6881",
            "47.149.85.59:63059",
            "47.72.203.4:16349",
            "46.219.215.107:24949",
            "45.226.98.146:56645",
            "45.134.20.85:30765",
            "45.126.169.83:10319",
            "45.86.211.19:58403",
            "45.86.210.29:6881",
            "43.231.28.146:41417",
            "41.208.191.23:57793",
            "37.120.152.250:6877",
            "37.106.66.234:10049",
            "37.19.197.185:53310",
            "31.209.236.33:38957",
            "31.46.188.140:53464",
            "31.44.143.225:65525",
            "31.30.172.155:29601",
            "31.11.71.223:34474",
            "23.81.64.214:31868",
            "14.192.208.152:40339",
            "14.153.84.166:6881",
            "5.193.21.163:52763",
            "5.91.63.129:11786",
            "5.13.131.29:21017",
            "2.238.135.175:49443",
            "2.218.57.86:6882",
            "1.58.65.106:8999",
            "223.233.83.253:38709",
            "222.83.138.95:52000",
            "222.65.86.165:52000",
            "220.233.193.15:39748",
            "219.74.71.214:16881",
            "218.207.164.218:55245",
            "217.132.198.68:22923",
            "217.74.147.38:28532",
            "217.8.12.10:56356",
            "216.131.74.131:60680",
            "212.234.220.34:54624",
            "212.233.158.166:10795",
            "212.85.161.215:1856",
            "209.134.35.4:6881",
            "207.232.29.61:38542",
            "204.112.209.203:22116",
            "203.40.29.248:51587",
            "201.247.15.219:50177",
            "198.54.134.146:56068",
            "197.184.176.129:35782",
            "197.91.217.133:14782",
            "197.87.220.134:6881",
            "197.3.200.76:53207",
            "194.35.123.121:51712",
            "194.35.122.87:23177",
            "193.138.44.250:10405",
            "193.36.224.59:14271",
            "192.145.119.72:51413",
            "192.0.201.228:28431",
            "191.181.217.152:23198",
            "191.181.217.152:16293",
            "190.237.0.50:6882",
            "190.234.178.173:5446",
            "190.213.130.8:57266",
            "190.213.34.232:62464",
            "190.121.65.106:35296",
            "189.190.224.132:54643",
            "189.154.200.119:51413",
            "189.153.151.202:18031",
            "189.60.10.43:27790",
            "188.126.89.80:32845",
            "188.36.210.233:6881",
            "188.30.87.8:6881",
            "186.23.132.160:51413",
            "186.13.38.150:51413",
            "185.254.75.40:11329",
            "185.245.87.188:7229",
            "185.218.127.163:57591",
            "185.216.146.248:51413",
            "185.158.242.63:39072",
            "185.93.1.193:16017",
            "185.83.69.75:47479",
            "185.21.217.5:32812",
            "184.75.214.163:47118",
            "184.22.103.3:12496",
            "183.182.104.81:6881",
            "182.177.33.144:6881",
            "181.214.218.66:34254",
            "181.214.167.196:46243",
            "181.214.153.167:53264",
            "181.188.37.175:18434",
            "181.116.200.205:50263",
            "181.91.84.58:19583",
            "181.68.19.50:44776",
            "180.190.218.154:16799",
            "180.190.183.64:58596",
            "180.140.124.3:45944",
            "180.117.170.146:27505",
            "180.74.216.28:19383",
            "179.61.197.5:40277",
            "179.60.72.246:37627",
            "179.6.168.31:27457",
            "178.220.255.230:25541",
            "178.162.196.10:30814",
            "177.33.139.10:54354",
            "176.203.167.97:38429",
            "176.116.244.55:57675",
            "173.32.224.250:47265",
            "171.22.106.144:16826",
            "171.6.241.38:23570",
            "162.156.201.81:50000",
            "161.29.122.151:52123",
            "161.8.69.207:52548",
            "159.146.34.214:54932",
            "157.97.121.123:49608",
        ];

        return MagnetAcquirer {
            bootstrap_nodes: endpoints
                .into_iter()
                .map(|endpoint| endpoint.parse().unwrap())
                .collect(),
        };
    }

    // TODO: separate out into MagnetLink struct
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
                println!("Error: got unexpected value for xt in magnet link: {}", v);
                continue;
            }

            let info_hash = v[9..].to_string();

            if info_hash.len() != 40 {
                println!(
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

    async fn acquire_node_hash(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let id = String::from("abcdefghij0123456789");
        let ping = MagnetMessage::<Ping> {
            payload: Ping { id: id.into() },
        };
        let ping_bytes = ping.to_bencode().unwrap();

        for addr in &self.bootstrap_nodes {
            let resp_bytes = match make_req(&ping_bytes, addr).await? {
                Some(v) => v,
                None => continue,
            };

            let ip = match Endpoint::from_bencode(&resp_bytes) {
                Ok(v) => {
                    println!(
                        "Got own endpoint: {:?}:{:?} from peer: {:?}",
                        Ipv4Addr::from(v.ip),
                        v.port,
                        addr
                    );
                    v.ip
                }
                Err(e) => {
                    println!("Got error: {e}");
                    continue;
                }
            };

            return Ok(Self::compute_node_id(ip));
        }
        Err(Box::new(TorrentInfoAcquireFailed(
            "Could not determine our node ID from bootstrap nodes".to_owned(),
        )))
    }

    async fn acquire_peers(
        &self,
        id: Vec<u8>,
        info_hash: Vec<u8>,
    ) -> Result<HashSet<SocketAddrV4>, Box<dyn std::error::Error>> {
        // TODO: Make this a priority queue
        let mut unvisited_nodes = self.bootstrap_nodes.clone();
        let mut visited_nodes = HashSet::<SocketAddrV4>::new();
        let mut peers = HashSet::<SocketAddrV4>::new();
        let max_peers = 100;
        let num_workers = 5;

        let (tx_admin_message, mut rx_admin_message) = mpsc::channel(128);
        
        let get_peers = MagnetMessage::<GetPeers> {
            payload: GetPeers {
                id,
                info_hash: info_hash.clone(),
            },
        };
        let get_peers_bytes = Arc::new(get_peers.to_bencode().unwrap());

        for _ in 0..num_workers {
            let bytes = get_peers_bytes.clone();
            let tx = tx_admin_message.clone();
            tokio::spawn(async { match peer_acquirer::run(bytes, tx).await {
                Ok(_) => {},
                Err(e) => {
                    println!("Got err: {}", e);
                },
            } });
        }

        loop {
            tokio::select! {
                admin_message = rx_admin_message.recv() => {
                    match admin_message.expect("Error receiving message") {
                        admin_message::AdminMessage::NodeAddressRequest(req) => {
                            let addr = unvisited_nodes.pop();
                            if let Some(v) = addr {
                                visited_nodes.insert(v);
                            }
                            let _ = req.chan.send(addr);
                        },
                        admin_message::AdminMessage::AddressList(req) => {
                            if !req.peers.is_empty() {
                                let endpoints = HashSet::<SocketAddrV4>::from_iter(req.peers);
                                let endpoints: HashSet<_> = endpoints.difference(&peers).cloned().collect();
                                println!(
                                    "Got {} new peer endpoints (total: {})",
                                    endpoints.len(),
                                    endpoints.len() + peers.len()
                                );
                                peers.extend(endpoints);
                        
                                if peers.len() >= max_peers {
                                    break;
                                }
                            }
                            if !req.nodes.is_empty() {
                                let pre_len = unvisited_nodes.len();
                                for node in req.nodes {
                                    if !visited_nodes.contains(&node) {
                                        unvisited_nodes.push(node);
                                    }
                                }
                                println!(
                                    "Got {} new node endpoints (total: {}, xor distance: {})",
                                    unvisited_nodes.len() - pre_len,
                                    unvisited_nodes.len(),
                                    Self::compute_xor_distance(&info_hash, &req.id)
                                );
                            }
                            let _ = req.ack.send(());
                        },
                    }
                }
            }
        }

        if peers.is_empty() {
            return Err(Box::new(TorrentInfoAcquireFailed(
                "Failed to acquire peers from DHT".to_owned(),
            )));
        }

        Ok(peers)
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

    fn compute_xor_distance(x: &Vec<u8>, y: &Vec<u8>) -> f32 {
        if x.len() != y.len() {
            println!("Error: mismatched sizes (x: {}, y: {})", x.len(), y.len());
        }

        let mut res = 0;

        for v in x.iter().zip(y.iter()).map(|(a, b)| a ^ b) {
            res += if v == 0 { 8 } else { v.leading_zeros() }
        }
        return 100.0 - res as f32 / 1.6;
    }
}

impl TorrentInfoAcquirer for MagnetAcquirer {
    async fn acquire(
        &self,
        torrent: String,
    ) -> Result<super::TorrentInfo, Box<dyn std::error::Error>> {
        let id = self.acquire_node_hash().await?;
        let info_hash = match Self::parse_info_hash(&torrent) {
            Some(v) => v,
            None => {
                return Err(Box::new(TorrentInfoAcquireFailed(
                    "Failed to parse magnet link".to_owned(),
                )))
            }
        };

        let peers = self.acquire_peers(id, info_hash).await?;

        println!(
            "Got peers!: {}",
            peers
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        );

        todo!()
    }
}
