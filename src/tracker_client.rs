use bendy::decoding::FromBencode;
use reqwest::Client;

use crate::parser::{
    metadata::{get_info_hash, Metadata},
    trackerinfo::TrackerInfo,
};

pub(crate) async fn req_tracker_info(
    md: &Metadata,
) -> Result<TrackerInfo, Box<dyn std::error::Error>> {
    let tracker_url = &md.announce;
    let hash = get_info_hash(&md).unwrap();
    let peer_id = String::from("%AA%AC%AD%C7%9C%90%1A%F8%1B%CC%C8%C6%EF%AD%A9%C7%DE%F9%99%CB");
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
