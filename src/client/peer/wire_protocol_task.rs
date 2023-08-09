use std::cmp::min;
use std::mem;
use std::sync::Arc;
use std::time::Duration;

use bitvec::prelude::*;
use rand::Rng;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{oneshot, Mutex};
use tokio::time::{self, sleep, Instant};

use super::message::bitfield::Bitfield;
use super::message::have::Have;
use super::message::piece::Piece;
use super::message::{Message, PeerWireMessage};
use super::{Command, PeerState};

use crate::builder::file_builder;
use crate::client::peer_manager::BitVecMutex;
use crate::parser::{metadata::Metadata, trackerinfo::PeerInfo};

use super::connection::Connection;

pub struct WireProtocolTask {
    receiver: Receiver<Command>,
    peer_state: PeerState,
    md: Arc<Metadata>,
    addr: Arc<str>,
    output_dir: Arc<str>,
    client_pieces: BitVecMutex,
}

impl WireProtocolTask {
    pub(crate) fn new(
        receiver: Receiver<Command>,
        md: Arc<Metadata>,
        addr: &str,
        output_dir: Arc<str>,
        client_pieces: BitVecMutex,
    ) -> Self {
        WireProtocolTask {
            receiver,
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
        }
    }

    pub(crate) async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        // println!("[{}] Started", self.addr);
        let mut conn = Connection::new(&self.addr, &self.md).await?;

        let mut peer_state = PeerState {
            client_choked: true,
            client_interested: false,
            peer_choked: true,
            peer_interested: false,
        };

        let mut peer_pieces: BitVec<u8, Msb0> =
            BitVec::<u8, Msb0>::repeat(false, self.md.num_pieces());

        let mut piece_index = 0;
        let mut block_index: u32 = 0;
        {
            // Acquire client_pieces mutex to send bitfield message to peer and determine piece index.
            let mut pieces = self.client_pieces.lock().await;
            let bitfield_msg = Bitfield {
                bitfield: bitvec_to_bytes(&pieces),
            };
            let _ = conn.push(Message::from(bitfield_msg)).await?;
            piece_index = match pieces.first_zero() {
                Some(v) => v.try_into().unwrap(),
                None => 0, //TODO: shouldn't this be return?
            };
            // println!("[{}] Acquiring piece {piece_index}", self.addr);
            pieces.set(0, true);
        }

        let mut data_buf = vec![0; self.md.info.piece_length.try_into().unwrap()];

        loop {
            if piece_index == self.md.num_pieces().try_into().unwrap() {
                return Ok(());
            }

            let msg = conn.pop().await?;

            // println!("[{}], {}", self.addr, msg.print());
            match msg {
                Message::Cancel(_) => return Ok(()),
                Message::Bitfield(Bitfield { bitfield: raw }) => {
                    peer_pieces = BitVec::<_, Msb0>::from_vec(
                        raw[0..(self.md.num_pieces() + 7) / 8].to_vec(),
                    );
                    {
                        let pieces = self.client_pieces.lock().await;
                        // TODO: only submit requests for matched pieces.
                        let matched_pieces = peer_pieces.clone() & !pieces.clone();
                        if matched_pieces.any() {
                            peer_state.client_interested = true;
                            if !peer_state.client_choked {
                                conn.request_block(&self.md, piece_index, block_index)
                                    .await?;
                            } else {
                                conn.send_interested().await?;
                            }
                        }
                    }
                }
                Message::Have(Have { piece_index }) => {
                    peer_pieces.set(piece_index.try_into().unwrap(), true);
                }
                Message::Piece(Piece {
                    index,
                    begin,
                    block,
                }) => {
                    if index == piece_index && begin == (block_index * (2 << 13)) {
                        let begin_usize: usize = begin.try_into().unwrap();
                        let block_len = min(block.len(), 2 << 13);
                        data_buf.splice(begin_usize..begin_usize + block_len, block);

                        if block_index + 1 == self.md.num_blocks() {
                            // println!(
                            //     "[{}] Downloaded piece {piece_index}/{}",
                            //     self.addr,
                            //     self.md.num_pieces()
                            // );
                            let data = mem::replace(
                                &mut data_buf,
                                vec![0; self.md.info.piece_length.try_into().unwrap()],
                            );
                            file_builder::write(&self.md, &self.output_dir, piece_index, 0, &data)?;
                            {
                                let mut pieces = self.client_pieces.lock().await;
                                pieces.set(piece_index.try_into().unwrap(), true);
                                if pieces.all() {
                                    // println!(
                                    //     "Thread completed in {} secs!",
                                    //     start.elapsed().as_secs_f32()
                                    // );
                                    return Ok(());
                                } else {
                                    piece_index = pieces
                                        .first_zero()
                                        .expect("Error: client_pieces bitfield is not all-ones, yet has no zeroes.")
                                        .try_into()
                                        .unwrap();
                                    block_index = 0;
                                    pieces.set(piece_index.try_into().unwrap(), true);
                                }
                            }
                        } else {
                            block_index += 1;
                        }
                        if !peer_state.client_choked {
                            conn.request_block(&self.md, piece_index, block_index)
                                .await?;
                        } else {
                            conn.send_interested().await?;
                        }
                    } else {
                        // println!("Received wrong block");
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
                    conn.request_block(&self.md, piece_index, block_index)
                        .await?;
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

// TODO: move this to impl
pub(crate) async fn run_proto_task(mut proto_task: WireProtocolTask) {
    let _ = proto_task.run().await;
}
