use std::fmt::Display;

use byteorder::{BigEndian, ReadBytesExt};
use enum_dispatch::enum_dispatch;
use urlencoding::encode_binary;

use self::{
    have::Have, interested::Interested, keep_alive::KeepAlive, not_interested::NotInterested,
    request::Request,
};

pub mod have;
pub mod interested;
pub mod keep_alive;
pub mod not_interested;
pub mod request;

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
        match self.id() {
            Some(id) => format!(
                "{}: <{:0>4}><{}><{:?}>",
                self.name(),
                self.len().to_string(),
                id,
                &self.payload()
            ),
            None => format!("{}: <{:0>4}><><>", self.name(), self.len().to_string()),
        }
    }
}

#[enum_dispatch(PeerWireMessage)]
pub enum Message {
    KeepAlive(KeepAlive),
    Interested(Interested),
    NotInterested(NotInterested),
    Have(Have),
    Request(Request),
}

pub fn parse(raw: Vec<u8>) -> Result<Message, ()> {
    if raw.len() < 4 {
        return Err(());
    }

    let mut len_prefix: &[u8] = &raw[0..4];
    let len_prefix: u32 = len_prefix.read_u32::<BigEndian>().unwrap();

    let id: u8 = match len_prefix {
        0 => return Ok(Message::from(KeepAlive {})),
        _ => raw[4],
    };

    match id {
        2 => {
            if raw.len() != 5 {
                return Err(());
            }
            return Ok(Message::from(Interested {}));
        }
        3 => {
            if raw.len() != 5 {
                return Err(());
            }
            return Ok(Message::from(NotInterested {}));
        }
        4 => {
            if raw.len() != 9 {
                return Err(());
            }
            let mut piece_index = &raw[5..9];
            let piece_index = piece_index.read_u32::<BigEndian>().unwrap();
            return Ok(Message::from(Have { piece_index }));
        }
        6 => {
            if raw.len() != 17 {
                return Err(());
            }
            let mut index = &raw[5..9];
            let mut begin = &raw[9..13];
            let mut length = &raw[13..17];

            let index = index.read_u32::<BigEndian>().unwrap();
            let begin = begin.read_u32::<BigEndian>().unwrap();
            let length = length.read_u32::<BigEndian>().unwrap();

            return Ok(Message::from(Request {
                index,
                begin,
                length,
            }));
        }
        _ => (&raw[5..]).to_vec(),
    };

    Err(())
}
