use sea_orm::ActiveModelTrait;
use sea_orm::sqlx::types::chrono;
use sea_orm::{ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use sea_orm::{prelude::Uuid, sqlx::types::chrono::Utc};
use tracing::instrument;

use crate::entities::scan;
use crate::utils;

#[derive(Debug)]
pub struct ScanId {
    pub session: String,
    pub device: Uuid,
}

#[instrument(name = "get_scan", skip(db))]
pub async fn get_by_session(db: &DatabaseConnection, session: &str) -> Result<scan::Model, DbErr> {
    scan::Entity::find()
        .filter(scan::Column::Session.eq(session))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Scan not found".into()))
}

#[instrument(name = "update_scan", skip(db))]
pub async fn create_or_update(db: &DatabaseConnection, id: ScanId) -> Result<scan::Model, DbErr> {
    let ScanId { session, device } = id;
    let scan: Option<scan::Model> = scan::Entity::find()
        .filter(scan::Column::Session.eq(&session))
        .filter(scan::Column::Device.eq(device))
        .one(db)
        .await?;

    let now = Utc::now();

    if let Some(scan) = scan {
        let mut scan: scan::ActiveModel = scan.into();
        scan.updated_at = Set(now.naive_utc());
        scan.update(db).await
    } else {
        let url = utils::file_url(&session, &device.to_string());
        let new_scan = scan::ActiveModel {
            session: Set(session),
            device: Set(device),
            path: Set(url),
            created_at: Set(now.naive_utc()),
            updated_at: Set(now.naive_utc()),
            ..Default::default()
        };

        new_scan.insert(db).await
    }
}

#[instrument(name = "assign_patient_to_scan", skip(db))]
pub async fn assign_patient(
    db: &DatabaseConnection,
    session: &str,
    pateint: Uuid,
) -> Result<scan::Model, DbErr> {
    let mut scan: scan::ActiveModel = scan::Entity::find()
        .filter(scan::Column::Session.eq(session))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Scan not found".into()))
        .map(Into::into)?;

    scan.patient_id = Set(Some(pateint));
    scan.update(db).await
}
