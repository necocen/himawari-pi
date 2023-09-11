use std::{collections::HashMap, fs::File, io::Write};

use headless_chrome::Browser;
use reqwest::{
    blocking::Client,
    header::{COOKIE, USER_AGENT},
};

fn main() -> anyhow::Result<()> {
    env_logger::init();

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
    let dl_path = &attrs[7];
    // トークンを取得
    let attrs = tab
        .wait_for_element("input#fixedToken")?
        .get_attributes()?
        .expect("expected attributes");
    let token = &attrs[7];
    let cookies = tab.get_cookies()?;
    // CAKEPHPのセッションID(?)を取得
    let cakephp_cookie = cookies
        .into_iter()
        .find(|cookie| cookie.name == *"CAKEPHP")
        .expect("expected CAKEPHP cookie")
        .value;
    // TODO: 更新時刻を取得
    log::debug!("CAKEPHP Cookie: {cakephp_cookie}");
    log::debug!("Token: {token}");
    log::debug!("Download Path: {dl_path}");

    let client = Client::new();
    let mut params = HashMap::new();
    params.insert("_method", "POST");
    params.insert("data[FileSearch][is_compress]", "false");
    params.insert("data[FileSearch][fixedToken]", token);
    params.insert("data[FileSearch][hashUrl]", "bDw2maKV");
    params.insert("action", "dir_download_dl");
    params.insert("filelist[0]", dl_path);
    params.insert("dl_path", dl_path);

    let req = client
        .post("https://sc-nc-web.nict.go.jp/wsdb_osndisk/fileSearch/download")
        .form(&params)
        .header(COOKIE, format!("CAKEPHP={cakephp_cookie}"))
        .header(USER_AGENT, version_info.user_agent) // NOTE: これが最初のトークン取得時のものと一致していないといけない
        .build()?;
    let mut resp = client.execute(req)?;

    // FIXME: ファイル名
    if resp.status().is_success() {
        let mut buf: Vec<u8> = vec![];
        resp.copy_to(&mut buf)?;
        let mut file = File::create("downloaded_file.png")?;
        file.write_all(&buf)?;
    }

    Ok(())
}
