use anyhow::Result;
use nix_derivation::{NixHash, StorePath};
use nix_narinfo::NarInfo;
use serde::{Deserialize, Serialize};

use crate::store_path_hash::StorePathHash;

#[derive(Clone, Deserialize, Serialize)]
pub struct RegistryNarRecord {
    pub store_path_hash: String,
    pub store_path: String,
    pub nar_hash: String,
    pub nar_size: i64,
    pub cache_url: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NarRecord {
    pub store_path_hash: StorePathHash,
    pub store_path: StorePath,
    pub nar_hash: NixHash,
    pub nar_size: u64,
    pub cache_url: String,
}

impl NarRecord {
    pub fn matches_nar(&self, other: &Self) -> bool {
        self.store_path_hash == other.store_path_hash
            && self.store_path == other.store_path
            && self.nar_hash == other.nar_hash
            && self.nar_size == other.nar_size
    }
}

impl TryFrom<RegistryNarRecord> for NarRecord {
    type Error = anyhow::Error;

    fn try_from(model: RegistryNarRecord) -> Result<Self, Self::Error> {
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
