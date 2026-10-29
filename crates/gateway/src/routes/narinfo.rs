use crate::{
    AppState,
    cache::get::{CacheServer, FetchNarInfoError},
    nar_record::{NarRecord, RegistryNarRecord},
    templates::narinfo::{NarInfoPath, NarInfoResponse},
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NarInfoError {
    #[error("narinfo not found")]
    NotFound,
    #[error("multiple build results have the same number of reports")]
    Ambiguous,
    #[error("invalid cache URL")]
    Url(#[from] url::ParseError),
    #[error("failed to fetch upstream narinfo")]
    Upstream(#[from] FetchNarInfoError),
    #[error("failed to fetch registry record")]
    Registry(#[from] reqwest::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for NarInfoError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Ambiguous => StatusCode::CONFLICT,
            Self::Upstream(FetchNarInfoError::Request(error))
                if error.status() == Some(StatusCode::NOT_FOUND) =>
            {
                StatusCode::NOT_FOUND
            }
            Self::Upstream(_) | Self::Registry(_) => StatusCode::BAD_GATEWAY,
            Self::Url(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        if status.is_server_error() {
            eprintln!("narinfo error: {self:#}");
        }

        status.into_response()
    }
}

// Temporary policy: count reports for each distinct build result, regardless of cache URL.
pub(crate) fn select_report(reports: Vec<RegistryNarRecord>) -> Result<NarRecord, NarInfoError> {
    let mut records = reports
        .into_iter()
        .map(NarRecord::try_from)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut winner = None;
    let mut max_votes = 0;
    let mut tied = false;

    for (index, record) in records.iter().enumerate() {
        let votes = records
            .iter()
            .filter(|other| record.matches_nar(other))
            .count();
        if votes > max_votes {
            winner = Some(index);
            max_votes = votes;
            tied = false;
        } else if votes == max_votes
            && winner.is_some_and(|winner| !record.matches_nar(&records[winner]))
        {
            tied = true;
        }
    }

    if tied {
        return Err(NarInfoError::Ambiguous);
    }
    Ok(records.swap_remove(winner.ok_or(NarInfoError::NotFound)?))
}

pub async fn narinfo(
    State(state): State<AppState>,
    Path(narinfo_path): Path<NarInfoPath>,
) -> Result<NarInfoResponse, NarInfoError> {
    let response = state
        .http
        .get(format!(
            "{}/nar-info/{}",
            state.registry_url.trim_end_matches('/'),
            narinfo_path.hash()
        ))
        .send()
        .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Err(NarInfoError::NotFound);
    }
    let reports = response
        .error_for_status()?
        .json::<Vec<RegistryNarRecord>>()
        .await?;
    let record = select_report(reports)?;

    let cache_server = CacheServer::try_from(record.cache_url.as_str())?;
    let server_info = cache_server
        .fetch_narinfo(&state.http, &record.store_path_hash)
        .await?;
    let server_record = NarRecord::try_from(server_info.clone())?;
    let nar_url = cache_server.url().join(&server_record.cache_url)?;
    if !record.matches_nar(&server_record) {
        return Err(NarInfoError::NotFound);
    }

    Ok(NarInfoResponse::from_upstream(
        server_info,
        nar_url.to_string(),
    )?)
}
