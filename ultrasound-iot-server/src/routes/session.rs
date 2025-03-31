use uuid::Uuid;

use actix::Addr;
use actix_web::web::{Data, Path, Payload};
use actix_web::{Error, HttpRequest, HttpResponse, get};
use actix_web_actors::ws;

use crate::session::lobby::Lobby;
use crate::session::session_connection::SessionConnection;

#[get("/ws/{session_id}/{client_id}")]
pub async fn ws_index(
    req: HttpRequest,
    stream: Payload,
    path: Path<(String, Uuid)>,
    lobby: Data<Addr<Lobby>>,
) -> Result<HttpResponse, Error> {
    let (session, client) = path.into_inner();
    let ws = SessionConnection::new(client, session, lobby.get_ref().clone());
    ws::start(ws, &req, stream)
}
