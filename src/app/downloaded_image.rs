use std::path::PathBuf;

use iced::{
    theme,
    widget::{button, text},
    Element,
};

use crate::himawari::DownloadId;

use super::Message;

#[derive(Debug, Clone)]
pub struct DownloadedImage {
    pub path: PathBuf,
    pub id: DownloadId,
}

impl DownloadedImage {
    pub fn view(&self) -> Element<Message> {
        let timestamp = self.id.as_local_datetime().format("%Y-%m-%d %H:%M");
        button(text(timestamp).size(30))
            .on_press(Message::SelectImage(self.clone()))
            .style(theme::Button::Text)
            .into()
    }
}
