mod connection;
mod message;

use std::cmp::min;
use std::io::{Error as IOError, ErrorKind};
use std::mem;
use std::sync::Arc;

use bitvec::prelude::*;

use tokio::sync::mpsc::{self};
use tokio::sync::oneshot;

use message::bitfield::Bitfield;
use message::have::Have;
use message::piece::Piece;
use message::Message;

use crate::builder::file_builder;
use crate::client::manager::BitVecMutex;
use crate::parser::metadata::Metadata;

use connection::Connection;

use super::admin_message::{
    AdminMessage, PeerBitfield, PeerDisconnect, PieceDownload, PieceIndexRequest,
};

pub struct PeerHandler {
    peer_state: PeerState,
    md: Arc<Metadata>,
    addr: Arc<str>,
    output_dir: Arc<str>,
    client_pieces: BitVecMutex,
    tx_admin_message: mpsc::Sender<AdminMessage>,
}

impl PeerHandler {
    pub(crate) fn init(
        md: Arc<Metadata>,
        addr: &str,
        output_dir: Arc<str>,
        client_pieces: BitVecMutex,
        tx_admin_message: mpsc::Sender<AdminMessage>,
    ) {
        let handler = PeerHandler {
            peer_state: PeerState {
                client_choked: true,
                client_interested: false,
                peer_choked: true,
                peer_interested: false,
            },
            md,
            addr: addr.into(),
            output_dir,
            client_pieces,
            tx_admin_message,
        };

        tokio::spawn(PeerHandler::start(handler));
    }

    pub(crate) async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (tx_cancel, mut rx_cancel) = mpsc::channel::<()>(1);

        let mut conn = Connection::new(&self.addr, &self.md, tx_cancel).await?;

        let mut peer_state = PeerState {
            client_choked: true,
            client_interested: false,
            peer_choked: true,
            peer_interested: false,
        };

        let mut block_index: u32 = 0;
        let mut piece_index = None;

        {
            // Acquire client_pieces mutex to send bitfield message to peer and determine piece index.
            // TODO: get this from the manager
            let mut pieces = self.client_pieces.lock().await;
            let bitfield_msg = Bitfield {
                bitfield: bitvec_to_bytes(&pieces),
            };
            let _ = conn.push(Message::from(bitfield_msg)).await?;
            pieces.set(0, true);
        }

        let mut data_buf = vec![0; self.md.info.piece_length.try_into().unwrap()];

