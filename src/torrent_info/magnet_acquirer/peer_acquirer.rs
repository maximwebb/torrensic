use std::sync::Arc;

use crate::torrent_info::magnet_acquirer::GetPeersResponse;

use bendy::decoding::FromBencode;
use tokio::sync::{mpsc, oneshot};

use crate::parser::magnet_message::MagnetMessage;

use super::admin_message::{AddressList, AdminMessage, NodeAddressRequest};

pub(crate) async fn run(
    msg_bytes: Arc<Vec<u8>>,
    tx_admin_message: mpsc::Sender<AdminMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let (tx, rx) = oneshot::channel();
        let _ = tx_admin_message.send(AdminMessage::NodeAddressRequest(NodeAddressRequest {
            chan: tx,
        })).await;

        let addr = match rx.await? {
            Some(v) => v,
            None => {
                println!("Got None when requesting node address, exiting");
                break;
            },
        };

        let resp = super::make_req(&msg_bytes, &addr).await?;

        let resp = match resp {
            Some(v) => v,
            None => continue,
        };

        let GetPeersResponse{ peers, id, nodes } = match MagnetMessage::<GetPeersResponse>::from_bencode(&resp) {
            Ok(v) => v.payload,
            Err(e) => {
                println!("Error parsing response: {}", e.to_string());
                continue;
            }
        };

        let (tx, rx) = oneshot::channel();
        let _ = tx_admin_message.send(AdminMessage::AddressList(AddressList {
            ack: tx, peers, nodes, id
        })).await;

        let _ = rx.await?;
    }

    Ok(())
}
