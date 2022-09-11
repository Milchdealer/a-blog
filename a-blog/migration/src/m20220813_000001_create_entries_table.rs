use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Entries::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Entries::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Entries::UserId).big_integer().not_null())
                    .col(ColumnDef::new(Entries::EntryDate).date().not_null())
                    .col(ColumnDef::new(Entries::Exercise).string().not_null())
                    .col(ColumnDef::new(Entries::Sets).unsigned().not_null())
                    .col(ColumnDef::new(Entries::RepsDuration).unsigned().not_null())
                    .col(ColumnDef::new(Entries::Load).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Entries::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Entries {
    Table,
    Id,
    UserId,
    EntryDate,
    Exercise,
    Sets,
    RepsDuration,
    Load,
}
