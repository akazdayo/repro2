use anyhow::Result;
use axum::http::StatusCode;
use sea_orm::{ConnectionTrait, EntityTrait};

use crate::db::entities::nar_info::{self, Model};

pub async fn find(db: &impl ConnectionTrait, hash: &str) -> Result<Option<Model>, StatusCode> {
    let info = nar_info::Entity::find_by_id(hash)
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(info)
}
