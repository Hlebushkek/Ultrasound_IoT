use secrecy::ExposeSecret;

use tracing::debug;

use actix::prelude::*;
use actix_web::{App, HttpServer, web::Data};

use services::sea_orm::Database;

use ultrasound_iot_server::app_state::AppState;
use ultrasound_iot_server::routes;
use ultrasound_iot_server::session::lobby::Lobby;
use ultrasound_iot_server::settings::Settings;
use ultrasound_iot_server::utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let settings = Settings::new().expect("Failed to initialize settings");

    utils::init_tracing("debug");

    debug!("{:?}", settings);

    let conn_str = settings.database.connection_string();
    let conn = Database::connect(conn_str.expose_secret())
        .await
        .expect("Failed to connect to database");
    let app_state = AppState { conn };

    let lobby_addr = Lobby::default().start();

    HttpServer::new(move || {
        App::new()
            .configure(routes::configure)
            .app_data(Data::new(app_state.clone()))
            .app_data(Data::new(lobby_addr.clone()))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
