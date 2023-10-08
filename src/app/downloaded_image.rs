use std::path::PathBuf;

use iced::{
    theme,
    widget::{button, text},
    Color, Element,
};

use crate::himawari::DownloadId;

use super::Message;

#[derive(Debug, Clone)]
pub struct DownloadedImage {
    pub path: PathBuf,
    pub id: DownloadId,
}

impl DownloadedImage {
    pub fn view(&self, is_selected: bool) -> Element<Message> {
        let timestamp = self.id.as_local_datetime().format("%Y-%m-%d %H:%M");
        let text_color = if is_selected {
            Color::from_rgb8(0xff, 0xf1, 0x00) // Yellow
        } else {
            Color::WHITE
        };
        button(
            text(timestamp)
                .size(30)
                .style(theme::Text::Color(text_color)),
        )
        .on_press(Message::SelectImage(self.clone()))
        .style(theme::Button::Text)
        .into()
    }
}
