use chrono::{DateTime, Utc};
use reqwest::Client;

use super::{DownloadId, LATEST_JSON_URL};

pub async fn fetch_download_info() -> anyhow::Result<DownloadId> {
    let client = Client::new();
    let latest_timestamp: LatestTimestamp =
        client.get(LATEST_JSON_URL).send().await?.json().await?;

    Ok(DownloadId::new(latest_timestamp.date))
}

#[derive(serde::Deserialize)]
struct LatestTimestamp {
    #[serde(with = "date_format")]
    date: DateTime<Utc>,
}

mod date_format {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{self, Deserialize, Deserializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, FORMAT)
            .map(|d| d.and_utc())
            .map_err(serde::de::Error::custom)
    }
}
