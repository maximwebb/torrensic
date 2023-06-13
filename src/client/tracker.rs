use bendy::decoding::FromBencode;
use reqwest::Client;
use urlencoding::encode_binary;

use crate::parser::{
    metadata::{get_info_hash, Metadata},
    trackerinfo::TrackerInfo,
};

pub(crate) async fn req_tracker_info(
    md: &Metadata,
) -> Result<TrackerInfo, Box<dyn std::error::Error>> {
    let tracker_url = &md.announce;
    let hash = get_info_hash(&md).unwrap();
    let peer_id = encode_binary(b"-TO0000-0123456789AB");
    let port = String::from("3000");
    let url = format!("{tracker_url}?info_hash={hash}&peer_id={peer_id}");

    let client = Client::new();

    let res = client
        .get(url)
        .query(&[("port", &port)])
        .send()
        .await?
        .bytes()
        .await?;

    let tracker_info = TrackerInfo::from_bencode(&res).unwrap();

    Ok(tracker_info)
}
