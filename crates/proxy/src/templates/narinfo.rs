use super::compression::Compression;
use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

pub struct NarInfo {
    pub store_path: String,
    pub url: String,
    pub compression: Compression,
    pub nar_hash: String,
    pub nar_size: u64,
    pub references: Vec<String>,
}

impl IntoResponse for NarInfo {
    fn into_response(self) -> Response {
        let body = format!(
            "\
StorePath: {}
URL: {}
Compression: {}
NarHash: {}
NarSize: {}
References: {}
",
            self.store_path,
            self.url,
            self.compression,
            self.nar_hash,
            self.nar_size,
            self.references.join(" "),
        );

        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/x-nix-narinfo")],
            body,
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
