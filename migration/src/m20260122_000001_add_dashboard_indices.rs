use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Index for TOKEN SUM: (deleted_at, token_count_total)
        manager
            .create_index(
                Index::create()
                    .name("idx_character_cards_token_sum")
                    .table(CharacterCard::Table)
                    .col(CharacterCard::DeletedAt)
                    .col(CharacterCard::TokenCountTotal)
                    .to_owned(),
            )
            .await?;

        // Index for RECENT EDITS sort: (deleted_at, updated_at)
        manager
            .create_index(
                Index::create()
                    .name("idx_character_cards_updated")
                    .table(CharacterCard::Table)
                    .col(CharacterCard::DeletedAt)
                    .col(CharacterCard::UpdatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_character_cards_token_sum")
                    .table(CharacterCard::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_character_cards_updated")
                    .table(CharacterCard::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum CharacterCard {
    #[sea_orm(iden = "character_cards")]
    Table,
    DeletedAt,
    TokenCountTotal,
    UpdatedAt,
}
