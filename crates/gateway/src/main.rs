mod cache;
mod nar_record;
mod routes;
mod store_path_hash;
mod templates;

use axum::{Router, routing::get};
use reqwest::Client;

#[derive(Clone)]
pub struct AppState {
    registry_url: String,
    http: Client,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        registry_url: std::env::var("REGISTRY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3001".to_owned()),
        http: Client::new(),
    };

    let app = Router::new()
        .route("/nix-cache-info", get(nix_cache_info))
        .route("/{narinfo_path}", get(routes::narinfo::narinfo))
        // /narは上流キャッシュサーバーに任せることにした
        .with_state(state);
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

#[cfg(test)]
mod tests {
    use crate::nar_record::RegistryNarRecord;
    use crate::routes::narinfo::{narinfo, select_report};
    use axum::{Json, Router, http::status::StatusCode, routing::get};
    use axum::{
        extract::{Path, State},
        response::IntoResponse,
    };
    use reqwest::Client;
    use templates::narinfo::NarInfoPath;

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

    async fn mock_registry(cache_url: String) -> String {
        let record = RegistryNarRecord {
            store_path_hash: STORE_PATH_HASH.to_owned(),
            store_path: STORE_PATH.to_owned(),
            nar_hash: NAR_HASH.to_owned(),
            nar_size: NAR_SIZE,
            cache_url,
        };
        mock_registry_reports(vec![record]).await
    }

    async fn mock_registry_reports(reports: Vec<RegistryNarRecord>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let registry_url = format!("http://{}", listener.local_addr().unwrap());
        let app = Router::new().route(
            &format!("/nar-info/{STORE_PATH_HASH}"),
            get(move || {
                let reports = reports.clone();
                async move { Json(reports) }
            }),
        );
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        registry_url
    }

    #[tokio::test]
    async fn returns_narinfo_when_registry_and_upstream_records_match() {
        let cache_url = mock_cache().await;
        let state = AppState {
            registry_url: mock_registry(cache_url.clone()).await,
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
            registry_url: mock_registry(mock_missing_cache().await).await,
            http: Client::new(),
        };
        let path = NarInfoPath::try_from(format!("{STORE_PATH_HASH}.narinfo")).unwrap();

        let result = narinfo(State(state), Path(path)).await;

        match result {
            Err(error) => assert_eq!(error.into_response().status(), StatusCode::NOT_FOUND),
            Ok(_) => panic!("expected the missing upstream narinfo to return an error"),
        }
    }

    #[tokio::test]
    async fn returns_narinfo_for_the_most_reported_build_result() {
        let cache_url = mock_cache().await;
        let record = RegistryNarRecord {
            store_path_hash: STORE_PATH_HASH.to_owned(),
            store_path: STORE_PATH.to_owned(),
            nar_hash: NAR_HASH.to_owned(),
            nar_size: NAR_SIZE,
            cache_url: cache_url.clone(),
        };
        let other = RegistryNarRecord {
            nar_size: NAR_SIZE + 1,
            ..record.clone()
        };
        let registry_url = mock_registry_reports(vec![other, record.clone(), record]).await;

        let state = AppState {
            registry_url,
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

        assert!(
            String::from_utf8(body.to_vec())
                .unwrap()
                .contains(&format!("URL: {cache_url}nar/archive.nar"))
        );
    }

    #[test]
    fn counts_matching_results_from_different_caches_together() {
        let record = RegistryNarRecord {
            store_path_hash: STORE_PATH_HASH.to_owned(),
            store_path: STORE_PATH.to_owned(),
            nar_hash: NAR_HASH.to_owned(),
            nar_size: NAR_SIZE,
            cache_url: "https://first.example/".to_owned(),
        };
        let matching = RegistryNarRecord {
            cache_url: "https://second.example/".to_owned(),
            ..record.clone()
        };
        let other = RegistryNarRecord {
            nar_size: NAR_SIZE + 1,
            ..record.clone()
        };

        let winner = select_report(vec![other, record, matching]).unwrap();

        assert_eq!(winner.cache_url, "https://first.example/");
    }

    #[tokio::test]
    async fn returns_conflict_when_results_have_equal_report_counts() {
        let record = RegistryNarRecord {
            store_path_hash: STORE_PATH_HASH.to_owned(),
            store_path: STORE_PATH.to_owned(),
            nar_hash: NAR_HASH.to_owned(),
            nar_size: NAR_SIZE,
            cache_url: "http://127.0.0.1:1/".to_owned(),
        };
        let other = RegistryNarRecord {
            nar_size: NAR_SIZE + 1,
            ..record.clone()
        };
        let state = AppState {
            registry_url: mock_registry_reports(vec![record, other]).await,
            http: Client::new(),
        };
        let path = NarInfoPath::try_from(format!("{STORE_PATH_HASH}.narinfo")).unwrap();
        let result = narinfo(State(state), Path(path)).await;

        match result {
            Err(error) => assert_eq!(error.into_response().status(), StatusCode::CONFLICT),
            Ok(_) => panic!("expected tied build results to be rejected"),
        }
    }
}
