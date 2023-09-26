use chrono::{DateTime, Local, Utc};
mod download;
mod fetch;

pub use download::{download_subscription, Progress};
pub use fetch::fetch_download_info;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DownloadId(DateTime<Utc>);

impl DownloadId {
    pub fn new(datetime: impl Into<DateTime<Utc>>) -> Self {
        Self(datetime.into())
    }

    pub fn as_utc_datetime(&self) -> DateTime<Utc> {
        self.0
    }

    pub fn as_local_datetime(&self) -> DateTime<Local> {
        DateTime::from(self.0)
    }
}
