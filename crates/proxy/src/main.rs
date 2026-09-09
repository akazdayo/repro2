mod templates;

use anyhow::Result;
use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use templates::{compression::Compression, narinfo::NarInfo, narinfo::NarInfoPath};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/nix-cache-info", get(nix_cache_info))
        .route("/{narinfo_path}", get(narinfo))
        .route("/nar/{id}", get(nar));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn narinfo(Path(narinfo_path): Path<NarInfoPath>) -> Result<NarInfo, StatusCode> {
    // TODO: ちゃんとキャッシュを検索するようにする
    if narinfo_path.hash() != "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(NarInfo {
        store_path: "/nix/store/...".into(),
        url: "nar/foo.nar".into(),
        compression: Compression::None,
        nar_hash: "sha256:...".into(),
        nar_size: 1234,
        references: vec![],
    })
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
