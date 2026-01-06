use std::net::{Ipv4Addr, SocketAddrV4};

use byteorder::{BigEndian, ReadBytesExt};

pub mod ring_buffer;
pub mod logger;

#[macro_export]
macro_rules! log {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {{
        let file_line = format!("[{}:{}]", file!(), line!());
        print!("{} ", colored::Colorize::cyan(colored::Colorize::bold(file_line.as_str())));
        println!($($arg)*);
    }};
}

#[macro_export]
macro_rules! log_warn {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {{
        let file_line = format!("[{}:{}]", file!(), line!());
        print!("{} ", colored::Colorize::yellow(colored::Colorize::bold(file_line.as_str())));
        println!($($arg)*);
    }};
}

#[macro_export]
macro_rules! log_err {
    () => {
        $crate::print!("\n")
    };
    ($($arg:tt)*) => {{
        let file_line = format!("[{}:{}]", file!(), line!());
        print!("{} ", colored::Colorize::red(colored::Colorize::bold(file_line.as_str())));
        println!($($arg)*);
    }};
}

#[allow(dead_code)]
pub(crate) fn count_ones(v: &Vec<bool>) -> u32 {
    return v.iter().filter(|&&x| x).count().try_into().unwrap();
}

#[allow(dead_code)]
pub(crate) fn fuzzy_xor_distance(x: &Vec<u8>, y: &Vec<u8>) -> u32 {
    if x.len() != y.len() {
        log!("Error: mismatched sizes (x: {}, y: {})", x.len(), y.len());
    }

    let mut res = 0;

    for v in x.iter().zip(y.iter()).map(|(a, b)| a ^ b) {
        if v == 0 {
            res += 8
        } else {
            res += v.leading_zeros();
            break;
        }
    }
    return x.len() as u32 * 8 - res;
}

pub(crate) fn addr_from_bytes(bytes: &[u8]) -> Result<SocketAddrV4, ()> {
    if bytes.len() < 6 {
        return Err(());
    }

    let mut ip_raw = &bytes[..4];
    let mut port_raw = &bytes[4..6];
    let ip = ip_raw.read_u32::<BigEndian>().unwrap();
    let port = port_raw.read_u16::<BigEndian>().unwrap();
    Ok(SocketAddrV4::new(Ipv4Addr::from_bits(ip), port))
}