        loop {
            let msg = tokio::select! {
                v = conn.pop() => {
                    match v {
                        Ok(v) => Ok(v),
                        Err(_) => Err(()),
                    }
                }
                _ = rx_cancel.recv() => {
                    Err(())
                }
            };

            let msg = match msg {
                Ok(v) => v,
                Err(_) => {
                    let _ = self
                        .tx_admin_message
                        .send(AdminMessage::PeerDisconnect(PeerDisconnect {
                            addr: self.addr.clone(),
                        }))
                        .await;
                    return Err(Box::new(IOError::new(
                        ErrorKind::ConnectionReset,
                        "Connection reset by peer",
                    )));
                }
            };

            match msg {
                Message::Cancel(_) => return Ok(()),

                Message::Bitfield(Bitfield { bitfield: raw }) => {
                    let mut peer_pieces: Vec<bool> = raw
                        .iter()
                        .flat_map(|&byte| (0..8).rev().map(move |bit| (byte >> bit) & 1 == 1))
                        .collect();

                    peer_pieces.truncate(self.md.num_pieces());

                    self.send_bitfield_update(peer_pieces).await;

                    // Request piece index from peer manager
                    piece_index = self.get_piece_index().await;
                    self.peer_state.client_interested = piece_index.is_some();

                    if let Some(index) = piece_index {
                        if !peer_state.client_choked {
                            conn.request_block(&self.md, index, block_index).await?;
                        } else {
                            conn.send_interested().await?;
                        }
                    }
                }
                Message::Have(Have { piece_index: index }) => {
                    self.send_have_update(index).await;

                    if piece_index.is_none() {
                        piece_index = self.get_piece_index().await;
                    }
                }
                Message::Piece(Piece {
                    index,
                    begin,
                    block,
                }) => {
                    // TODO: properly handle case where piece_index is None
                    if index == piece_index.unwrap_or(u32::MAX)
                        && begin == (block_index * (2 << 13))
                    {
                        let begin_usize: usize = begin.try_into().unwrap();
                        let block_len = min(block.len(), 2 << 13);
                        data_buf.splice(begin_usize..begin_usize + block_len, block);

                        if block_index + 1 == self.md.num_blocks() {
                            let data = mem::replace(
                                &mut data_buf,
                                vec![0; self.md.info.piece_length.try_into().unwrap()],
                            );
                            file_builder::write(&self.md, &self.output_dir, index, 0, &data)?;
                            let _ = self
                                .tx_admin_message
                                .send(AdminMessage::PieceDownload(PieceDownload { index }))
                                .await;

                            // Request piece from peer manager - if no valid ones, we are no longer interested in peer.
                            piece_index = self.get_piece_index().await;
                            self.peer_state.client_interested = piece_index.is_some();

                            block_index = 0;
                        } else {
                            block_index += 1;
                        }

                        // Request next block
                        if !peer_state.client_choked && peer_state.peer_interested {
                            if let Some(index) = piece_index {
                                conn.request_block(&self.md, index, block_index).await?;
                            }
                        } else {
                            conn.send_interested().await?;
                        }
                    } else {
                        // Received wrong block
                    }
                }
                Message::Choke(_) => {
                    peer_state.client_choked = true;
                    if !peer_state.client_interested {
                        conn.send_interested().await?;
                        peer_state.client_interested = true;
                    }
                }
                Message::Unchoke(_) => {
                    peer_state.client_choked = false;
                    if !peer_state.client_interested {
                        conn.send_interested().await?;
                        peer_state.client_interested = true;
                    }
                    if let Some(index) = piece_index {
                        conn.request_block(&self.md, index, block_index).await?;
                    }
                }
                Message::Interested(_) => {
                    peer_state.peer_interested = true;
                    peer_state.peer_choked = false;
                }
                Message::NotInterested(_) => peer_state.peer_interested = false,
                _ => continue,
            }
        }
    }

    async fn send_bitfield_update(&self, bitfield: Vec<bool>) {
        let (tx, rx) = oneshot::channel();

        let res = self
            .tx_admin_message
            .send(AdminMessage::PeerBitfield(PeerBitfield {
                ack: tx,
                addr: self.addr.clone(),
                peer_bitfield: bitfield,
            }))
            .await;

        // Wait for manager response before returning
        rx.await.unwrap();
    }

    async fn send_have_update(&self, index: u32) {
        let mut new_pieces = vec![false; self.md.num_pieces()];
        new_pieces[TryInto::<usize>::try_into(index).unwrap()] = true;

        self.send_bitfield_update(new_pieces).await;
    }

    async fn get_piece_index(&self) -> Option<u32> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .tx_admin_message
            .send(AdminMessage::PieceIndexRequest(PieceIndexRequest {
                chan: tx,
                addr: self.addr.clone(),
            }))
            .await;

        match rx.await {
            Ok(v) => return v,
            Err(_) => {
                println!("Client received error");
                return None;
            }
        }
    }

    async fn start(mut proto_task: PeerHandler) {
        let _ = proto_task.run().await;
    }
}

fn bitvec_to_bytes(bits: &BitVec<u8, Msb0>) -> Vec<u8> {
    let mut bv = bits.to_owned();
    let pad_amt = (8 - (bv.len() % 8)) % 8;
    for _ in 0..pad_amt {
        bv.push(false);
    }

    bv.force_align();
    let res: Vec<u8> = bv.into_vec();
    res
}

pub(crate) struct PeerState {
    client_choked: bool,
    client_interested: bool,
    peer_choked: bool,
    peer_interested: bool,
}
