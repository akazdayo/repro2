use std::fmt;

use nix_derivation::StorePath;
use thiserror::Error;

const NIX32: &str = "0123456789abcdfghijklmnpqrsvwxyz";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorePathHash(String);

#[derive(Debug, Error)]
#[error("invalid Nix store path hash")]
pub struct InvalidStorePathHash;

impl TryFrom<String> for StorePathHash {
    type Error = InvalidStorePathHash;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 32 || !value.chars().all(|c| NIX32.contains(c)) {
            return Err(InvalidStorePathHash);
        }

        Ok(Self(value))
    }
}

impl TryFrom<StorePath> for StorePathHash {
    type Error = InvalidStorePathHash;

    fn try_from(value: StorePath) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&StorePath> for StorePathHash {
    type Error = InvalidStorePathHash;

    fn try_from(value: &StorePath) -> Result<Self, Self::Error> {
        let basename = value.to_basename();
        let hash = basename.get(..32).ok_or(InvalidStorePathHash)?;

        Self::try_from(hash.to_owned())
    }
}

impl StorePathHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StorePathHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use nix_derivation::StorePath;

    use super::StorePathHash;

    #[test]
    fn accepts_a_nix_store_path_hash() {
        let hash = StorePathHash::try_from("0123456789abcdfghijklmnpqrsvwxyz".to_owned()).unwrap();

        assert_eq!(hash.as_str(), "0123456789abcdfghijklmnpqrsvwxyz");
    }

    #[test]
    fn rejects_a_non_nix_base32_hash() {
        assert!(StorePathHash::try_from("0123456789abcdefghijklmnopqrstuv".to_owned()).is_err());
    }

    #[test]
    fn extracts_the_hash_from_a_store_path() {
        let path: StorePath = "/nix/store/y1a49lg2ja68djssigz14lhdxvxcwbxa-example"
            .parse()
            .unwrap();

        let hash = StorePathHash::try_from(path.clone()).unwrap();

        assert_eq!(hash.as_str(), "y1a49lg2ja68djssigz14lhdxvxcwbxa");

        let hash = StorePathHash::try_from(&path).unwrap();

        assert_eq!(hash.as_str(), "y1a49lg2ja68djssigz14lhdxvxcwbxa");
    }
}
