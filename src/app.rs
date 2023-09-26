use std::{
    fs::read_dir,
    io::Cursor,
    iter,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, NaiveDateTime, Utc};
use iced::{
    theme,
    widget::{
        button, column, container, horizontal_space, image as iced_image, pick_list, row,
        scrollable, text, Column, Space,
    },
    window, Application, Color, Command, Element, Length, Subscription,
};
use image::{
    imageops::{self, FilterType},
    DynamicImage, RgbImage,
};
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    himawari::{self, DownloadInfo, Progress},
    modal::Modal,
};

pub struct App {
    images: Vec<Image>,
    download: Option<Download>,
    current_image: Option<(usize, iced_image::Handle)>,
    shows_menu: bool,
}

#[derive(Debug, Clone)]
pub struct Image {
    path: PathBuf,
    timestamp: DateTime<Utc>,
}

impl Image {
    fn view(&self) -> Element<Message> {
        let timestamp = self
            .timestamp
            .with_timezone(&chrono_tz::Asia::Tokyo)
            .format("%Y-%m-%d %H:%M");
        button(text(timestamp).size(30))
            .on_press(Message::SelectImage(self.clone()))
            .style(theme::Button::Text)
            .into()
    }
}

#[derive(Debug)]
struct Download {
    info: DownloadInfo,
    state: DownloadState,
}

impl Download {
    fn new(info: DownloadInfo) -> Self {
        Download {
            info,
            state: DownloadState::Starting,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        himawari::download_subscription(&self.info)
            .map(|(ts, p)| Message::DownloadProgressed(ts, p))
    }
}

#[derive(Debug)]
enum DownloadState {
    Starting,
    Downloading { progress: f32 },
    Finished,
    Failed(Arc<anyhow::Error>),
}

#[derive(Debug, Clone)]
pub enum Message {
    Fetch,
    Download(DownloadInfo),
    DownloadProgressed(DateTime<Utc>, Progress),
    DownloadCompleted(Image),
    ShowMenu,
    HideMenu,
    SelectImage(Image),
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, iced::Command<Self::Message>) {
        // FIXME: ここが同期なのは不満がある
        let images = Self::get_images();
        let current_image = images
            .iter()
            .enumerate()
            .last()
            .map(|(i, image)| (i, iced_image::Handle::from_path(&image.path)));
        (
            App {
                images,
                download: None,
                current_image,
                shows_menu: false,
            },
            Command::batch(vec![
                window::change_mode(window::Mode::Fullscreen),
                Command::perform(async {}, |_| Message::Fetch),
            ]),
        )
    }

