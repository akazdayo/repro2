mod cache;
mod routes;
mod store_path_hash;
mod templates;

use anyhow::Result;
use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use cache::http::HttpBinaryCache;
use nix_narinfo::NarInfo;
use reqwest::{Client, StatusCode};
use templates::narinfo::{NarInfoPath, NarInfoResponse};

use crate::store_path_hash::StorePathHash;

pub struct Cache<S, A> {
    source_addr: S,
    listen_addr: A,
}

pub trait CacheSource {
    // サーバーに問い合わせてくれる関数のイメージ
    async fn lookup(&self, hash: &StorePathHash) -> Result<Option<NarInfo>>;
}

#[derive(Clone)]
struct AppState {
    http: Client,
    source_addr: String,
}

impl<S, A> Cache<S, A>
where
    S: CacheSource + Send + Sync + 'static + std::fmt::Display,
    A: tokio::net::ToSocketAddrs,
{
    pub fn source(mut self, source_addr: S) -> Self {
        self.source_addr = source_addr;
        self
    }

    pub fn listen(mut self, listen_addr: A) -> Self {
        self.listen_addr = listen_addr;
        self
    }

    pub async fn run(&self) {
        let state = AppState {
            http: Client::new(),
            source_addr: self.source_addr.to_string(),
        };
        let app = Router::new()
            .route("/nix-cache-info", get(nix_cache_info))
            .route("/{narinfo_path}", get(route_narinfo))
            // /narは上流キャッシュサーバーに任せることにした
            .with_state(state);
        let listener = tokio::net::TcpListener::bind(&self.listen_addr)
            .await
            .unwrap();

        axum::serve(listener, app).await.unwrap();
    }

}

async fn route_narinfo(
    State(state): State<AppState>,
    Path(narinfo_path): Path<NarInfoPath>,
) -> Result<NarInfoResponse, StatusCode> {
    // TODO: policyを任意で設定できるようにした方が良さそう
    // let reports = response
    //    .error_for_status()?
    //    .json::<Vec<RegistryNarRecord>>()
    //    .await?;
    // let record = select_report(reports)?;

    let cache_server = HttpBinaryCache::try_from(state.source_addr)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let response = cache_server
        .fetch_narinfo(&state.http, narinfo_path.hash())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(NarInfoResponse(response))
}

async fn nix_cache_info() -> &'static str {
    "\
StoreDir: /nix/store
WantMassQuery: 0
Priority: 30
"
}
