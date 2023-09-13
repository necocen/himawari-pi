use std::{
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Asia::Tokyo;
use headless_chrome::Browser;
use image::imageops::FilterType;
use reqwest::{
    header::{COOKIE, USER_AGENT},
    Client,
};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct DownloadInfo {
    cakephp_cookie: String,
    token: String,
    dl_path: String,
    user_agent: String,
    timestamp: DateTime<Utc>,
}

pub async fn fetch_download_info() -> anyhow::Result<DownloadInfo> {
    let task = tokio::task::spawn_blocking(|| -> anyhow::Result<DownloadInfo> {
        let browser = Browser::default()?;
        let version_info = browser.get_version()?;
        let tab = browser.new_tab()?;
        tab.navigate_to("https://sc-nc-web.nict.go.jp/wsdb_osndisk/shareDirDownload/bDw2maKV")?;
        // 検索の完了を待つ
        _ = tab.wait_for_element("div#search_btn.enabled")?;

        // FIXME: attributesの順番が変わると壊れるのでちゃんとする
        // 最新のファイル名を取得
        let attrs = tab
            .wait_for_element("table#data_im_table tbody tr:first-child")?
            .get_attributes()?
            .expect("expected attributes");
        let dl_path = attrs[7].clone();
        // 観測日時を取得
        let timestamp = NaiveDateTime::parse_from_str(&attrs[9], "%Y/%m/%d %H:%M:%S")?
            .and_local_timezone(Tokyo)
            .single()
            .with_context(|| "failed to apply Asia/Tokyo timezone to given time")?
            .with_timezone(&Utc);
        // トークンを取得
        let attrs = tab
            .wait_for_element("input#fixedToken")?
            .get_attributes()?
            .expect("expected attributes");
        let token = attrs[7].clone();
        let cookies = tab.get_cookies()?;
        // CAKEPHPのセッションID(?)を取得
        let cakephp_cookie = cookies
            .into_iter()
            .find(|cookie| cookie.name == *"CAKEPHP")
            .expect("expected CAKEPHP cookie")
            .value;
        log::info!("Fetched download info (timestamp: {timestamp:?})");
        log::debug!("CAKEPHP Cookie: {cakephp_cookie}");
        log::debug!("Token: {token}");
        log::debug!("Download Path: {dl_path}");

        Ok(DownloadInfo {
            cakephp_cookie,
            token,
            dl_path,
            user_agent: version_info.user_agent,
            timestamp,
        })
    });
    task.await?
}

pub async fn get_image(download_info: &DownloadInfo) -> anyhow::Result<PathBuf> {
    let dir = "./images";
    // まずファイルがあるかを調べ、あれば返す
    let image_path = Path::new(&format!(
        "{dir}/{}.png",
        download_info.timestamp.format("%Y%m%d%H%M%S")
    ))
    .to_path_buf();
    if image_path.exists() {
        log::info!("Image already exists: {}", image_path.to_str().unwrap());
        return Ok(image_path);
    }

    // なければ取得する
    let client = Client::new();
    let mut params = HashMap::new();
    params.insert("_method", "POST");
    params.insert("data[FileSearch][is_compress]", "false");
    params.insert("data[FileSearch][fixedToken]", &download_info.token);
    params.insert("data[FileSearch][hashUrl]", "bDw2maKV");
    params.insert("action", "dir_download_dl");
    params.insert("filelist[0]", &download_info.dl_path);
    params.insert("dl_path", &download_info.dl_path);

    let resp = client
        .post("https://sc-nc-web.nict.go.jp/wsdb_osndisk/fileSearch/download")
        .form(&params)
        .header(COOKIE, format!("CAKEPHP={}", download_info.cakephp_cookie))
        .header(USER_AGENT, &download_info.user_agent) // NOTE: これが最初のトークン取得時のものと一致していないといけない
        .send()
        .await?
        .error_for_status()?;

    let mut buffer = Cursor::new(Vec::new());
    let bytes = resp.bytes().await?;
    image::load_from_memory(&bytes)?
        .resize(1080, 1080, FilterType::Nearest)
        .write_to(&mut buffer, image::ImageOutputFormat::Png)?;
    if fs::metadata(dir).await.is_err() {
        fs::create_dir(dir).await?;
    }
    let mut file = fs::File::create(&image_path).await?;
    file.write_all(buffer.get_ref()).await?;

    log::info!("Image downloaded: {}", image_path.to_str().unwrap());
    Ok(image_path)
}
