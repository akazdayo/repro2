mod db;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::Serialize;

use db::entities::{nar_info, prelude::NarInfo};

#[derive(Serialize)]
struct NarRecord {
    store_path_hash: String,
    store_path: String,
    nar_hash: String,
    nar_size: i64,
    cache_url: String,
}

impl From<nar_info::Model> for NarRecord {
    fn from(model: nar_info::Model) -> Self {
        Self {
            store_path_hash: model.store_path_hash,
            store_path: model.store_path,
            nar_hash: model.nar_hash,
            nar_size: model.nar_size,
            cache_url: model.cache_url,
        }
    }
}

#[tokio::main]
async fn main() {
    let db = db::connection::connect().await.unwrap();
    let app = Router::new()
        .route("/nar-info/{hash}", get(nar_info))
        .with_state(db);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn nar_info(
    State(db): State<DatabaseConnection>,
    Path(hash): Path<String>,
) -> Result<Json<NarRecord>, StatusCode> {
    let model = NarInfo::find_by_id(hash)
        .one(&db)
        .await
        .map_err(|error| {
            eprintln!("registry error: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(model.into()))
}
