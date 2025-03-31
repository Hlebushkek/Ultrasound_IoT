use std::collections::HashMap;

use actix::{Actor, Context, Handler, Recipient};

use super::message::*;

type Socket = Recipient<SessionMessage>;

/// Store map of session id to recipient
pub struct Lobby {
    sessions: HashMap<String, Socket>,
}

impl Default for Lobby {
    fn default() -> Self {
        Lobby {
            sessions: HashMap::new(),
        }
    }
}

impl Actor for Lobby {
    type Context = Context<Self>;
}

impl Handler<Connect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) {
        println!(
            "Client {}, connected to session: {}",
            msg.client, msg.session
        );
        self.sessions.insert(msg.session, msg.addr);
    }
}

impl Handler<Disconnect> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _ctx: &mut Context<Self>) {
        println!("Client {}, disconnected: {}", msg.client, msg.session);
        self.sessions.remove(&msg.session);
    }
}

impl Handler<ScanData> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: ScanData, _ctx: &mut Context<Self>) {
        println!("Received scan data for session: {:?}", msg.session);
        if let Some(addr) = self.sessions.get(&msg.session) {
            let message = SessionMessage(msg.session);
            addr.do_send(message);
        } else {
            println!("No active session for id: {:?}", msg.session);
        }
    }
}
