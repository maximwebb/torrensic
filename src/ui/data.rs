use serde_json;
use std::collections::HashMap;

use reqwest::Client;

pub(crate) struct LatLon {
    pub(crate) lat: f32,
    pub(crate) lon: f32,
}

pub(crate) async fn get_ip_locations(
    hosts: Vec<String>,
) -> Result<HashMap<String, Option<LatLon>>, Box<dyn std::error::Error>> {
    let client = Client::new();
    let body = serde_json::to_string(&hosts[..100])?;

    let raw = client
        .post("http://ip-api.com/batch")
        .query(&[("fields", "status,lat,lon")])
        .body(body)
        .send()
        .await?
        .text()
        .await?;

    let data: Vec<serde_json::Value> = serde_json::from_str(&raw)?;

    // Initialise all entries to None
    let mut res: HashMap<String, Option<LatLon>> = HashMap::new();
    for host in &hosts {
        res.insert(host.to_string(), None);
    }

    for (host, loc) in hosts.iter().zip(data.iter()) {
        if let Some(status) = loc.get("status") {
            if status == "success" {
                res.insert(
                    host.to_string(),
                    Some(LatLon {
                        lat: loc
                            .get("lat")
                            .expect("Missing lat field")
                            .as_f64()
                            .expect("") as f32,
                        lon: loc
                            .get("lon")
                            .expect("Missing lon field")
                            .as_f64()
                            .expect("") as f32,
                    }),
                );
            } else {
                res.insert(host.to_string(), None);
            }
        } else {
            res.insert(host.to_string(), None);
        }
    }

    return Ok(res);
}
