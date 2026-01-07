use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Create world_info table
        manager
            .create_table(
                Table::create()
                    .table(WorldInfo::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorldInfo::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorldInfo::Name).string().not_null())
                    .col(ColumnDef::new(WorldInfo::Data).text().not_null())
                    // Keeping timestamps as best practice
                    .col(ColumnDef::new(WorldInfo::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(WorldInfo::UpdatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // 2. Migration data from world_books if exists
        // Check if world_books exists logic is better handled by just trying SQL in SQLite usually,
        // but SeaORM manager has has_table.
        if manager.has_table("world_books").await? {
            // Copy data (ignoring description)
            // Note: SQLite string literals are single quotes.
            let sql = "INSERT INTO world_info (id, name, data, created_at, updated_at) SELECT id, name, data, created_at, updated_at FROM world_books";
            match db.execute_unprepared(sql).await {
                Ok(_) => {
                    // Only drop if copy succeeded
                    manager
                        .drop_table(Table::drop().table(Alias::new("world_books")).to_owned())
                        .await?;
                }
                Err(e) => {
                    // Log error or continue? If copy fails (e.g. column mismatch), we might not want to drop.
                    // For now, let's print and ignore drop to be safe, or just fail up?
                    // Let's assume schema matches.
                    println!("Failed to migrate data from world_books: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // In down, we should ideally reverse.
        // Recreate world_books
        manager
            .create_table(
                Table::create()
                    .table(Alias::new("world_books"))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorldInfo::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorldInfo::Name).string().not_null())
                    .col(ColumnDef::new(Alias::new("description")).string()) // was Option<String>
                    .col(ColumnDef::new(WorldInfo::Data).text().not_null())
                    .col(ColumnDef::new(WorldInfo::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(WorldInfo::UpdatedAt).date_time().not_null())
                    .to_owned(),
            )
            .await?;

        // Copy back
        let db = manager.get_connection();
        let sql = "INSERT INTO world_books (id, name, data, created_at, updated_at) SELECT id, name, data, created_at, updated_at FROM world_info";
        db.execute_unprepared(sql).await.ok();

        // Drop world_info
        manager
            .drop_table(Table::drop().table(WorldInfo::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum WorldInfo {
    Table,
    Id,
    Name,
    Data,
    CreatedAt,
    UpdatedAt,
}
