pub mod scan;
pub mod session;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/scan")
            .service(scan::get)
            .service(scan::receive)
            .service(scan::assign_patient),
    )
    .service(web::scope("/session").service(session::ws_index));
}
