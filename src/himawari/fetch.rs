use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::Client;

use super::DownloadInfo;

pub async fn fetch_download_info() -> anyhow::Result<DownloadInfo> {
    let client = Client::new();
    let latest_timestamp: LatestTimestamp = client
        .get("https://himawari8.nict.go.jp/img/FULL_24h/latest.json")
        .send()
        .await?
        .json()
        .await?;

    Ok(DownloadInfo {
        timestamp: latest_timestamp.datetime()?,
    })
}

#[derive(serde::Deserialize)]
struct LatestTimestamp {
    date: String,
}

impl LatestTimestamp {
    fn datetime(&self) -> anyhow::Result<DateTime<Utc>> {
        Ok(NaiveDateTime::parse_from_str(&self.date, "%Y-%m-%d %H:%M:%S")?.and_utc())
    }
}
