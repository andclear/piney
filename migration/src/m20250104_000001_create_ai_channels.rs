use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AiChannels::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AiChannels::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AiChannels::Name).string().not_null())
                    .col(ColumnDef::new(AiChannels::BaseUrl).string().not_null())
                    .col(ColumnDef::new(AiChannels::ApiKey).string().not_null())
                    .col(ColumnDef::new(AiChannels::ModelId).string().not_null())
                    .col(
                        ColumnDef::new(AiChannels::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AiChannels::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(AiChannels::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AiChannels::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AiChannels {
    Table,
    Id,
    Name,
    BaseUrl,
    ApiKey,
    ModelId,
    IsActive,
    CreatedAt,
    UpdatedAt,
}
