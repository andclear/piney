//! 创建图库表
//!
//! 用于存储导入的图片及其元数据

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Image::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Image::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Image::Title).string_len(255).not_null())
                    .col(ColumnDef::new(Image::CategoryId).uuid().null())
                    .col(ColumnDef::new(Image::Tags).text().not_null().default("[]"))
                    .col(ColumnDef::new(Image::FilePath).text().not_null())
                    .col(ColumnDef::new(Image::ThumbnailPath).text().not_null())
                    .col(ColumnDef::new(Image::Width).integer().not_null())
                    .col(ColumnDef::new(Image::Height).integer().not_null())
                    .col(ColumnDef::new(Image::FileSize).big_integer().not_null())
                    .col(ColumnDef::new(Image::ColorCategory).string_len(10).null())
                    .col(
                        ColumnDef::new(Image::IsAi)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Image::AiPlatform).string_len(20).null())
                    .col(ColumnDef::new(Image::AiPrompt).text().null())
                    .col(ColumnDef::new(Image::AiNegativePrompt).text().null())
                    .col(
                        ColumnDef::new(Image::IsAuthorized)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Image::IsFavorite)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Image::UserNotes).text().null())
                    .col(
                        ColumnDef::new(Image::CharCards)
                            .text()
                            .not_null()
                            .default("[]"),
                    )
                    .col(ColumnDef::new(Image::CreatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_image_category")
                            .from(Image::Table, Image::CategoryId)
                            .to(ImageCategory::Table, ImageCategory::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建分类索引
        manager
            .create_index(
                Index::create()
                    .name("idx_image_category_id")
                    .table(Image::Table)
                    .col(Image::CategoryId)
                    .to_owned(),
            )
            .await?;

        // 创建收藏索引
        manager
            .create_index(
                Index::create()
                    .name("idx_image_favorite")
                    .table(Image::Table)
                    .col(Image::IsFavorite)
                    .to_owned(),
            )
            .await?;

        // 创建颜色分类索引
        manager
            .create_index(
                Index::create()
                    .name("idx_image_color")
                    .table(Image::Table)
                    .col(Image::ColorCategory)
                    .to_owned(),
            )
            .await?;

        // 创建导入时间索引
        manager
            .create_index(
                Index::create()
                    .name("idx_image_created_at")
                    .table(Image::Table)
                    .col(Image::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Image::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Image {
    Table,
    Id,
    Title,
    CategoryId,
    Tags,
    FilePath,
    ThumbnailPath,
    Width,
    Height,
    FileSize,
    ColorCategory,
    IsAi,
    AiPlatform,
    AiPrompt,
    AiNegativePrompt,
    IsAuthorized,
    IsFavorite,
    UserNotes,
    CharCards,
    CreatedAt,
}

#[derive(Iden)]
enum ImageCategory {
    Table,
    Id,
}
