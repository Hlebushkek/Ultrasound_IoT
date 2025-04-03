use actix::{Message, Recipient};
use uuid::Uuid;

#[derive(Message)]
#[rtype(result = "()")]
pub struct SessionMessage(pub String);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<SessionMessage>,
    pub session: String,
    pub client: Uuid,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub session: String,
    pub client: Uuid,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ScanData {
    pub session: String,
}
