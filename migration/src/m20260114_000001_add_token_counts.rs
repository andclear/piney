use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCard::Table)
                    .add_column(
                        ColumnDef::new(CharacterCard::TokenCountTotal)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCard::Table)
                    .add_column(
                        ColumnDef::new(CharacterCard::TokenCountSpec)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCard::Table)
                    .add_column(ColumnDef::new(CharacterCard::TokenCountWb).integer().null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCard::Table)
                    .add_column(
                        ColumnDef::new(CharacterCard::TokenCountOther)
                            .integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCard::Table)
                    .drop_column(CharacterCard::TokenCountTotal)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCard::Table)
                    .drop_column(CharacterCard::TokenCountSpec)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCard::Table)
                    .drop_column(CharacterCard::TokenCountWb)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(CharacterCard::Table)
                    .drop_column(CharacterCard::TokenCountOther)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum CharacterCard {
    #[sea_orm(iden = "character_cards")]
    Table,
    TokenCountTotal,
    TokenCountSpec,
    TokenCountWb,
    TokenCountOther,
}
