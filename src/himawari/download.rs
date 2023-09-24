use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use iced::{subscription, Subscription};
use reqwest::{
    header::{COOKIE, USER_AGENT},
    Client, Response,
};

use crate::himawari::DownloadInfo;

pub fn download_subscription(
    download_info: &DownloadInfo,
) -> Subscription<(DateTime<Utc>, Progress)> {
    let timestamp = download_info.timestamp;
    subscription::unfold(
        timestamp,
        State::Ready(download_info.clone()),
        move |state| download(timestamp, state),
    )
}

async fn download(timestamp: DateTime<Utc>, state: State) -> ((DateTime<Utc>, Progress), State) {
    match state {
        State::Ready(download_info) => {
            let response = get_response(&download_info).await;
            match response {
                Ok(response) => {
                    if let Some(total) = response.content_length() {
                        log::info!("Start downloading {}", response.url());
                        (
                            (timestamp, Progress::Started),
                            State::Downloading {
                                response,
                                total,
                                downloaded: 0,
                                data: Vec::with_capacity(total as usize),
                            },
                        )
                    } else {
                        (
                            (
                                timestamp,
                                Progress::Failed(Arc::new(anyhow!("failed to get content length"))),
                            ),
                            State::Finished,
                        )
                    }
                }
                Err(e) => ((timestamp, Progress::Failed(Arc::new(e))), State::Finished),
            }
        }
        State::Downloading {
            mut response,
            total,
            downloaded,
            mut data,
        } => match response.chunk().await {
            Ok(Some(chunk)) => {
                let downloaded = downloaded + chunk.len() as u64;
                let percentage = (downloaded as f32 / total as f32) * 100.0;
                // log::debug!("Download progress {downloaded}/{total} ({percentage:.2}%)");
                data.extend(chunk);
                (
                    (timestamp, Progress::Advanced(percentage)),
                    State::Downloading {
                        response,
                        total,
                        downloaded,
                        data,
                    },
                )
            }
            Ok(None) => {
                log::info!("Download finished");
                ((timestamp, Progress::Finished(data)), State::Finished)
            }
            Err(e) => (
                (timestamp, Progress::Failed(Arc::new(e.into()))),
                State::Finished,
            ),
        },
        State::Finished => {
            // ここで停止
            iced::futures::future::pending().await
        }
    }
}

async fn get_response(download_info: &DownloadInfo) -> anyhow::Result<Response> {
    let client = Client::new();
    let mut params = HashMap::new();
    params.insert("_method", "POST");
    params.insert("data[FileSearch][is_compress]", "false");
    params.insert("data[FileSearch][fixedToken]", &download_info.token);
    params.insert("data[FileSearch][hashUrl]", "bDw2maKV");
    params.insert("action", "dir_download_dl");
    params.insert("filelist[0]", &download_info.dl_path);
    params.insert("dl_path", &download_info.dl_path);

    let response = client
        .post("https://sc-nc-web.nict.go.jp/wsdb_osndisk/fileSearch/download")
        .form(&params)
        .header(COOKIE, format!("CAKEPHP={}", download_info.cakephp_cookie))
        .header(USER_AGENT, &download_info.user_agent) // NOTE: これが最初のトークン取得時のものと一致していないといけない
        .send()
        .await?
        .error_for_status()?;

    Ok(response)
}

#[derive(Debug, Clone)]
pub enum Progress {
    Started,
    Advanced(f32),
    Finished(Vec<u8>),
    Failed(Arc<anyhow::Error>),
}

enum State {
    Ready(DownloadInfo),
    Downloading {
        response: Response,
        total: u64,
        downloaded: u64,
        data: Vec<u8>,
    },
    Finished,
}
