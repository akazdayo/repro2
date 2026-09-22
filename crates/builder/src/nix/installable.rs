#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Installable(String);

impl Installable {
    pub fn new(reference: &str, attribute: &str) -> Self {
        Self {
            0: format!("{reference}#{attribute}"),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InstallableError {
    ParseError,
}

impl TryFrom<String> for Installable {
    type Error = InstallableError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (reference, attribute) = value.split_once('#').ok_or(InstallableError::ParseError)?;

        if reference.is_empty() || attribute.is_empty() || attribute.contains('#') {
            return Err(InstallableError::ParseError);
        }

        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::Installable;

    #[test]
    fn parses_a_flake_reference_with_an_attribute() {
        let installable = Installable::try_from("nixpkgs#hello".to_owned()).unwrap();

        assert_eq!(installable, Installable::new("nixpkgs", "hello"));
    }

    #[test]
    fn rejects_an_installable_without_an_attribute_separator() {
        assert!(Installable::try_from("nixpkgs".to_owned()).is_err());
    }

    #[test]
    fn rejects_an_empty_reference_or_attribute() {
        assert!(Installable::try_from("#hello".to_owned()).is_err());
        assert!(Installable::try_from("nixpkgs#".to_owned()).is_err());
    }

    #[test]
    fn rejects_multiple_attribute_separators() {
        assert!(Installable::try_from("nixpkgs#hello#out".to_owned()).is_err());
    }
}
