use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Character Cards
        manager
            .create_table(
                Table::create()
                    .table(CharacterCards::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CharacterCards::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CharacterCards::Name).string().not_null())
                    .col(ColumnDef::new(CharacterCards::Description).string())
                    .col(ColumnDef::new(CharacterCards::Author).string())
                    .col(ColumnDef::new(CharacterCards::Avatar).string())
                    .col(ColumnDef::new(CharacterCards::Spec).string())
                    .col(ColumnDef::new(CharacterCards::SpecVersion).string())
                    .col(ColumnDef::new(CharacterCards::Data).text().not_null())
                    .col(
                        ColumnDef::new(CharacterCards::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CharacterCards::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // World Books
        manager
            .create_table(
                Table::create()
                    .table(WorldBooks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorldBooks::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorldBooks::Name).string().not_null())
                    .col(ColumnDef::new(WorldBooks::Description).string())
                    .col(ColumnDef::new(WorldBooks::Data).text().not_null())
                    .col(ColumnDef::new(WorldBooks::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(WorldBooks::UpdatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorldBooks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(CharacterCards::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CharacterCards {
    Table,
    Id,
    Name,
    Description,
    Author,
    Avatar,
    Spec,
    SpecVersion,
    Data,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum WorldBooks {
    Table,
    Id,
    Name,
    Description,
    Data,
    CreatedAt,
    UpdatedAt,
}
