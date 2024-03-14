use std::sync::Mutex;
use std::time::Instant;

use actix::*;
use actix_web::{web, HttpRequest, HttpResponse};

use actix_web_actors::ws;

use crate::queue;
use crate::reserr::ResErr;
use crate::server;
use crate::session;

/// Entry point for our websocket route
pub async fn chat_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::ChatServer>>,
    queue: web::Data<Mutex<queue::Queue>>,
) -> Result<HttpResponse, ResErr> {
    let room;
    {
        let mut guard = queue.lock().unwrap();
        room = guard.reserve().copied();
    }
    match room {
        Some(x) => ws::start(
            session::WsChatSession {
                id: 0,
                hb: Instant::now(),
                room: x.to_owned(),
                addr: srv.get_ref().clone(),
            },
            &req,
            stream,
        )
        .map_err(|_| {
            {
                let mut guard = queue.lock().unwrap();
                guard.refund(&x);
            }
            ResErr::BadClientData("something wrong")
        }),
        None => Err(ResErr::BadClientData("full queue")),
    }
}
