//! 创建图库分类表
//!
//! 独立于角色库分类，用于图库的分类管理

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ImageCategory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ImageCategory::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ImageCategory::Name)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ImageCategory::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(ImageCategory::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建排序索引
        manager
            .create_index(
                Index::create()
                    .name("idx_image_category_sort")
                    .table(ImageCategory::Table)
                    .col(ImageCategory::SortOrder)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ImageCategory::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum ImageCategory {
    Table,
    Id,
    Name,
    SortOrder,
    CreatedAt,
}
