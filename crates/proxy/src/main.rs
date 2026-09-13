mod cache;
mod db;
mod store_path_hash;
mod templates;

use anyhow::Result;
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use db::connection;
use reqwest::Client;
use sea_orm::DatabaseConnection;
use templates::narinfo::{NarInfoPath, NarInfoResponse};

use crate::{cache::get::CacheServer, db::narinfo::NarRecord};

#[derive(Clone)]
struct AppState {
    db: DatabaseConnection,
    http: Client,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        db: connection::connect().await.unwrap(),
        http: Client::new(),
    };

    let app = Router::new()
        .route("/nix-cache-info", get(nix_cache_info))
        .route("/{narinfo_path}", get(narinfo))
        // /narは上流キャッシュサーバーに任せることにした
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn narinfo(
    State(state): State<AppState>,
    Path(narinfo_path): Path<NarInfoPath>,
) -> Result<NarInfoResponse, StatusCode> {
    println!("narinfo request: {}", narinfo_path.hash());
    let Some(record) = db::narinfo::find(&state.db, narinfo_path.hash())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    else {
        // TODO: 将来的にこのあたり抽象化したい。
        return Err(StatusCode::NOT_FOUND);
    };

    let cache_server = CacheServer(
        "https://cache.nixos.org"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    let server_record = NarRecord::try_from(
        cache_server
            .fetch_narinfo(&state.http, &record.store_path_hash)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if record != server_record {
        return Err(StatusCode::NOT_FOUND);
    }

    NarInfoResponse::from_record(record).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn nix_cache_info() -> &'static str {
    "\
StoreDir: /nix/store
WantMassQuery: 0
Priority: 30
"
}
