use std::time::{Duration, Instant};

use uuid::Uuid;
use actix::{Actor, ActorContext, ActorFutureExt, AsyncContext, ContextFutureSpawner, WrapFuture};
use actix::{Addr, Running, Handler, StreamHandler};

use actix_web_actors::ws;

use super::lobby::Lobby;
use super::message::*;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct SessionConnection {
    pub session: String,
    pub client: Uuid,
    pub lobby_addr: Addr<Lobby>,
    pub hb: Instant,
}

impl SessionConnection {
    pub fn new(client: Uuid, session: String, lobby_addr: Addr<Lobby>) -> Self {
        Self {
            client,
            session,
            lobby_addr,
            hb: Instant::now(),
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("WebSocket heartbeat failed for client: {:?}", act.session);
                act.lobby_addr.do_send(Disconnect {
                    session: act.session.to_owned(),
                    client: act.client.to_owned(),
                });
                ctx.stop();
                return;
            }
            ctx.ping(b"PING");
        });
    }
}

impl Actor for SessionConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        let addr = ctx.address();
        self.lobby_addr
            .send(Connect {
                addr: addr.recipient(),
                session: self.session.clone(),
                client: self.client,
            })
            .into_actor(self)
            .then(|res, _act, ctx| {
                if res.is_err() {
                    ctx.stop();
                }
                futures::future::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.lobby_addr.do_send(Disconnect { session: self.session.clone(), client: self.client.clone() });
        Running::Stop
    }
}

impl Handler<SessionMessage> for SessionConnection {
    type Result = ();

    fn handle(&mut self, msg: SessionMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SessionConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                println!("Received text: {}", text);
                ctx.text(format!("Echo: {}", text));
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}