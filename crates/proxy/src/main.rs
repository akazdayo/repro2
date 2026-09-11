mod cache;
mod db;
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
        .route("/nar/{id}", get(nar))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn narinfo(
    State(state): State<AppState>,
    Path(narinfo_path): Path<NarInfoPath>,
) -> Result<NarInfoResponse, StatusCode> {
    println!("narinfo request: {}", narinfo_path.hash());
    let Some(model) = db::narinfo::find(&state.db, narinfo_path.hash()).await? else {
        return Err(StatusCode::NOT_FOUND);
    };

    NarInfoResponse::try_from(model).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn nix_cache_info() -> &'static str {
    "\
StoreDir: /nix/store
WantMassQuery: 0
Priority: 30
"
}

async fn nar(Path(id): Path<String>) -> Response {
    let expected = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.nar";

    if id != expected {
        return StatusCode::NOT_FOUND.into_response();
    }

    // 本来ここは実際のNARデータ
    let nar: Vec<u8> = vec![];

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/x-nix-nar")],
        nar,
    )
        .into_response()
}
