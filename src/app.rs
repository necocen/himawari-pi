use std::{path::PathBuf, time::Duration};

use iced::{
    widget::{
        image::{self, Handle},
        Space,
    },
    window, Application, Command, Length,
};

use crate::himawari;

pub struct App {
    image: Option<image::Handle>,
}

#[derive(Debug)]
pub enum Message {
    Update,
    Image(anyhow::Result<PathBuf>),
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, iced::Command<Self::Message>) {
        (
            App { image: None },
            Command::batch(vec![
                window::change_mode(window::Mode::Fullscreen),
                Command::perform(App::update_image(), Message::Image),
            ]),
        )
    }

    fn title(&self) -> String {
        "Himawari".to_string()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::Update => Command::perform(App::update_image(), Message::Image),
            Message::Image(Ok(path_buf)) => {
                self.image = Some(Handle::from_path(path_buf));
                Command::none()
            }
            Message::Image(Err(e)) => {
                log::error!("{e}");
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<Message> {
        if let Some(handle) = &self.image {
            image::Image::new(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            Space::new(Length::Fill, Length::Fill).into()
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_secs(300)).map(|_| Message::Update)
    }
}

impl App {
    async fn update_image() -> anyhow::Result<PathBuf> {
        let info = himawari::fetch_download_info().await?;
        himawari::get_image(&info).await
    }
}
