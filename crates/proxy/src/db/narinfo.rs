use anyhow::Result;
use nix_derivation::{NixHash, StorePath};
use nix_narinfo::NarInfo;
use sea_orm::{ConnectionTrait, EntityTrait};

use crate::db::entities::nar_info;
use crate::store_path_hash::StorePathHash;

#[derive(Debug, PartialEq, Eq)]
pub struct NarRecord {
    pub store_path_hash: StorePathHash,
    pub store_path: StorePath,
    pub nar_hash: NixHash,
    pub nar_size: u64,
    pub cache_url: String,
}

impl TryFrom<nar_info::Model> for NarRecord {
    type Error = anyhow::Error;

    fn try_from(model: nar_info::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            store_path_hash: StorePathHash::try_from(model.store_path_hash)?,
            store_path: model.store_path.parse()?,
            nar_hash: model.nar_hash.parse()?,
            nar_size: model.nar_size.try_into()?,
            cache_url: model.cache_url,
        })
    }
}

impl TryFrom<NarInfo> for NarRecord {
    type Error = anyhow::Error;

    fn try_from(value: NarInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            store_path_hash: StorePathHash::try_from(value.store_path())?.clone(),
            store_path: value.store_path().clone(),
            nar_hash: value.nar_hash().clone(),
            nar_size: value.nar_size().clone(),
            cache_url: value.url().to_owned(),
        })
    }
}

pub async fn find(db: &impl ConnectionTrait, hash: &StorePathHash) -> Result<Option<NarRecord>> {
    let info = nar_info::Entity::find_by_id(hash.as_str()).one(db).await?;

    info.map(NarRecord::try_from).transpose()
}
