use std::time::Duration;

use bitvec::prelude::*;
use tokio::io::{AsyncWriteExt, ReadBuf};
use tokio::time::timeout;
use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::client::message::bitfield::Bitfield;
use crate::client::message::have::Have;
use crate::client::message::interested::Interested;
use crate::client::message::piece::Piece;
use crate::client::message::request::Request;
use crate::client::message::{Message, PeerWireMessage};
use crate::{
    client::{handshake::handshake, message::parse},
    parser::{metadata::Metadata, trackerinfo::PeerInfo},
};

pub(crate) async fn run(peer: &PeerInfo, md: &Metadata) -> Result<(), Box<dyn std::error::Error>> {
    // let ip = "79.100.5.104";
    // let port = 36367;
    // let addr = format!("{}:{}", ip, port);
    let addr = format!("{}:{}", peer.ip, peer.port);
    let addr_str = addr.clone();
    println!("Connecting to {addr_str}");
    let socket = TcpStream::connect(addr);
    let socket = match timeout(Duration::from_millis(10000), socket).await {
        Ok(v) => match v {
            Ok(v) => v,
            Err(e) => return Err(Box::new(e)),
        },
        Err(e) => return Err(Box::new(e)),
    };

    let (mut rd, mut wr) = tokio::io::split(socket);
    println!("Performing handshake with {addr_str}");
    let mut remaining = handshake(peer, md, &mut rd, &mut wr).await?;

    let mut peer_state = PeerState {
        client_choked: true,
        client_interested: false,
        peer_choked: true,
        peer_interested: false,
    };

    let mut client_pieces: BitVec<u8, Msb0> = BitVec::<u8, Msb0>::repeat(false, md.pieces_len());
    let mut peer_pieces: BitVec<u8, Msb0> = BitVec::<u8, Msb0>::repeat(false, md.pieces_len());

    let bitfield_msg = Bitfield {
        bitfield: bitvec_to_bytes(&client_pieces),
    };
    let _ = wr.write_all(&bitfield_msg.serialise()).await?;

    let mut piece_index: u32 = 0;
    let mut block_index: u32 = 0;

    loop {
        let rem = &remaining;
        let mut buf = [0; 33000];
        let n = rd.read(&mut buf[..]).await?;
        let buf = [rem.to_vec(), buf[0..n].to_vec()].concat();

        if buf.len() == 0 {
            println!("Empty message");
            return Ok(());
        }

        let buf_ = buf.clone();
        let msg = match parse(buf) {
            Ok(v) => {
                remaining = Vec::new();
                v
            }
            Err(_) => {
                println!("Partial message");
                remaining = buf_;
                continue;
            }
        };
        println!("{}", msg.print());
        match msg {
            Message::Cancel(_) => return Ok(()),
            Message::Bitfield(Bitfield { bitfield: raw }) => {
                peer_pieces = BitVec::<_, Msb0>::from_vec(raw[0..md.pieces_len() / 8].to_vec());
                let matched_pieces = peer_pieces.clone() & !client_pieces.clone();
                if matched_pieces.any() {
                    peer_state.client_interested = true;
                    if !peer_state.client_choked {
                        let request_msg = Message::from(Request {
                            index: piece_index,
                            begin: block_index * (2 << 14),
                            length: 2 << 14,
                        });
                        wr.write_all(&request_msg.serialise()).await?;
                    } else {
                        let interest_msg = Message::from(Interested {});
                        wr.write_all(&interest_msg.serialise()).await?;
                    }
                }
            }
            Message::Have(Have { piece_index }) => {
                peer_pieces.set(piece_index, true);
            }
            Message::Piece(Piece {
                index,
                begin,
                block,
            }) => {
                if index == piece_index && begin == (block_index * (2 << 14)) {
                    println!("Received block {block_index} from piece {piece_index}!");
                    if block_index + 1 == md.block_num().try_into().unwrap() {
                        block_index = 0;
                        piece_index += 1;
                    } else {
                        block_index += 1;
                    }
                    if !peer_state.client_choked {
                        let request_msg = Message::from(Request {
                            index: piece_index,
                            begin: block_index * (2 << 14),
                            length: 2 << 14,
                        });
                        wr.write_all(&request_msg.serialise()).await?;
                    } else {
                        let interest_msg = Message::from(Interested {});
                        wr.write_all(&interest_msg.serialise()).await?;
                    }
                } else {
                    println!("Received wrong block");
                }
            }
            Message::Choke(_) => peer_state.client_choked = true,
            Message::Unchoke(_) => {
                peer_state.client_choked = false;
                let interest_msg = Message::from(Interested {});
                wr.write_all(&interest_msg.serialise()).await?;
                let request_msg = Message::from(Request {
                    index: piece_index,
                    begin: 0,
                    length: 2 << 14,
                });
                wr.write_all(&request_msg.serialise()).await?;
            }
            Message::Interested(_) => peer_state.peer_interested = true,
            Message::NotInterested(_) => peer_state.peer_interested = false,
            _ => continue,
        }
    }
}

struct PeerState {
    client_choked: bool,
    client_interested: bool,
    peer_choked: bool,
    peer_interested: bool,
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
