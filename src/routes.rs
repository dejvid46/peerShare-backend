use std::sync::Mutex;
use std::time::Instant;

use actix::*;
use actix_files::NamedFile;
use actix_web::{web, HttpRequest, HttpResponse, Responder};

use actix_web_actors::ws;

use crate::queue;
use crate::reserr::ResErr;
use crate::server;
use crate::session;

pub async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

pub async fn test() -> impl Responder {
    NamedFile::open_async("./static/test.html").await.unwrap()
}

pub async fn send() -> impl Responder {
    NamedFile::open_async("./static/send.html").await.unwrap()
}

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
