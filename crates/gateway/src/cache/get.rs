use nix_narinfo::NarInfo;
use thiserror::Error;
use url::Url;

use crate::store_path_hash::StorePathHash;

pub struct CacheServer(pub Url);

#[derive(Debug, Error)]
pub enum FetchNarInfoError {
    #[error("failed to build narinfo URL")]
    Url(#[from] url::ParseError),
    #[error("failed to fetch narinfo")]
    Request(#[from] reqwest::Error),
    #[error("failed to parse narinfo")]
    Parse(#[from] nix_narinfo::ParseError),
}

impl TryFrom<String> for CacheServer {
    type Error = url::ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&str> for CacheServer {
    type Error = url::ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut url = Url::parse(value)?;

        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }

        Ok(Self(url))
    }
}

impl CacheServer {
    pub fn url(&self) -> &Url {
        &self.0
    }

    fn narinfo_url(&self, hash: &StorePathHash) -> Result<Url, url::ParseError> {
        self.url().join(&format!("{hash}.narinfo"))
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
    use super::CacheServer;
    use crate::store_path_hash::StorePathHash;

    #[test]
    fn builds_a_narinfo_url_below_the_cache_base_path() {
        let cache = CacheServer::try_from("https://cache.example.org/nix-cache").unwrap();
        let hash = StorePathHash::try_from("0123456789abcdfghijklmnpqrsvwxyz".to_owned()).unwrap();
        let url = cache.narinfo_url(&hash).unwrap();

        assert_eq!(
            url.as_str(),
            "https://cache.example.org/nix-cache/0123456789abcdfghijklmnpqrsvwxyz.narinfo"
        );
    }

    #[tokio::test]
    async fn fetches_narinfo_from_cache() {
        use axum::{Router, routing::get};
        use reqwest::Client;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cache_url = format!("http://{}/", listener.local_addr().unwrap());
        let app = Router::new().route(
            "/y1a49lg2ja68djssigz14lhdxvxcwbxa.narinfo",
            get(|| async {
                "StorePath: /nix/store/y1a49lg2ja68djssigz14lhdxvxcwbxa-hello-2.12.3\n\
                 URL: nar/archive.nar\n\
                 Compression: none\n\
                 NarHash: sha256-rS0qEqEXArxnAdzxNkNv+4PaHxXcQ/JdN0Kjkuq6XSY=\n\
                 NarSize: 226640\n"
            }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let cache = CacheServer::try_from(cache_url).unwrap();
        let hash = StorePathHash::try_from("y1a49lg2ja68djssigz14lhdxvxcwbxa".to_owned()).unwrap();
        let http = Client::new();

        let narinfo = cache.fetch_narinfo(&http, &hash).await.unwrap();

        assert_eq!(
            narinfo.nar_hash().to_sri_string(),
            "sha256-rS0qEqEXArxnAdzxNkNv+4PaHxXcQ/JdN0Kjkuq6XSY="
        )
    }
}
