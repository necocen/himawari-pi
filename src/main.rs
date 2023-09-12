mod himawari;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let info = himawari::fetch_download_info().await?;
    himawari::get_image(&info).await?;
    log::info!("Done");

    Ok(())
}
