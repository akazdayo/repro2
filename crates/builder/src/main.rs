use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use clap::Parser;
use nix::{build::Build, installable::Installable};
use serde::Serialize;

#[derive(Parser)]
struct Cli {
    /// Flake installable to build, for example nixpkgs#hello
    installable: String,
    /// ビルド結果を登録する Registry の URL
    #[arg(long, default_value = "http://127.0.0.1:3001")]
    registry_url: String,
}

/// Registry に送信する、Nix のビルド出力ごとの報告内容。
#[derive(Serialize)]
struct BuildReport<'a> {
    drv_path: Option<&'a str>,
    output_name: &'a str,
    store_path_hash: &'a str,
    store_path: &'a str,
    nar_hash: &'a str,
    nar_size: i64,
}

fn main() -> Result<()> {
    // コマンドライン引数を読み込み、flake の参照と属性名の形式を確認する。
    let cli = Cli::parse();
    let installable = Installable::try_from(cli.installable)
        .map_err(|_| anyhow::anyhow!("expected a flake installable such as nixpkgs#hello"))?;
    // ビルドごとに衝突しない一時 store の名前を作る。
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_nanos();
    let store =
        PathBuf::from("/tmp").join(format!("repro2-build-store-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&store).context("failed to create an empty build store")?;

    // 既存の store からの再ビルドはせず、NixOS キャッシュからの代替取得を許可する。
    let build = Build::new(installable)
        .rebuild(false)
        .substitute(true)
        .store(Some(store.clone()))
        .substituters(Some("https://cache.nixos.org/".to_owned()));

    // Registry への報告が応答しない場合に備え、HTTP 要求にタイムアウトを設定する。
    let http = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    println!("Store: {}", store.display());
    // Nix が返した各ビルド結果について、出力ごとの情報を取得して Registry に報告する。
    for result in build.run()? {
        let drv_path = result.drv_path.as_ref().and_then(|path| path.to_str());
        for (name, path) in &result.outputs {
            // 出力の NAR hash とサイズを Nix store から取得して表示する。
            let info = build.path_info(path)?;
            println!(
                "{name}: {path}\n  NarHash: {}\n  NarSize: {}",
                info.nar_hash, info.nar_size
            );

            // store path の先頭にあるハッシュ部分を取り出す。
            let store_path_hash = path
                .strip_prefix("/nix/store/")
                .and_then(|name| name.split_once('-'))
                .map(|(hash, _)| hash)
                .context("unexpected Nix store path")?;

            // 出力名や store path と NAR 情報をまとめ、Registry の報告形式に変換する。
            let report = BuildReport {
                drv_path,
                output_name: name,
                store_path_hash,
                store_path: path,
                nar_hash: &info.nar_hash,
                nar_size: info.nar_size.try_into()?,
            };

            // Registry の build-reports エンドポイントへ送信し、HTTP エラーも失敗として扱う。
            http.post(format!(
                "{}/build-reports",
                cli.registry_url.trim_end_matches('/')
            ))
            .json(&report)
            .send()?
            .error_for_status()?;
            println!("Registered: {path}");
        }
    }

    Ok(())
}
