use anyhow::Context as _;
use chrono::{NaiveDateTime, Utc};
use chrono_tz::Asia::Tokyo;
use headless_chrome::Browser;

use super::DownloadInfo;

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
