use std::net::{Ipv4Addr, SocketAddrV4};

use byteorder::{BigEndian, ReadBytesExt};

pub mod ring_buffer;


pub(crate) fn count_ones(v: &Vec<bool>) -> u32 {
    return v.iter().filter(|&&x| x).count().try_into().unwrap();
}

pub(crate) fn addr_from_bytes(bytes: &[u8]) -> Result<SocketAddrV4, ()> {
    if bytes.len() < 6 {
        return Err(())
    }

    let mut ip_raw = &bytes[..4];
    let mut port_raw = &bytes[4..6];
    let ip = ip_raw.read_u32::<BigEndian>().unwrap();
    let port = port_raw.read_u16::<BigEndian>().unwrap();
    Ok(SocketAddrV4::new(Ipv4Addr::from_bits(ip), port))
}