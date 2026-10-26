use sea_orm_migration::{prelude::*, schema::*};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260923_172120_create_build_reports"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("build_reports")
                    .col(integer("id").auto_increment().primary_key())
                    // The previous schema did not record the derivation or output name.
                    .col(string_null("drv_path"))
                    .col(string_null("output_name"))
                    .col(string("store_path_hash"))
                    .col(string("store_path"))
                    .col(string("nar_hash"))
                    .col(big_integer("nar_size"))
                    .col(string("cache_url"))
                    .col(date_time_default_now("created_at"))
                    .to_owned(),
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO build_reports (store_path_hash, store_path, nar_hash, nar_size, cache_url, created_at) \
                 SELECT store_path_hash, store_path, nar_hash, nar_size, cache_url, updated_at FROM nar_info",
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_build_reports_store_path_hash")
                    .table("build_reports")
                    .col("store_path_hash")
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_build_reports_input")
                    .table("build_reports")
                    .col("drv_path")
                    .col("output_name")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table("nar_info").to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("nar_info")
                    .col(string("store_path_hash").primary_key())
                    .col(string("store_path"))
                    .col(string("nar_hash"))
                    .col(big_integer("nar_size"))
                    .col(string("cache_url"))
                    .col(string("status").default("pending"))
                    .col(date_time_default_now("updated_at"))
                    .to_owned(),
            )
            .await?;
        // The old primary key cannot represent multiple reports for one store path.
        // In that case the insert fails rather than silently discarding reports.
        manager
            .get_connection()
            .execute_unprepared(
                "INSERT INTO nar_info (store_path_hash, store_path, nar_hash, nar_size, cache_url, updated_at) \
                 SELECT store_path_hash, store_path, nar_hash, nar_size, cache_url, created_at FROM build_reports",
            )
            .await?;
        manager
            .drop_table(Table::drop().table("build_reports").to_owned())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm_migration::sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    #[tokio::test]
    async fn preserves_existing_records_and_accepts_multiple_reports_for_one_path() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let manager = SchemaManager::new(&db);
        super::super::m20220101_000001_create_table::Migration
            .up(&manager)
            .await
            .unwrap();
        db.execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO nar_info (store_path_hash, store_path, nar_hash, nar_size, cache_url) \
             VALUES ('abc', '/nix/store/abc-example', 'sha256-first', 10, 'https://cache.example/')",
        ))
        .await
        .unwrap();

        Migration.up(&manager).await.unwrap();
        db.execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO build_reports (drv_path, output_name, store_path_hash, store_path, nar_hash, nar_size, cache_url) \
             VALUES ('/nix/store/example.drv', 'out', 'abc', '/nix/store/abc-example', \
                     'sha256-second', 10, 'https://other-cache.example/')",
        ))
        .await
        .unwrap();

        let reports = db
            .query_all_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT drv_path, nar_hash FROM build_reports WHERE store_path_hash = 'abc' ORDER BY id",
            ))
            .await
            .unwrap();
        assert_eq!(reports.len(), 2);
        assert_eq!(
            reports[0]
                .try_get::<Option<String>>("", "drv_path")
                .unwrap(),
            None
        );
        assert_eq!(
            reports[0].try_get::<String>("", "nar_hash").unwrap(),
            "sha256-first"
        );
        assert_eq!(
            reports[1].try_get::<String>("", "nar_hash").unwrap(),
            "sha256-second"
        );
    }
}
