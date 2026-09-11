use nix_narinfo::NarInfo;
use thiserror::Error;
use url::Url;

use crate::store_path_hash::StorePathHash;

pub struct CacheServerUrl(Url);

#[derive(Debug, Error)]
pub enum FetchNarInfoError {
    #[error("failed to build narinfo URL")]
    Url(#[from] url::ParseError),
    #[error("failed to fetch narinfo")]
    Request(#[from] reqwest::Error),
    #[error("failed to parse narinfo")]
    Parse(#[from] nix_narinfo::ParseError),
}

impl TryFrom<String> for CacheServerUrl {
    type Error = url::ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for CacheServerUrl {
    type Error = url::ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut url = Url::parse(value)?;

        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }

        Ok(Self(url))
    }
}

impl CacheServerUrl {
    fn narinfo_url(&self, hash: &StorePathHash) -> Result<Url, url::ParseError> {
        self.0.join(&format!("{hash}.narinfo"))
    }

    pub async fn fetch_narinfo(
        &self,
        http: &reqwest::Client,
        hash: &StorePathHash,
    ) -> Result<NarInfo, FetchNarInfoError> {
        let response = http
            .get(self.narinfo_url(hash)?)
            .send()
            .await?
            .error_for_status()?;
        let body = response.bytes().await?;

        Ok(NarInfo::parse_in(&Default::default(), &body)?)
    }
}

#[cfg(test)]
mod tests {
    use super::CacheServerUrl;
    use crate::store_path_hash::StorePathHash;

    #[test]
    fn builds_a_narinfo_url_below_the_cache_base_path() {
        let cache = CacheServerUrl::try_from("https://cache.example.org/nix-cache").unwrap();
        let hash = StorePathHash::try_from("0123456789abcdfghijklmnpqrsvwxyz".to_owned()).unwrap();
        let url = cache.narinfo_url(&hash).unwrap();

        assert_eq!(
            url.as_str(),
            "https://cache.example.org/nix-cache/0123456789abcdfghijklmnpqrsvwxyz.narinfo"
        );
    }

    #[tokio::test]
    async fn fetch_cache() {
        use reqwest::Client;

        // curl -fsSL https://cache.nixos.org/y1a49lg2ja68djssigz14lhdxvxcwbxa.narinfo
        let cache = CacheServerUrl::try_from("https://cache.nixos.org").unwrap();
        let hash = StorePathHash::try_from("y1a49lg2ja68djssigz14lhdxvxcwbxa".to_owned()).unwrap();
        let http = Client::new();

        let narinfo = cache.fetch_narinfo(&http, &hash).await.unwrap();
        println!("{}", narinfo.nar_hash().to_sri_string());

        assert_eq!(
            narinfo.nar_hash().to_sri_string(),
            "sha256-rS0qEqEXArxnAdzxNkNv+4PaHxXcQ/JdN0Kjkuq6XSY="
        )
    }
}
