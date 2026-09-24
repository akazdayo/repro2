mod nix;

use std::{path::PathBuf, time::SystemTime};

use anyhow::{Context, Result};
use clap::Parser;
use nix::{build::Build, installable::Installable};

#[derive(Parser)]
struct Cli {
    /// Flake installable to build, for example nixpkgs#hello
    installable: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let installable = Installable::try_from(cli.installable)
        .map_err(|_| anyhow::anyhow!("expected a flake installable such as nixpkgs#hello"))?;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_nanos();
    let store =
        PathBuf::from("/tmp").join(format!("repro2-build-store-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&store).context("failed to create an empty build store")?;
    let build = Build::new(installable)
        .rebuild(false)
        .substitute(true)
        .store(Some(store.clone()))
        .substituters(Some("https://cache.nixos.org/".to_owned()));

    println!("Store: {}", store.display());
    for result in build.run()? {
        for (name, path) in result.outputs {
            let info = build.path_info(&path)?;
            println!(
                "{name}: {path}\n  NarHash: {}\n  NarSize: {}",
                info.nar_hash, info.nar_size
            );
        }
    }

    Ok(())
}
