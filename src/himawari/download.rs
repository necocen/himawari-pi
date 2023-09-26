use std::sync::Arc;

use anyhow::Context as _;
use futures::{future::try_join_all, stream::FuturesUnordered, FutureExt, StreamExt};
use iced::{subscription, Subscription};
use reqwest::{Client, Response};

use crate::himawari::DownloadId;

pub fn download_subscription(id: DownloadId) -> Subscription<(DownloadId, Progress)> {
    subscription::unfold(id, State::Ready(id), move |state| download(id, state))
}

async fn download(timestamp: DownloadId, state: State) -> ((DownloadId, Progress), State) {
    match state {
        State::Ready(id) => {
            let items = match get_download_items(&id).await {
                Ok(items) => items,
                Err(e) => {
                    return ((timestamp, Progress::Failed(Arc::new(e))), State::Finished);
                }
            };
            log::info!("Start downloading");
            ((timestamp, Progress::Started), State::Downloading(items))
        }
        State::Downloading(mut items) => {
            let first_result = {
                // 未完了のダウンロードのchunkをFuturesUnorderedで並行実行し、最初に返ってきたものをnext()で取得する
                items
                    .iter_mut()
                    .enumerate()
                    .filter(|(_, item)| !item.is_finished)
                    .map(|(i, item)| item.response.chunk().map(move |result| (i, result)))
                    .collect::<FuturesUnordered<_>>()
                    .next()
                    .await
            };

            let Some((i, result)) = first_result else {
                // Noneということは元々0要素だったということ　つまりすべて完了済み
                log::info!("Download finished");
                return (
                    (timestamp, Progress::Finished(items.map(|item| item.data))),
                    State::Finished,
                );
            };

            match result {
                Ok(Some(chunk)) => {
                    items[i].downloaded += chunk.len() as u64;
                    items[i].data.extend(chunk);
                }
                Ok(None) => {
                    items[i].is_finished = true;
                }
                Err(e) => {
                    return (
                        (timestamp, Progress::Failed(Arc::new(e.into()))),
                        State::Finished,
                    );
                }
            }

            let downloaded: u64 = items.iter().map(|item| item.downloaded).sum();
            let total: u64 = items.iter().map(|item| item.total).sum();
            let percentage = downloaded as f32 / total as f32;

            // log::info!(
            //     "{} {} {} {} {percentage}",
            //     items[0].is_finished,
            //     items[1].is_finished,
            //     items[2].is_finished,
            //     items[3].is_finished
            // );
            (
                (timestamp, Progress::Advanced(percentage)),
                State::Downloading(items),
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
    Finished([Vec<u8>; 4]),
    Failed(Arc<anyhow::Error>),
}

enum State {
    Ready(DownloadId),
    Downloading([DownloadItem; 4]),
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

async fn get_download_items(id: &DownloadId) -> anyhow::Result<[DownloadItem; 4]> {
    let client = Client::new();
    let url =
        id.0.format("https://himawari8.nict.go.jp/img/D531106/2d/550/%Y/%m/%d/%H%M%S")
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
