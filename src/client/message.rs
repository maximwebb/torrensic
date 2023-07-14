use std::cmp::min;

use byteorder::{BigEndian, ReadBytesExt};
use enum_dispatch::enum_dispatch;

use self::{
    bitfield::Bitfield, cancel::Cancel, choke::Choke, have::Have, interested::Interested,
    keep_alive::KeepAlive, not_interested::NotInterested, piece::Piece, request::Request,
    unchoke::Unchoke,
};

pub mod bitfield;
pub mod cancel;
pub mod choke;
pub mod have;
pub mod interested;
pub mod keep_alive;
pub mod not_interested;
pub mod piece;
pub mod request;
pub mod unchoke;

#[enum_dispatch]
pub trait PeerWireMessage {
    fn serialise(&self) -> Vec<u8> {
        let len_prefix = self.len().to_be_bytes().to_vec();
        match self.id() {
            Some(id) => [len_prefix, vec![id], self.payload()].concat(),
            None => len_prefix,
        }
    }

    fn id(&self) -> Option<u8> {
        return None;
    }

    fn len(&self) -> u32 {
        let id_len = match self.id() {
            Some(_) => 1,
            None => 0,
        };

        return (id_len + self.payload().len()).try_into().unwrap();
    }

    fn payload(&self) -> Vec<u8> {
        return Vec::new();
    }

    fn name(&self) -> String;

    fn print(&self) -> String {
        let payload_len = min(self.payload().len(), 60);
        match self.id() {
            Some(id) => format!(
                "{}: <{:0>4}><{}><{:?}>",
                self.name(),
                self.len().to_string(),
                id,
                &self.payload()[..payload_len]
            ),
            None => format!("{}: <{:0>4}><><>", self.name(), self.len().to_string()),
        }
    }
}

#[enum_dispatch(PeerWireMessage)]
pub enum Message {
    KeepAlive(KeepAlive),
    Choke(Choke),
    Unchoke(Unchoke),
    Interested(Interested),
    NotInterested(NotInterested),
    Have(Have),
    Bitfield(Bitfield),
    Request(Request),
    Piece(Piece),
    Cancel(Cancel),
}

pub fn parse(raw: Vec<u8>) -> Result<(Message, Vec<u8>), ()> {
    if raw.len() < 4 {
        return Err(());
    }

    let mut len_prefix: &[u8] = &raw[0..4];
    let len_prefix: u32 = len_prefix.read_u32::<BigEndian>().unwrap();

    if len_prefix > 200_000 {
        return Err(());
    }

    if len_prefix + 4 > raw.len().try_into().unwrap() {
        return Err(());
    }

    // Capture remaining bytes
    let msg_len: usize = (len_prefix + 4).try_into().unwrap();
    let rem = raw[msg_len..].to_vec();

    let id: u8 = match len_prefix {
        0 => return Ok((Message::from(KeepAlive {}), rem)),
        _ => raw[4],
    };

    match id {
        0 => {
            if len_prefix != 1 {
                return Err(());
            }
            return Ok((Message::from(Choke {}), rem));
        }
        1 => {
            if len_prefix != 1 {
                return Err(());
            }
            return Ok((Message::from(Unchoke {}), rem));
        }
        2 => {
            if len_prefix != 1 {
                return Err(());
            }
            return Ok((Message::from(Interested {}), rem));
        }
        3 => {
            if len_prefix != 1 {
                return Err(());
            }
            return Ok((Message::from(NotInterested {}), rem));
        }
        4 => {
            if len_prefix != 5 {
                return Err(());
            }
            let mut piece_index = &raw[5..9];
            let piece_index = piece_index.read_u32::<BigEndian>().unwrap();
            return Ok((
                Message::from(Have {
                    piece_index: piece_index.try_into().unwrap(),
                }),
                rem,
            ));
        }
        5 => {
            let bitfield = raw[5..].to_vec();
            return Ok((Message::from(Bitfield { bitfield }), rem));
        }
        6 => {
            if len_prefix != 13 {
                return Err(());
            }
            let mut index = &raw[5..9];
            let mut begin = &raw[9..13];
            let mut length = &raw[13..17];

            let index = index.read_u32::<BigEndian>().unwrap();
            let begin = begin.read_u32::<BigEndian>().unwrap();
            let length = length.read_u32::<BigEndian>().unwrap();

            return Ok((
                Message::from(Request {
                    index,
                    begin,
                    length,
                }),
                rem,
            ));
        }
        7 => {
            if len_prefix < 10 {
                return Err(());
            }

            let mut index = &raw[5..9];
            let mut begin = &raw[9..13];
            let block = raw[13..].to_vec();

            let index = index.read_u32::<BigEndian>().unwrap();
            let begin = begin.read_u32::<BigEndian>().unwrap();

            return Ok((
                Message::from(Piece {
                    index,
                    begin,
                    block,
                }),
                rem,
            ));
        }
        8 => {
            if len_prefix != 13 {
                return Err(());
            }
            let mut index = &raw[5..9];
            let mut begin = &raw[9..13];
            let mut length = &raw[13..17];

            let index = index.read_u32::<BigEndian>().unwrap();
            let begin = begin.read_u32::<BigEndian>().unwrap();
            let length = length.read_u32::<BigEndian>().unwrap();

            return Ok((
                Message::from(Cancel {
                    index,
                    begin,
                    length,
                }),
                rem,
            ));
        }
        _ => Err(()),
    }
}
