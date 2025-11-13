use std::time::Duration;

use bendy::decoding::FromBencode;
use tokio::{io, sync::mpsc, time::{sleep, timeout}};

use crate::{log, log_err, log_warn, parser::file_info::FileInfo};

use super::PeerAcquirer;

use messages::{MetadataHandshake, MetadataRequest, MetadataResponse, serialise_magnet_msg};
use socket_handler::MagnetSocketHandler;

mod messages;
mod socket_handler;

pub struct MagnetTorrentInfoAcquirer {
    info_hash: Vec<u8>
}

impl MagnetTorrentInfoAcquirer {
    pub fn new(info_hash: Vec<u8>) -> Self {
        Self {
            info_hash
        }
    }

    pub async fn get_torrent_info(&self, peer_acquirer: &mut impl PeerAcquirer) -> io::Result<FileInfo>
    {
        loop {
            let peers = peer_acquirer
                .try_get_peers()
                .await
                .expect("Received invalid peer message");

            for addr in peers {
                let (tx_cancel, mut rx_cancel) = mpsc::channel::<()>(1);

                let Ok(mut sock_handler) = MagnetSocketHandler::try_new(&addr.to_string()).await else {
                    log_warn!("Failed to connect to peer");
                    continue;
                };

                log!("[{}] Connected", addr);

                let Ok(handshake_resp) = sock_handler.handshake(&self.info_hash).await else {
                    log_warn!("Failed to handshake with peer");
                    continue;
                };

                log!(
                    "[{}] Completed handshake - got: {}",
                    addr,
                    String::from_utf8_lossy(&handshake_resp)
                );

                let Ok(ext_handshake_resp_bytes) = sock_handler.read().await else {
                    log_warn!("Failed to read from peer");
                    continue;
                };

                let Ok(ext_handshake) =
                    MetadataHandshake::from_bencode(&ext_handshake_resp_bytes) else {
                        log_warn!("Failed to parse extension handshake response: {}", String::from_utf8_lossy(&ext_handshake_resp_bytes));
                        continue;
                    };

                log!("[{}] Completed extension handshake: {:?}", addr, ext_handshake);

                let size = ext_handshake.size;
                let msg_code = ext_handshake.msg_code;
                let mut piece_index: u32 = 0;
                let mut num_recv_bytes: u32 = 0;
                let mut recv_bytes = Vec::new();

                loop {
                    log!("[{}] Requesting metadata piece {}", addr, piece_index);
                    let req = MetadataRequest {
                        piece_index,
                        msg_code,
                    };

                    let req_bytes = serialise_magnet_msg(&req, msg_code);
                    sock_handler.write(&req_bytes).await?;

                    let msg_bytes = match timeout(Duration::from_millis(5000), sock_handler.read()).await {
                        Ok(Ok(v)) => {
                            v
                        }
                        Ok(Err(e)) => {
                            log_err!("Got error: {e:?}");
                            continue;
                        }
                        Err(_) => {
                            log_warn!("Timed out when reading from socket, retrying...");
                            continue;
                        }
                    };

                    let Ok(mut response) = MetadataResponse::deserialise(&msg_bytes) else {
                        log_warn!("[{}] Got unknown message (len {}): {}", addr, msg_bytes.len(), String::from_utf8_lossy(&msg_bytes));
                        continue;
                    };

                    if piece_index == response.header.piece {
                        piece_index += 1;
                        num_recv_bytes += response.payload.len() as u32;
                        recv_bytes.append(&mut response.payload);
                    }
                    else {
                        log_warn!("Got wrong piece - expected {}, got {}", piece_index, response.header.piece);
                        continue;
                    }


                    if num_recv_bytes >= size {
                        log!("[{}] Finished acquiring metadata", addr);
                        break;
                    }
                }

                match FileInfo::from_bencode(&recv_bytes) {
                    Ok(v) => {
                        return Ok(v);
                    },
                    Err(e) =>  {
                        log!("Failed to parse: {:?}", e);
                    }
                }
            }
    }
}
}