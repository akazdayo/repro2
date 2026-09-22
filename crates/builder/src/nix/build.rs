use std::{collections::BTreeMap, path::PathBuf, process::Command};

use super::installable::Installable;
use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    installable: Installable, // like a build target
    rebuild: bool,
    substitute: bool,
    store: Option<PathBuf>,
}

impl Build {
    pub fn new(installable: Installable) -> Self {
        let path = PathBuf::from("/tmp/repro2-build-store/");

        Self {
            installable,
            rebuild: false,
            substitute: true,
            store: Some(path),
        }
    }

    pub fn rebuild(mut self, rebuild: bool) -> Self {
        self.rebuild = rebuild;
        self
    }

    pub fn substitute(mut self, substitute: bool) -> Self {
        self.substitute = substitute;
        self
    }

    pub fn store(mut self, store: Option<PathBuf>) -> Self {
        self.store = store;
        self
    }

    /// Runs `nix build` without creating a result symlink and returns its JSON result.
    ///
    /// Unless `self.store` is set, Nix inherits the caller's store and daemon settings.
    pub fn run(&self) -> Result<Vec<BuildOutput>> {
        let output = Command::new("nix")
            .args(self.args())
            .output()
            .context("failed to start nix build")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "nix build failed with exit status {}: {}",
                output.status,
                stderr.trim()
            );
        }

        serde_json::from_slice(&output.stdout).context("failed to parse JSON from nix build")
    }

    fn args(&self) -> Vec<String> {
        let mut args = vec![String::from("build")];
        if let Some(store) = &self.store {
            args.extend([
                String::from("--store"),
                store.to_string_lossy().into_owned(),
            ]);
        }
        if self.rebuild {
            args.push(String::from("--rebuild"));
        }
        args.extend([
            String::from("--option"),
            String::from("substitute"),
            String::from(if self.substitute { "true" } else { "false" }),
            String::from("--no-link"),
            String::from("--json"),
            self.installable.as_str().to_owned(),
        ]);
        args
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOutput {
    pub drv_path: Option<PathBuf>,
    pub outputs: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_a_normal_build_command_by_default() {
        let build = Build::new(Installable::try_from("nixpkgs#hello".to_owned()).unwrap());

        assert_eq!(
            build.args(),
            [
                "build",
                "--store",
                "/tmp/repro2-build-store/",
                "--option",
                "substitute",
                "true",
                "--no-link",
                "--json",
                "nixpkgs#hello",
            ]
        );
    }

    #[test]
    fn creates_a_local_rebuild_command_when_requested() {
        let build = Build::new(
            Installable::try_from("github:example/project#package".to_owned()).unwrap(),
        )
        .rebuild(true)
        .substitute(false)
        .store(Some(PathBuf::from("/tmp/builder-store")));

        assert_eq!(
            build.args(),
            [
                "build",
                "--store",
                "/tmp/builder-store",
                "--rebuild",
                "--option",
                "substitute",
                "false",
                "--no-link",
                "--json",
                "github:example/project#package",
            ]
        );
    }

    #[test]
    fn parses_all_outputs_from_nix_json() {
        let json = br#"[{"drvPath":"/nix/store/dmqn88qa7f73wcmdsg1722v7i73h1nay-cowsay-3.8.4.drv","outputs":{"man":"/nix/store/d4ppn8jk1dw0xsb0fyfizlwj2c7d4lxg-cowsay-3.8.4-man","out":"/nix/store/9xspds4a6qncn5kb8mgp6l2sdd4iplgf-cowsay-3.8.4"}}]"#;

        let outputs: Vec<BuildOutput> = serde_json::from_slice(json).unwrap();

        assert_eq!(outputs.len(), 1);
        assert_eq!(
            outputs[0].drv_path,
            Some(PathBuf::from(
                "/nix/store/dmqn88qa7f73wcmdsg1722v7i73h1nay-cowsay-3.8.4.drv"
            ))
        );
        assert_eq!(
            outputs[0].outputs["out"],
            "/nix/store/9xspds4a6qncn5kb8mgp6l2sdd4iplgf-cowsay-3.8.4"
        );
        assert_eq!(
            outputs[0].outputs["man"],
            "/nix/store/d4ppn8jk1dw0xsb0fyfizlwj2c7d4lxg-cowsay-3.8.4-man"
        );
    }
}
