use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite doesn't support multiple ADD COLUMN in one statement
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .add_column(ColumnDef::new(CharacterCards::CustomSummary).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .add_column(ColumnDef::new(CharacterCards::UserNote).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .add_column(
                        ColumnDef::new(CharacterCards::MetadataModified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite supports DROP COLUMN since 3.35.0, but usually one by one is safer
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .drop_column(CharacterCards::CustomSummary)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .drop_column(CharacterCards::UserNote)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .drop_column(CharacterCards::MetadataModified)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum CharacterCards {
    Table,
    CustomSummary,
    UserNote,
    MetadataModified,
}
