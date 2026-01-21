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
                        ColumnDef::new(CharacterCard::Source)
                            .string()
                            .not_null()
                            .default("import"), // 默认值兼容现有数据
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
                    .drop_column(CharacterCard::Source)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum CharacterCard {
    #[sea_orm(iden = "character_cards")]
    Table,
    Source,
}
