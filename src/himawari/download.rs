use std::{future::Future, sync::Arc};

use anyhow::anyhow;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::future::Either;
use iced::{subscription, Subscription};
use reqwest::{Client, Response};
use tokio::select;

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
            let url = download_info
                .timestamp
                .format("https://himawari8.nict.go.jp/img/D531106/2d/550/%Y/%m/%d/%H%M%S")
                .to_string();
            let client = Client::new();
            let Ok(res00) = client.get(url.clone() + "_0_0.png").send().await else {
                return (
                    (timestamp, Progress::Failed(Arc::new(anyhow!("fail 00")))),
                    State::Finished,
                );
            };
            let Ok(res01) = client.get(url.clone() + "_0_1.png").send().await else {
                return (
                    (timestamp, Progress::Failed(Arc::new(anyhow!("fail 01")))),
                    State::Finished,
                );
            };
            let Ok(res10) = client.get(url.clone() + "_1_0.png").send().await else {
                return (
                    (timestamp, Progress::Failed(Arc::new(anyhow!("fail 10")))),
                    State::Finished,
                );
            };
            let Ok(res11) = client.get(url + "_1_1.png").send().await else {
                return (
                    (timestamp, Progress::Failed(Arc::new(anyhow!("fail 11")))),
                    State::Finished,
                );
            };
            let Some(total00) = res00.content_length() else {
                return (
                    (
                        timestamp,
                        Progress::Failed(Arc::new(anyhow!("failed to get content length"))),
                    ),
                    State::Finished,
                );
            };
            let Some(total01) = res01.content_length() else {
                return (
                    (
                        timestamp,
                        Progress::Failed(Arc::new(anyhow!("failed to get content length"))),
                    ),
                    State::Finished,
                );
            };
            let Some(total10) = res10.content_length() else {
                return (
                    (
                        timestamp,
                        Progress::Failed(Arc::new(anyhow!("failed to get content length"))),
                    ),
                    State::Finished,
                );
            };
            let Some(total11) = res11.content_length() else {
                return (
                    (
                        timestamp,
                        Progress::Failed(Arc::new(anyhow!("failed to get content length"))),
                    ),
                    State::Finished,
                );
            };
            log::info!("Start downloading");
            (
                (timestamp, Progress::Started),
                State::Downloading {
                    items: [
                        [
                            DownloadItem {
                                response: res00,
                                total: total00,
                                downloaded: 0,
                                data: Vec::with_capacity(total00 as usize),
                                is_finished: false,
                            },
                            DownloadItem {
                                response: res01,
                                total: total01,
                                downloaded: 0,
                                data: Vec::with_capacity(total01 as usize),
                                is_finished: false,
                            },
                        ],
                        [
                            DownloadItem {
                                response: res10,
                                total: total10,
                                downloaded: 0,
                                data: Vec::with_capacity(total10 as usize),
                                is_finished: false,
                            },
                            DownloadItem {
                                response: res11,
                                total: total11,
                                downloaded: 0,
                                data: Vec::with_capacity(total11 as usize),
                                is_finished: false,
                            },
                        ],
                    ],
                },
            )
        }
        State::Downloading {
            items: [[mut items00, mut items01], [mut items10, mut items11]],
        } => {
            if items00.is_finished
                && items01.is_finished
                && items10.is_finished
                && items11.is_finished
            {
                log::info!("Download finished");
                return (
                    (
                        timestamp,
                        Progress::Finished([
                            [items00.data, items01.data],
                            [items10.data, items11.data],
                        ]),
                    ),
                    State::Finished,
                );
            }
            select! {
                chunk = items00.chunk() => {
                    match chunk {
                        Ok(Some(chunk)) => {
                            items00.downloaded += chunk.len() as u64;
                            items00.data.extend(chunk);
                        }
                        Ok(None) => {
                            items00.is_finished = true;
                        }
                        Err(e) => {
                            return  (
                                (timestamp, Progress::Failed(Arc::new(e.into()))),
                                State::Finished,
                            );
                        }
                    }
                }
                chunk = items01.chunk() => {
                    match chunk {
                        Ok(Some(chunk)) => {
                            items01.downloaded += chunk.len() as u64;
                            items01.data.extend(chunk);
                        }
                        Ok(None) => {
                            items01.is_finished = true;
                        }
                        Err(e) => {
                            return  (
                                (timestamp, Progress::Failed(Arc::new(e.into()))),
                                State::Finished,
                            );
                        }
                    }
                }
                chunk = items10.chunk() => {
                    match chunk {
                        Ok(Some(chunk)) => {
                            items10.downloaded += chunk.len() as u64;
                            items10.data.extend(chunk);
                        }
                        Ok(None) => {
                            items10.is_finished = true;
                        }
                        Err(e) => {
                            return  (
                                (timestamp, Progress::Failed(Arc::new(e.into()))),
                                State::Finished,
                            );
                        }
                    }
                }
                chunk = items11.chunk() => {
                    match chunk {
                        Ok(Some(chunk)) => {
                            items11.downloaded += chunk.len() as u64;
                            items11.data.extend(chunk);
                        }
                        Ok(None) => {
                            items11.is_finished = true;
                        }
                        Err(e) => {
                            return  (
                                (timestamp, Progress::Failed(Arc::new(e.into()))),
                                State::Finished,
                            );
                        }
                    }
                }
            };

            let percentage =
                (items00.downloaded + items01.downloaded + items10.downloaded + items11.downloaded)
                    as f32
                    / (items00.total + items01.total + items10.total + items11.total) as f32;

            // log::info!(
            //     "{} {} {} {} {percentage}",
            //     items00.is_finished,
            //     items01.is_finished,
            //     items10.is_finished,
            //     items11.is_finished
            // );
            (
                (timestamp, Progress::Advanced(percentage)),
                State::Downloading {
                    items: [[items00, items01], [items10, items11]],
                },
            )
        }
        State::Finished => {
            // ここで停止
            iced::futures::future::pending().await
        }
    }
}

#[derive(Debug, Clone)]
pub enum Progress {
    Started,
    Advanced(f32),
    Finished([[Vec<u8>; 2]; 2]),
    Failed(Arc<anyhow::Error>),
}

enum State {
    Ready(DownloadInfo),
    Downloading { items: [[DownloadItem; 2]; 2] },
    Finished,
}

struct DownloadItem {
    response: Response,
    total: u64,
    downloaded: u64,
    data: Vec<u8>,
    is_finished: bool,
}

impl DownloadItem {
    // 完了していたらpendingになるchunk
    fn chunk(
        &mut self,
    ) -> Either<
        futures::future::Pending<Result<Option<Bytes>, reqwest::Error>>,
        impl Future<Output = Result<Option<Bytes>, reqwest::Error>> + '_,
    > {
        if self.is_finished {
            Either::Left(futures::future::pending())
        } else {
            Either::Right(self.response.chunk())
        }
    }
}
