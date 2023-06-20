use std::{
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use bendy::decoding::FromBencode;
use byteorder::{BigEndian, ReadBytesExt};
use rand::Rng;
use reqwest::Client;
use std::io::Error as IOError;
use tokio::{net::UdpSocket, time::timeout};
use trust_dns_resolver::Resolver;
use urlencoding::encode_binary;

use crate::parser::{
    metadata::{get_urlenc_info_hash, Metadata},
    trackerinfo::{PeerInfo, TrackerInfo},
};

pub(crate) async fn req_http_tracker_info(
    md: &Metadata,
) -> Result<TrackerInfo, Box<dyn std::error::Error>> {
    let tracker_url = &md.announce;
    let hash = get_urlenc_info_hash(&md).unwrap();
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

    let tracker_info = TrackerInfo::from_bencode(&res).unwrap();

    Ok(tracker_info)
}

pub(crate) async fn req_udp_tracker_info(
    md: &Metadata,
) -> Result<TrackerInfo, Box<dyn std::error::Error>> {
    let mut rng = rand::thread_rng();
    let trans_id: u32 = rng.gen();
    let proto_id: u64 = 0x41727101980;
    let action: u32 = 0;

    let connect_msg: Vec<u8> = [
        proto_id.to_be_bytes().to_vec(),
        action.to_be_bytes().to_vec(),
        trans_id.to_be_bytes().to_vec(),
    ]
    .concat();

    let socket = UdpSocket::bind("0.0.0.0:3000").await?;

    for tracker in &md.announce_list {
        let mut duration = 1000;
        let tracker = &tracker[0];
        // let tracker = "udp://tracker.opentrackr.org:1337/announce";
        let url = url::Url::parse(tracker).unwrap();
        let addr = match url.socket_addrs(|| None) {
            Ok(v) => v[0],
            Err(_) => {
                println!("Invalid tracker url: {tracker}.");
                continue;
            }
        };
        socket.connect(addr).await?;

        let mut buf = [0; 16];
        let _ = socket.send(&connect_msg).await?;
        let resp = socket.recv(&mut buf);
        let n = match timeout(Duration::from_millis(duration), resp).await {
            Err(_) => {
                println!(
                    "Failed to receive response from {tracker} after {duration}ms, connecting to next..."
                );
                continue;
            }
            Ok(fut) => fut?,
        };

        let mut action_recv = &buf[..4];
        let mut trans_id_recv = &buf[4..8];
        let mut conn_id_recv = &buf[8..];

        println!(
            "Connected to {tracker}, with connection ID {:?}",
            conn_id_recv
        );

        let action_recv = action_recv.read_u32::<BigEndian>()?;
        let trans_id_recv = trans_id_recv.read_u32::<BigEndian>()?;
        let conn_id_recv = conn_id_recv.read_u64::<BigEndian>()?;

        if action_recv != 0 || trans_id_recv != trans_id {
            println!("Invalid response from server");
            continue;
        }

        let action: u32 = 1;
        let info_hash = &md.info_hash;
        let peer_id = b"-TO0000-0123456789AB";
        let downloaded: u64 = 0;
        let left: u64 = 38666240;
        let uploaded: u64 = 0;
        let event: u32 = 0;
        let ip: u32 = 0;
        let key: u32 = 12345;
        let num_want: i32 = -1;
        let port: u16 = 3000;

        let announce_msg = [
            conn_id_recv.to_be_bytes().to_vec(),
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
        .concat();

        loop {
            let mut buf = [0; 1024];
            let _ = socket.send(&announce_msg).await?;
            let resp = socket.recv(&mut buf);
            let n = match timeout(Duration::from_millis(duration), resp).await {
                Err(_) => {
                    if duration >= 20000 {
                        break;
                    }

                    println!("Failed to receive announce response from tracker after {duration}ms, retrying...");
                    duration *= 2;
                    continue;
                }
                Ok(fut) => fut?,
            };

            let raw = buf.to_vec();

            let res = TrackerInfo::from_raw(raw).unwrap();
            println!("here");

            return Ok(res)

            // return Err(Box::new(IOError::new(ErrorKind::InvalidData, "Complete")));
        }
    }

    return Err(Box::new(IOError::new(
        ErrorKind::NotConnected,
        "Failed to Connect",
    )));
}
