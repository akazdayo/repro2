use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/nix-cache-info", get(nix_cache_info))
        .route("/{hash}.narinfo", get(narinfo))
        .route("/nar/{id}", get(nar));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn nix_cache_info() -> &'static str {
    "\
StoreDir: /nix/store
WantMassQuery: 0
Priority: 30
"
}

async fn narinfo(Path(hash): Path<String>) -> Response {
    // 仮: このhashだけcache hitにする
    if hash != "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" {
        return StatusCode::NOT_FOUND.into_response();
    }

    let body = format!(
        "\
StorePath: /nix/store/{hash}-hello
URL: nar/{hash}.nar
Compression: none
NarHash: sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=
NarSize: 1234
References:
"
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/x-nix-narinfo")],
        body,
    )
        .into_response()
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
