use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. 创建 categories 表
        manager
            .create_table(
                Table::create()
                    .table(Categories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Categories::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Categories::Name).string().not_null())
                    .col(
                        ColumnDef::new(Categories::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Categories::CreatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // 2. 为 character_cards 表添加新字段
        // category_id (外键，可空)
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .add_column(ColumnDef::new(CharacterCards::CategoryId).uuid())
                    .to_owned(),
            )
            .await?;

        // tags (JSON 数组，默认空数组)
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .add_column(
                        ColumnDef::new(CharacterCards::Tags)
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    .to_owned(),
            )
            .await?;

        // rating (评分，0-5)
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .add_column(
                        ColumnDef::new(CharacterCards::Rating)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .to_owned(),
            )
            .await?;

        // cover_blur (封面是否模糊)
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .add_column(
                        ColumnDef::new(CharacterCards::CoverBlur)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        // version (版本号)
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .add_column(ColumnDef::new(CharacterCards::Version).string())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除 character_cards 的新字段
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .drop_column(CharacterCards::CategoryId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .drop_column(CharacterCards::Tags)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .drop_column(CharacterCards::Rating)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .drop_column(CharacterCards::CoverBlur)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCards::Table)
                    .drop_column(CharacterCards::Version)
                    .to_owned(),
            )
            .await?;

        // 删除 categories 表
        manager
            .drop_table(Table::drop().table(Categories::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Categories {
    Table,
    Id,
    Name,
    SortOrder,
    CreatedAt,
}

#[derive(DeriveIden)]
enum CharacterCards {
    Table,
    CategoryId,
    Tags,
    Rating,
    CoverBlur,
    Version,
}
