use std::error::Error;
use std::io::ErrorKind;
use std::time::Duration;

use bitvec::prelude::*;
use tokio::io::{AsyncWriteExt, ReadBuf, WriteHalf};
use tokio::time::timeout;
use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::builder::file_builder;
use crate::client::message::bitfield::Bitfield;
use crate::client::message::have::Have;
use crate::client::message::interested::Interested;
use crate::client::message::piece::Piece;
use crate::client::message::request::{self, Request};
use crate::client::message::{Message, PeerWireMessage};
use crate::{
    client::{handshake::handshake, message::parse},
    parser::{metadata::Metadata, trackerinfo::PeerInfo},
};

pub(crate) async fn run(
    peer: &PeerInfo,
    md: &Metadata,
    output_dir: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = "212.21.12.9:41891";
    // let addr = format!("{}:{}", peer.ip, peer.port);
    let addr_str = addr.clone();
    println!("Connecting to {addr_str}");
    let socket = TcpStream::connect(addr);
    let socket = match timeout(Duration::from_millis(5000), socket).await {
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

    let mut client_pieces: BitVec<u8, Msb0> = BitVec::<u8, Msb0>::repeat(false, md.num_pieces());
    let mut peer_pieces: BitVec<u8, Msb0> = BitVec::<u8, Msb0>::repeat(false, md.num_pieces());

    let bitfield_msg = Bitfield {
        bitfield: bitvec_to_bytes(&client_pieces),
    };
    let _ = wr.write_all(&bitfield_msg.serialise()).await?;

    let mut piece_index: u32 = 0;
    let mut block_index: u32 = 0;
    loop {
        if piece_index == md.num_pieces().try_into().unwrap() {
            return Ok(());
        }

        let rem = &remaining;
        let mut buf = [0; 17000];
        let read_fut = rd.read(&mut buf[..]);
        let n = match timeout(Duration::from_millis(500), read_fut).await {
            Ok(v) => v.unwrap(),
            Err(_) => 0,
        };

        let buf = [rem.to_vec(), buf[0..n].to_vec()].concat();

        if buf.len() == 0 {
            println!("Received empty message");
            continue;
        }

        let buf_ = buf.clone();
        let msg = match parse(buf) {
            Ok((msg, rem)) => {
                remaining = rem;
                msg
            }
            Err(_) => {
                remaining = buf_;
                continue;
            }
        };
        println!("{}", msg.print());
        match msg {
            Message::Cancel(_) => return Ok(()),
            Message::Bitfield(Bitfield { bitfield: raw }) => {
                peer_pieces = BitVec::<_, Msb0>::from_vec(raw[0..md.num_pieces() / 8].to_vec());
                let matched_pieces = peer_pieces.clone() & !client_pieces.clone();
                if matched_pieces.any() {
                    peer_state.client_interested = true;
                    if !peer_state.client_choked {
                        request_block(md, piece_index, block_index, &mut wr).await?;
                    } else {
                        send_interested(&mut wr).await?;
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
                if index == piece_index && begin == (block_index * (2 << 13)) {
                    println!("Received block {block_index} from piece {piece_index}");
                    file_builder::write(md, output_dir, piece_index, begin, block)?;
                    if block_index + 1 == md.num_blocks() {
                        client_pieces.set(piece_index.try_into().unwrap(), true);
                        block_index = 0;
                        piece_index += 1;
                    } else {
                        block_index += 1;
                    }
                    if !peer_state.client_choked {
                        request_block(md, piece_index, block_index, &mut wr).await?;
                    } else {
                        send_interested(&mut wr).await?;
                    }
                } else {
                    println!("Received wrong block");
                }
            }
            Message::Choke(_) => {
                peer_state.client_choked = true;
                if !peer_state.client_interested {
                    send_interested(&mut wr).await?;
                    peer_state.client_interested = true;
                }
            }
            Message::Unchoke(_) => {
                peer_state.client_choked = false;
                if !peer_state.client_interested {
                    send_interested(&mut wr).await?;
                    peer_state.client_interested = true;
                }
                request_block(md, piece_index, block_index, &mut wr).await?;
            }
            Message::Interested(_) => {
                peer_state.peer_interested = true;
                peer_state.peer_choked = false;
            }
            Message::NotInterested(_) => peer_state.peer_interested = false,
            Message::KeepAlive(_) => {
                request_block(md, piece_index, block_index, &mut wr).await?;
            }
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

async fn request_block(
    md: &Metadata,
    piece_index: u32,
    block_index: u32,
    wr: &mut WriteHalf<TcpStream>,
) -> Result<(), Box<dyn Error>> {
    let request_msg = Message::from(Request {
        index: piece_index,
        begin: block_index * (2 << 13),
        length: md.block_len(piece_index, block_index),
    });
    wr.write_all(&request_msg.serialise()).await?;
    Ok(())
}

async fn send_interested(wr: &mut WriteHalf<TcpStream>) -> Result<(), Box<dyn Error>> {
    let interest_msg = Message::from(Interested {});
    wr.write_all(&interest_msg.serialise()).await?;
    Ok(())
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
