use chrono::{DateTime, Utc};
mod download;
mod fetch;

pub use download::{download_subscription, Progress};
pub use fetch::fetch_download_info;

#[derive(Debug, Clone)]
pub struct DownloadInfo {
    cakephp_cookie: String,
    token: String,
    dl_path: String,
    user_agent: String,
    pub timestamp: DateTime<Utc>,
}
