use app::App;
use iced::{Application, Settings};

mod app;
mod himawari;
mod modal;

fn main() -> iced::Result {
    env_logger::init();
    App::run(Settings::default())
}
