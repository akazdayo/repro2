use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260925_000001_nullable_cache_url"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE TABLE build_reports_new (\
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, \
                drv_path varchar NULL, output_name varchar NULL, store_path_hash varchar NOT NULL, \
                store_path varchar NOT NULL, nar_hash varchar NOT NULL, nar_size integer NOT NULL, \
                cache_url varchar NULL, created_at datetime_text NOT NULL DEFAULT CURRENT_TIMESTAMP\
            )",
        )
        .await?;
        db.execute_unprepared(
            "INSERT INTO build_reports_new \
             (id, drv_path, output_name, store_path_hash, store_path, nar_hash, nar_size, cache_url, created_at) \
             SELECT id, drv_path, output_name, store_path_hash, store_path, nar_hash, nar_size, cache_url, created_at \
             FROM build_reports",
        )
        .await?;
        db.execute_unprepared("DROP TABLE build_reports").await?;
        db.execute_unprepared("ALTER TABLE build_reports_new RENAME TO build_reports")
            .await?;
        db.execute_unprepared(
            "CREATE INDEX idx_build_reports_store_path_hash ON build_reports (store_path_hash)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX idx_build_reports_input ON build_reports (drv_path, output_name)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Migration(
            "build reports without a cache URL cannot fit the previous schema".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm_migration::sea_orm::{ConnectionTrait, Database};

    #[tokio::test]
    async fn retains_old_reports_and_accepts_a_report_without_cache_url() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let manager = SchemaManager::new(&db);
        super::super::m20220101_000001_create_table::Migration
            .up(&manager)
            .await
            .unwrap();
        super::super::m20260923_172120_create_build_reports::Migration
            .up(&manager)
            .await
            .unwrap();
        db.execute_unprepared(
            "INSERT INTO build_reports (store_path_hash, store_path, nar_hash, nar_size, cache_url) \
             VALUES ('old', '/nix/store/old-example', 'sha256-old', 10, 'https://cache.example/')",
        )
        .await
        .unwrap();

        Migration.up(&manager).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO build_reports (store_path_hash, store_path, nar_hash, nar_size) \
             VALUES ('new', '/nix/store/new-example', 'sha256-new', 20)",
        )
        .await
        .unwrap();
        let rows = db
            .query_all_raw(sea_orm_migration::sea_orm::Statement::from_string(
                sea_orm_migration::sea_orm::DbBackend::Sqlite,
                "SELECT store_path_hash, cache_url FROM build_reports ORDER BY id",
            ))
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].try_get::<String>("", "store_path_hash").unwrap(),
            "old"
        );
        assert_eq!(
            rows[0].try_get::<String>("", "cache_url").unwrap(),
            "https://cache.example/"
        );
        assert_eq!(
            rows[1].try_get::<String>("", "store_path_hash").unwrap(),
            "new"
        );
        assert_eq!(
            rows[1].try_get::<Option<String>>("", "cache_url").unwrap(),
            None
        );
    }
}
