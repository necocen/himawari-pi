use app::App;
use iced::{Application, Settings};

mod app;
mod himawari;

fn main() -> iced::Result {
    env_logger::init();
    App::run(Settings::default())
}
