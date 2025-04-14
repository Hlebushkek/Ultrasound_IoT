use crate::m20250401_163440_create_patient_table::Patient;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Scan::Table)
                    .if_not_exists()
                    .col(string(Scan::Session))
                    .col(uuid(Scan::Device))
                    .col(string(Scan::Path))
                    .col(uuid_null(Scan::PatientId))
                    .col(date_time(Scan::CreatedAt))
                    .col(date_time(Scan::UpdatedAt))
                    .primary_key(Index::create().col(Scan::Session).col(Scan::Device))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-scan-patient")
                            .from(Scan::Table, Scan::PatientId)
                            .to(Patient::Table, Patient::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Scan::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Scan {
    Table,
    Session,
    Device,
    Path,
    PatientId,
    CreatedAt,
    UpdatedAt,
}
