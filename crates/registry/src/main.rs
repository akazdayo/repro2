mod db;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Serialize;

use db::entities::{build_reports, prelude::BuildReports};

#[derive(Serialize)]
struct NarRecord {
    id: i64,
    drv_path: Option<String>,
    output_name: Option<String>,
    store_path_hash: String,
    store_path: String,
    nar_hash: String,
    nar_size: i64,
    cache_url: String,
}

impl From<build_reports::Model> for NarRecord {
    fn from(model: build_reports::Model) -> Self {
        Self {
            id: model.id,
            drv_path: model.drv_path,
            output_name: model.output_name,
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
) -> Result<Json<Vec<NarRecord>>, StatusCode> {
    let models = BuildReports::find()
        .filter(build_reports::Column::StorePathHash.eq(hash))
        .all(&db)
        .await
        .map_err(|error| {
            eprintln!("registry error: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if models.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(models.into_iter().map(Into::into).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{ConnectionTrait, Database};

    #[tokio::test]
    async fn returns_all_reports_for_the_same_store_path() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO build_reports (drv_path, output_name, store_path_hash, store_path, nar_hash, nar_size, cache_url) VALUES \
             ('/nix/store/example.drv', 'out', 'abc', '/nix/store/abc-example', 'sha256-first', 10, 'https://one.example/'), \
             ('/nix/store/example.drv', 'out', 'abc', '/nix/store/abc-example', 'sha256-second', 10, 'https://two.example/')",
        )
        .await
        .unwrap();

        let Json(reports) = nar_info(State(db), Path("abc".to_owned())).await.unwrap();

        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].nar_hash, "sha256-first");
        assert_eq!(reports[1].nar_hash, "sha256-second");
    }
}
