use anyhow::Result;
use axum::http::StatusCode;
use sea_orm::{ConnectionTrait, EntityTrait};

use crate::db::entities::nar_info;
pub use crate::db::entities::nar_info::Model;
use crate::store_path_hash::StorePathHash;

pub async fn find(
    db: &impl ConnectionTrait,
    hash: &StorePathHash,
) -> Result<Option<Model>, StatusCode> {
    let info = nar_info::Entity::find_by_id(hash.as_str())
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(info)
}
