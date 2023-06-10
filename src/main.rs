mod parser;

use tokio::{self};

use crate::parser::metadata::{get_info_hash, read_metadata};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let s: String = String::from("torrents/test.torrent");
    let info = read_metadata(&s).unwrap();
    let hash = get_info_hash(&info);
    println!("{}", hash.unwrap());

    Ok(())
}
