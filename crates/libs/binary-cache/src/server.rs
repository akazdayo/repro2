use crate::{
    StorePathHash,
    narinfo::{NarInfoPath, NarInfoResponse},
};
use anyhow::Result;
use axum::http::StatusCode;
use axum::{
    Router,
    extract::{Path, State},
    routing::get,
};
use nix_narinfo::NarInfo;
use std::sync::Arc;

pub struct Cache<S, A> {
    source_addr: S,
    listen_addr: A,
}

pub trait CacheSource {
    fn lookup(&self, hash: &StorePathHash) -> impl Future<Output = Result<Option<NarInfo>>> + Send;
}

impl<S, A> Cache<S, A>
where
    S: CacheSource + Send + Sync + 'static,
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

    pub async fn run(self) -> std::io::Result<()> {
        let app = Router::new()
            .route("/nix-cache-info", get(nix_cache_info))
            .route("/{narinfo_path}", get(route_narinfo::<S>))
            // /narは上流キャッシュサーバーに任せることにした
            .with_state(Arc::new(self.source_addr));
        let listener = tokio::net::TcpListener::bind(self.listen_addr).await?;

        axum::serve(listener, app).await
    }
}

async fn route_narinfo<S: CacheSource>(
    State(source): State<Arc<S>>,
    Path(narinfo_path): Path<NarInfoPath>,
) -> Result<NarInfoResponse, StatusCode> {
    let response = source
        .lookup(narinfo_path.hash())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(NarInfoResponse(response))
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
    use super::*;

    struct Missing;

    impl CacheSource for Missing {
        async fn lookup(&self, _: &StorePathHash) -> Result<Option<NarInfo>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn returns_not_found_when_source_has_no_narinfo() {
        let path =
            NarInfoPath::try_from("0123456789abcdfghijklmnpqrsvwxyz.narinfo".to_owned()).unwrap();
        assert!(matches!(
            route_narinfo(State(Arc::new(Missing)), Path(path)).await,
            Err(StatusCode::NOT_FOUND)
        ));
    }
}