    fn title(&self) -> String {
        "Himawari".to_string()
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            Message::ShowMenu => {
                self.shows_menu = true;
                Command::none()
            }
            Message::HideMenu => {
                self.shows_menu = false;
                Command::none()
            }
            Message::SelectImage(image) => {
                self.current_image = self
                    .images
                    .iter()
                    .enumerate()
                    .find(|(_, img)| img.timestamp == image.timestamp)
                    .map(|(i, image)| (i, iced_image::Handle::from_path(&image.path)));
                Command::none()
            }
            Message::Fetch => {
                Command::perform(himawari::fetch_download_info(), |result| match result {
                    Ok(info) => Message::Download(info),
                    Err(e) => {
                        // TODO: 「無」のMessageを定義すべきなのか？
                        log::error!("{e}");
                        panic!("{e}")
                    }
                })
            }
            Message::Download(info) => {
                if let Some(image) = self
                    .images
                    .iter()
                    .find(|image| image.timestamp == info.timestamp)
                {
                    log::debug!("Already downloaded: {}", image.path.display());
                    return Command::none();
                }
                self.download = Some(Download::new(info));
                Command::none()
            }
            Message::DownloadProgressed(_, Progress::Started) => {
                self.download.as_mut().unwrap().state =
                    DownloadState::Downloading { progress: 0.0 };
                Command::none()
            }
            Message::DownloadProgressed(_, Progress::Advanced(progress)) => {
                self.download.as_mut().unwrap().state = DownloadState::Downloading { progress };
                Command::none()
            }
            Message::DownloadProgressed(_, Progress::Failed(e)) => {
                log::error!("failed to download image: {e}");
                self.download.as_mut().unwrap().state = DownloadState::Failed(e);
                Command::none()
            }
            Message::DownloadProgressed(timestamp, Progress::Finished(data)) => {
                self.download.as_mut().unwrap().state = DownloadState::Finished;
                Command::perform(
                    App::resize_and_save_image(timestamp, data),
                    |result| match result {
                        Ok(image) => Message::DownloadCompleted(image),
                        Err(e) => panic!("{e}"),
                    },
                )
            }
            Message::DownloadCompleted(image) => {
                self.download = None;
                self.images.push(image);
                if let Some((i, _)) = self.current_image {
                    // current_imageが最新の画像だったら新しい画像に追従する
                    if i == self.images.len() - 2 {
                        self.current_image = self
                            .images
                            .iter()
                            .enumerate()
                            .last()
                            .map(|(i, image)| (i, iced_image::Handle::from_path(&image.path)));
                    }
                } else {
                    self.current_image = self
                        .images
                        .iter()
                        .enumerate()
                        .last()
                        .map(|(i, image)| (i, iced_image::Handle::from_path(&image.path)));
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<Message> {
        let Some((_, handle)) = &self.current_image else {
            return Space::new(Length::Fill, Length::Fill).into();
        };
        let content = iced_image::Image::new(handle.clone())
            .width(Length::Fill)
            .height(Length::Fill);
        let content = button(content)
            .on_press(Message::ShowMenu)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(0)
            .style(theme::Button::Text);
        let images = scrollable(
            Column::with_children(self.images.iter().rev().map(Image::view).collect()).spacing(10),
        )
        .width(Length::Fill)
        .direction(scrollable::Direction::Vertical(
            scrollable::Properties::new().width(50).scroller_width(50),
        ))
        .height(300);
        let modal = container(
            column![
                text("Sign Up").size(24),
                column![images, button("Close").on_press(Message::HideMenu),].spacing(10)
            ]
            .spacing(20),
        )
        .width(700)
        .padding(10)
        .style(theme::Container::Transparent);

        if self.shows_menu {
            Modal::new(content, modal).into()
        } else {
            content.into()
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        let fetch = iter::once(iced::time::every(Duration::from_secs(300)).map(|_| Message::Fetch));
        let progress = self
            .download
            .as_ref()
            .map(Download::subscription)
            .into_iter();

        Subscription::batch(progress.chain(fetch))
    }

    fn theme(&self) -> Self::Theme {
        iced::Theme::Dark
    }
}

impl App {
    const IMAGE_DIR: &'static str = "./images";

    fn get_images() -> Vec<Image> {
        match read_dir(Self::IMAGE_DIR) {
            Ok(paths) => {
                let mut images = paths
                    .filter_map(|path| {
                        let path = path.ok()?.path();
                        let file_name = path.file_name()?.to_str()?;
                        let Ok(timestamp) =
                            NaiveDateTime::parse_from_str(file_name, "%Y%m%d%H%M%S.png")
                        else {
                            log::warn!("unexpected filename: {file_name}");
                            return None;
                        };
                        let timestamp = timestamp.and_utc();
                        Some(Image { path, timestamp })
                    })
                    .collect::<Vec<_>>();
                images.sort_by(|i1, i2| i1.timestamp.cmp(&i2.timestamp));
                images
            }
            Err(e) => {
                log::error!("{e}");
                vec![]
            }
        }
    }

    async fn resize_and_save_image(
        timestamp: DateTime<Utc>,
        [data00, data01, data10, data11]: [Vec<u8>; 4],
    ) -> anyhow::Result<Image> {
        let top_left = Self::resize_image(data00);
        let bottom_left = Self::resize_image(data01);
        let top_right = Self::resize_image(data10);
        let bottom_right = Self::resize_image(data11);
        let (top_left, bottom_left, top_right, bottom_right) =
            tokio::try_join!(top_left, bottom_left, top_right, bottom_right)?;
        let mut combined = RgbImage::new(1080, 1080);
        imageops::replace(&mut combined, &top_left.to_rgb8(), 0, 0);
        imageops::replace(&mut combined, &bottom_left.to_rgb8(), 0, 540);
        imageops::replace(&mut combined, &top_right.to_rgb8(), 540, 0);
        imageops::replace(&mut combined, &bottom_right.to_rgb8(), 540, 540);

        let image_path = Path::new(&format!(
            "{}/{}.png",
            App::IMAGE_DIR,
            timestamp.format("%Y%m%d%H%M%S")
        ))
        .to_path_buf();
        if fs::metadata(App::IMAGE_DIR).await.is_err() {
            fs::create_dir(App::IMAGE_DIR).await?;
        }
        combined.save(&image_path)?;
        log::info!("Image saved: {}", image_path.display());
        Ok(Image {
            path: image_path,
            timestamp,
        })
    }

    async fn resize_image(data: Vec<u8>) -> anyhow::Result<DynamicImage> {
        let task = tokio::task::spawn_blocking(move || -> anyhow::Result<DynamicImage> {
            log::info!("Resizing image");
            let image = image::load_from_memory(&data)?.resize(540, 540, FilterType::Lanczos3);
            log::info!("Resized image");
            Ok(image)
        });
        task.await?
    }
}
