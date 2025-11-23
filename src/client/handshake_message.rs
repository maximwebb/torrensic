use crate::setup::magnet_link::InfoHash;

pub fn get_handshake_bytes(info_hash: &InfoHash) -> Vec<u8> {
    let pstr: Vec<u8> = b"BitTorrent protocol".to_vec();
    let pstrlen: Vec<u8> = vec![pstr.len().try_into().unwrap()];
    let mut reserved: Vec<u8> = vec![0; 8];
    let peer_id: Vec<u8> = b"-TO0000-0123456789AB".to_vec();

    // Request metadata
    reserved[5] |= 0x10;

    let msg = [
        pstrlen.as_slice(),
        pstr.as_slice(),
        reserved.as_slice(),
        info_hash,
        peer_id.as_slice(),
    ]
    .concat();

    msg
}
