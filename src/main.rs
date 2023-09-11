use std::collections::HashMap;

use headless_chrome::Browser;
use reqwest::{
    header::{COOKIE, USER_AGENT},
    Client,
};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let info = fetch_download_info().await?;

    let client = Client::new();
    let mut params = HashMap::new();
    params.insert("_method", "POST");
    params.insert("data[FileSearch][is_compress]", "false");
    params.insert("data[FileSearch][fixedToken]", &info.token);
    params.insert("data[FileSearch][hashUrl]", "bDw2maKV");
    params.insert("action", "dir_download_dl");
    params.insert("filelist[0]", &info.dl_path);
    params.insert("dl_path", &info.dl_path);

    let mut resp = client
        .post("https://sc-nc-web.nict.go.jp/wsdb_osndisk/fileSearch/download")
        .form(&params)
        .header(COOKIE, format!("CAKEPHP={}", info.cakephp_cookie))
        .header(USER_AGENT, info.user_agent) // NOTE: これが最初のトークン取得時のものと一致していないといけない
        .send()
        .await?;

    // FIXME: ファイル名
    if resp.status().is_success() {
        let mut file = File::create("downloaded_file.png").await?;
        while let Some(chunk) = resp.chunk().await? {
            file.write_all(&chunk).await?;
        }
    }

    Ok(())
}

struct DownloadInfo {
    cakephp_cookie: String,
    token: String,
    dl_path: String,
    user_agent: String,
}

async fn fetch_download_info() -> anyhow::Result<DownloadInfo> {
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
        // TODO: 更新時刻を取得
        log::info!("Fetched download info");
        log::debug!("CAKEPHP Cookie: {cakephp_cookie}");
        log::debug!("Token: {token}");
        log::debug!("Download Path: {dl_path}");

        Ok(DownloadInfo {
            cakephp_cookie,
            token,
            dl_path,
            user_agent: version_info.user_agent,
        })
    });
    task.await?
}
