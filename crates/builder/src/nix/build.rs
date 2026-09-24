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
    substituters: Option<String>,
}

impl Build {
    pub fn new(installable: Installable) -> Self {
        Self {
            installable,
            rebuild: false,
            substitute: true,
            store: Some(PathBuf::from("/tmp/repro2-build-store/")),
            substituters: None,
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

    pub fn substituters(mut self, substituters: Option<String>) -> Self {
        self.substituters = substituters;
        self
    }

    /// Runs `nix build` without creating a result symlink and returns its JSON result.
    /// Unless `self.store` is set, Nix inherits the caller's store and daemon settings.
    pub fn run(&self) -> Result<Vec<BuildOutput>> {
        if let Some(store) = &self.store {
            std::fs::create_dir_all(store.join("builds"))
                .context("failed to create Nix build directory")?;
        }
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

    pub fn path_info(&self, store_path: &str) -> Result<PathInfo> {
        let mut command = Command::new("nix");
        command.args(["path-info", "--json", "--json-format", "1"]);
        if let Some(store) = &self.store {
            command.arg("--store").arg(store);
        }
        let output = command
            .arg(store_path)
            .output()
            .context("failed to start nix path-info")?;
        if !output.status.success() {
            bail!(
                "nix path-info failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        let mut paths: BTreeMap<String, PathInfo> =
            serde_json::from_slice(&output.stdout).context("failed to parse nix path-info")?;
        paths
            .remove(store_path)
            .context("built store path not found in nix path-info")
    }

    fn args(&self) -> Vec<String> {
        let mut args = vec![String::from("build")];
        if let Some(store) = &self.store {
            args.extend([
                String::from("--store"),
                store.to_string_lossy().into_owned(),
                String::from("--option"),
                String::from("build-dir"),
                store.join("builds").to_string_lossy().into_owned(),
            ]);
        }
        if self.rebuild {
            args.push(String::from("--rebuild"));
        }
        args.extend([
            String::from("--option"),
            String::from("substitute"),
            String::from(if self.substitute { "true" } else { "false" }),
        ]);
        if let Some(substituters) = &self.substituters {
            args.extend([
                String::from("--option"),
                String::from("substituters"),
                substituters.clone(),
            ]);
        }
        args.extend([
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathInfo {
    pub nar_hash: String,
    pub nar_size: u64,
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
                "build-dir",
                "/tmp/repro2-build-store/builds",
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
    fn accepts_independent_build_settings() {
        let build =
            Build::new(Installable::try_from("github:example/project#package".to_owned()).unwrap())
                .rebuild(true)
                .substitute(false)
                .store(None)
                .substituters(Some("https://cache.nixos.org/".to_owned()));

        assert_eq!(
            build.args(),
            [
                "build",
                "--rebuild",
                "--option",
                "substitute",
                "false",
                "--option",
                "substituters",
                "https://cache.nixos.org/",
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
