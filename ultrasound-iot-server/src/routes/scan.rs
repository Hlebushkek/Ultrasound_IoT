use services::utils;
use uuid::Uuid;
use serde::Deserialize;
use tracing::debug;

use actix::Addr;
use actix_web::{get, patch, post, HttpResponse, Responder};
use actix_web::web::{Data, Json, Path};
use actix_web::Error;
use actix_files::NamedFile;

use services::scans::{self, ScanId};

use crate::app_state::AppState;
use crate::session::lobby::Lobby;
use crate::session::message::ScanData;

#[derive(Deserialize)]
pub struct ScanPayload {
    pub device: Uuid,
    pub session: String,
    pub values: Vec<f32>,
}

#[derive(Deserialize)]
pub struct ScanRequest {
    pub session: String,
    pub client: Uuid,
}

#[derive(Deserialize)]
pub struct AssignRequest {
    pub patient: Uuid,
}

#[get("")]
pub async fn get(data: Data<AppState>, payload: Json<ScanRequest>) -> Result<impl Responder, Error> {
    println!("Downloading scan from session {}, for client {}", payload.session, payload.client);
    
    let scan = scans::get_by_session(&data.conn, &payload.session)
        .await
        .map_err(|e| crate::utils::to_internal_error("DB", e))?;

    let path = utils::file_url(&scan.session, &scan.device.to_string());

    Ok(NamedFile::open_async(path).await)
}

#[post("")]
pub async fn receive(data: Data<AppState>, lobby: Data<Addr<Lobby>>, payload: Json<ScanPayload>) -> Result<impl Responder, Error> {
    let ScanPayload { device, session, values: _ } = payload.into_inner();
    debug!("Received scan data from device {}: {}", device, session);
    
    let scan_id = ScanId { session: session.clone(), device };
    let scan = services::scans::create_or_update(&data.conn, scan_id)
        .await
        .map_err(|e| crate::utils::to_internal_error("DB", e))?;
    
    // TODO: Select scan file randomly from dataset, copy to path
    debug!("{}", scan.path);

    lobby.do_send(ScanData { session });

    Ok(HttpResponse::Ok().body("Scan data processed"))
}

#[patch("/{session}")]
pub async fn assign_patient(data: Data<AppState>, payload: Json<AssignRequest>, session: Path<String>) -> Result<impl Responder, Error> {
    debug!("Assigning patient {} to session {}", payload.patient, session);
    
    let _ = services::scans::assign_patient(&data.conn, &session, payload.patient)
        .await
        .map_err(|e| crate::utils::to_internal_error("DB", e))?;

    Ok(HttpResponse::Ok().body("Assigned"))
}
