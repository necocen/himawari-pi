use std::sync::Arc;

use iced::{
    theme,
    widget::{container, text},
    Color, Element, Subscription,
};

use crate::himawari::{download_subscription, DownloadId};

use super::Message;

#[derive(Debug)]
#[non_exhaustive]
pub struct DownloadingImage {
    pub id: DownloadId,
    pub state: DownloadState,
}

impl DownloadingImage {
    pub fn new(id: DownloadId) -> Self {
        DownloadingImage {
            id,
            state: DownloadState::Starting,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        download_subscription(self.id).map(|(id, p)| Message::DownloadProgressed(id, p))
    }

    pub fn view(&self) -> Element<Message> {
        let timestamp = self.id.as_local_datetime().format("%Y-%m-%d %H:%M");
        container(
            text(timestamp)
                .size(30)
                .style(theme::Text::Color(Color::from_rgb8(128, 128, 128))),
        )
        .padding(5)
        .into()
    }
}

#[derive(Debug)]
pub enum DownloadState {
    Starting,
    Downloading { progress: f32 },
    Finished,
    Failed(Arc<anyhow::Error>),
}
