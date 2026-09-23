mod cache;
mod db;
mod store_path_hash;
mod templates;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use db::connection;
use reqwest::Client;
use sea_orm::DatabaseConnection;
use templates::narinfo::{NarInfoPath, NarInfoResponse};
use thiserror::Error;

use crate::{
    cache::get::{CacheServer, FetchNarInfoError},
    db::narinfo::NarRecord,
};

#[derive(Clone)]
struct AppState {
    db: DatabaseConnection,
    http: Client,
}

#[derive(Debug, Error)]
enum NarInfoError {
    #[error("narinfo not found")]
    NotFound,
    #[error("invalid cache URL")]
    Url(#[from] url::ParseError),
    #[error("failed to fetch upstream narinfo")]
    Upstream(#[from] FetchNarInfoError),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for NarInfoError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Upstream(FetchNarInfoError::Request(error))
                if error.status() == Some(StatusCode::NOT_FOUND) =>
            {
                StatusCode::NOT_FOUND
            }
            Self::Upstream(_) => StatusCode::BAD_GATEWAY,
            Self::Url(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        if status.is_server_error() {
            eprintln!("narinfo error: {self:#}");
        }

        status.into_response()
    }
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
) -> Result<NarInfoResponse, NarInfoError> {
    println!("narinfo request: {}", narinfo_path.hash());
    let record = db::narinfo::find(&state.db, narinfo_path.hash())
        .await?
        .ok_or(NarInfoError::NotFound)?;

    let cache_server = CacheServer::try_from(record.cache_url.as_str())?;
    let server_info = cache_server
        .fetch_narinfo(&state.http, &record.store_path_hash)
        .await?;
    let server_record = NarRecord::try_from(server_info.clone())?;
    println!("{:?}", cache_server.url());
    let nar_url = cache_server.url().join(&server_record.cache_url)?;
    println!("{:?}", record);
    println!("{:?}", server_record);
    if !record.matches_nar(&server_record) {
        return Err(NarInfoError::NotFound);
    }

    Ok(NarInfoResponse::from_upstream(
        server_info,
        nar_url.to_string(),
    )?)
}

async fn nix_cache_info() -> &'static str {
    "\
StoreDir: /nix/store
WantMassQuery: 0
Priority: 30
"
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::{Router, routing::get};
    use sea_orm::{DatabaseBackend, DatabaseConnection, MockDatabase, Value, prelude::DateTime};

    use super::*;

    const STORE_PATH_HASH: &str = "y1a49lg2ja68djssigz14lhdxvxcwbxa";
    const STORE_PATH: &str = "/nix/store/y1a49lg2ja68djssigz14lhdxvxcwbxa-hello-2.12.3";
    const NAR_HASH: &str = "sha256-rS0qEqEXArxnAdzxNkNv+4PaHxXcQ/JdN0Kjkuq6XSY=";
    const NAR_SIZE: i64 = 226640;

    async fn mock_cache() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cache_url = format!("http://{}/", listener.local_addr().unwrap());
        let body = format!(
            "StorePath: {STORE_PATH}\nURL: nar/archive.nar\nCompression: none\nNarHash: {NAR_HASH}\nNarSize: {NAR_SIZE}\n"
        );
        let app = Router::new().route(
            &format!("/{STORE_PATH_HASH}.narinfo"),
            get(move || async move { body }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        cache_url
    }

    async fn mock_missing_cache() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let cache_url = format!("http://{}/", listener.local_addr().unwrap());
        let app = Router::new().fallback(|| async { StatusCode::NOT_FOUND });
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        cache_url
    }

    fn mock_db(cache_url: String) -> DatabaseConnection {
        MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results([[BTreeMap::<&str, Value>::from([
                ("store_path_hash", STORE_PATH_HASH.into()),
                ("store_path", STORE_PATH.into()),
                ("nar_hash", NAR_HASH.into()),
                ("nar_size", NAR_SIZE.into()),
                ("cache_url", cache_url.into()),
                ("status", "pending".into()),
                (
                    "updated_at",
                    DateTime::parse_from_str("2026-09-14 00:00:00", "%F %T")
                        .unwrap()
                        .into(),
                ),
            ])]])
            .into_connection()
    }

    #[tokio::test]
    async fn returns_narinfo_when_db_and_upstream_records_match() {
        let cache_url = mock_cache().await;
        let state = AppState {
            db: mock_db(cache_url.clone()),
            http: Client::new(),
        };
        let path = NarInfoPath::try_from(format!("{STORE_PATH_HASH}.narinfo")).unwrap();

        let response = narinfo(State(state), Path(path))
            .await
            .unwrap()
            .into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains(&format!("URL: {cache_url}nar/archive.nar")));
    }

    #[tokio::test]
    async fn returns_not_found_when_upstream_narinfo_is_missing() {
        let state = AppState {
            db: mock_db(mock_missing_cache().await),
            http: Client::new(),
        };
        let path = NarInfoPath::try_from(format!("{STORE_PATH_HASH}.narinfo")).unwrap();

        let result = narinfo(State(state), Path(path)).await;

        match result {
            Err(error) => assert_eq!(error.into_response().status(), StatusCode::NOT_FOUND),
            Ok(_) => panic!("expected the missing upstream narinfo to return an error"),
        }
    }
}
