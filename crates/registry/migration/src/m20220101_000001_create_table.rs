use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table("nar_info")
                    .if_not_exists()
                    .col(string("store_path_hash").primary_key())
                    .col(string("store_path"))
                    .col(string("nar_hash"))
                    .col(big_integer("nar_size"))
                    .col(string("cache_url"))
                    .col(string("status").default("pending"))
                    .col(date_time_default_now("updated_at"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table("nar_info").to_owned())
            .await
    }
}
