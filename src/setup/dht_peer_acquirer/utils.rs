use std::{net::SocketAddrV4, time::Duration};

use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::{net::UdpSocket, time::timeout};

use crate::{log, log_err};

pub(crate) async fn make_req(
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

    let len = match timeout(Duration::from_millis(2000), resp).await {
        Err(_) => {
            // log!("Timeout when attempting to perform UDP tracker handshake with {addr} after 2000ms");
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

pub(crate) fn compute_node_id(ip: u32) -> Vec<u8> {
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

pub(crate) fn count_ones(v: &Vec<bool>) -> u32 {
    return v.iter().filter(|&&x| x).count().try_into().unwrap();
}

pub(crate) fn fuzzy_xor_distance(x: &Vec<u8>, y: &Vec<u8>) -> u32 {
    if x.len() != y.len() {
        log!("Error: mismatched sizes (x: {}, y: {})", x.len(), y.len());
    }

    let mut res = 0;

    for v in x.iter().zip(y.iter()).map(|(a, b)| a ^ b) {
        if v == 0 {
            res += 8
        } else {
            res += v.leading_zeros();
            break;
        }
    }
    return x.len() as u32 * 8 - res;
}
