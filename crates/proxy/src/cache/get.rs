use crate::templates::narinfo;
use anyhow::Result;

// url::UrlのWrapper
// それぞれの用途に対して型を変化させるための中間型として使うことを想定
pub struct CacheServerUrl(url::Url);

// Pathにnarinfoを含んでいることを示すための型
// Validであることを表現したかった
pub struct NarInfoUrl(url::Url);

impl TryFrom<String> for CacheServerUrl {
    type Error = url::ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut url = url::Url::parse(&value)?;

        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }

        Ok(Self(url))
    }
}

impl CacheServerUrl {
    // NarInfoUrlを引数にしたとき、validationは既に済んでいる状態にしたいからこの実装になった
    pub fn to_narinfo_url(&self, hash: &str) -> Result<NarInfoUrl, url::ParseError> {
        Ok(NarInfoUrl(self.0.join(&format!("{hash}.narinfo"))?))
    }
}

impl NarInfoUrl {
    pub fn as_url(&self) -> &url::Url {
        &self.0
    }
}

// TODO: DBに保存されているメタデータを元に実データを該当するサーバーに問い合わせる。
async fn get_narinfo(http: &reqwest::Client, url: NarInfoUrl) -> Result<narinfo::NarInfo> {
    let response = http
        .get(url.as_url().clone())
        .send()
        .await?
        .error_for_status()?;
    todo!()
}
