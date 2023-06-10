mod fileinfo;
mod parse;
use parse::Info;

use bendy::{
    self,
    decoding::{Decoder, FromBencode, Object},
    encoding::ToBencode,
};

use sha1;
use sha1::{Digest, Sha1};

fn get_info_hash(path: &String) -> Result<Vec<u8>, ()> {
    let res = std::fs::read(path).expect("Failed to read file");

    let mut dec = Decoder::new(&res).with_max_depth(5);

    let mut dict = match dec.next_object().unwrap() {
        Some(Object::Dict(d)) => d,
        _ => return Err(()),
    };

    let info = loop {
        match dict.next_pair().unwrap() {
            Some((b"info", val)) => break Info::decode_bencode_object(val).unwrap(),
            Some(_) => {
                continue;
            }
            None => return Err(()),
        };
    };

    let bytes = info.to_bencode().unwrap();

    let mut hasher: Sha1 = Sha1::new();
    hasher.update(bytes);
    let result: Vec<u8> = hasher.finalize().to_vec();

    Ok(result)
}

fn main() {
    let s: String = String::from("torrents/test.torrent");
    let x = get_info_hash(&s);
}
