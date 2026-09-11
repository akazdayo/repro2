pub mod narinfo;

async fn nix_cache_info() -> &'static str {
    "\
StoreDir: /nix/store
WantMassQuery: 0
Priority: 30
"
}
