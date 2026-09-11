use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use nix_derivation::{NixHash, StorePath};
use nix_narinfo::{Compression, NarInfo};
use serde::Deserialize;

use crate::db::narinfo::Model;

pub struct NarInfoResponse(pub NarInfo);

impl TryFrom<Model> for NarInfoResponse {
    type Error = anyhow::Error;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        let store_path = model.store_path.parse::<StorePath>()?;
        let nar_hash = model.nar_hash.parse::<NixHash>()?;
        let nar_size = model.nar_size.try_into()?;
        let info = NarInfo::builder(
            store_path,
            format!("nar/{}.nar", model.store_path_hash),
            nar_hash,
            nar_size,
        )
        .compression(Compression::None)
        .build()?;

        Ok(Self(info))
    }
}

impl IntoResponse for NarInfoResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/x-nix-narinfo")],
            self.0.to_canonical_bytes(),
        )
            .into_response()
    }
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct NarInfoPath(String);

impl TryFrom<String> for NarInfoPath {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let hash = value.strip_suffix(".narinfo").ok_or("not a narinfo path")?;

        const NIX32: &str = "0123456789abcdfghijklmnpqrsvwxyz";

        if hash.len() != 32 || !hash.chars().all(|c| NIX32.contains(c)) {
            return Err("invalid store hash");
        }

        Ok(Self(hash.to_owned()))
    }
}

impl NarInfoPath {
    pub fn hash(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::NarInfoPath;

    #[test]
    fn accepts_a_nix_store_hash_path() {
        let path =
            NarInfoPath::try_from("0123456789abcdfghijklmnpqrsvwxyz.narinfo".to_owned()).unwrap();

        assert_eq!(path.hash(), "0123456789abcdfghijklmnpqrsvwxyz");
    }

    #[test]
    fn rejects_an_invalid_nix_store_hash_path() {
        assert!(
            NarInfoPath::try_from("0123456789abcdefghijklmnopqrstuv.narinfo".to_owned()).is_err()
        );
    }
}
