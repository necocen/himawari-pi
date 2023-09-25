use std::{future::Future, sync::Arc};

use anyhow::Context as _;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::future::{try_join_all, Either};
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
            let [item00, item01, item10, item11] = match get_download_items(&download_info).await {
                Ok(items) => items,
                Err(e) => {
                    return ((timestamp, Progress::Failed(Arc::new(e))), State::Finished);
                }
            };
            log::info!("Start downloading");
            (
                (timestamp, Progress::Started),
                State::Downloading {
                    items: [[item00, item01], [item10, item11]],
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

#[derive(Debug)]
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

async fn get_download_items(download_info: &DownloadInfo) -> anyhow::Result<[DownloadItem; 4]> {
    let client = Client::new();
    let url = download_info
        .timestamp
        .format("https://himawari8.nict.go.jp/img/D531106/2d/550/%Y/%m/%d/%H%M%S")
        .to_string();
    let urls = [
        format!("{url}_0_0.png"),
        format!("{url}_0_1.png"),
        format!("{url}_1_0.png"),
        format!("{url}_1_1.png"),
    ];
    let futures = urls.map(|u| client.get(u).send());
    let responses = try_join_all(futures).await?;
    let items = responses
        .into_iter()
        .map(|response| {
            response.content_length().map(|total| DownloadItem {
                response,
                total,
                downloaded: 0,
                data: Vec::with_capacity(total as usize),
                is_finished: false,
            })
        })
        .collect::<Option<Vec<_>>>()
        .map(|vec| vec.try_into().unwrap())
        .with_context(|| "failed to get content_length")?;
    Ok(items)
}
