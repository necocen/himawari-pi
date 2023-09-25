use chrono::{DateTime, Utc};
mod download;
mod fetch;

pub use download::{download_subscription, Progress};
pub use fetch::fetch_download_info;

#[derive(Debug, Clone)]
pub struct DownloadInfo {
    pub timestamp: DateTime<Utc>,
}
