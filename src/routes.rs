use std::{
    time::Instant,
};
use std::sync::Mutex;

use actix::*;
use actix_files::{NamedFile};
use actix_web::{
    web, HttpRequest, HttpResponse, Responder
};

use actix_web_actors::ws;

use crate::server;
use crate::session;
use crate::queue;
use crate::reserr::ResErr;

pub async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

/// Entry point for our websocket route
pub async fn chat_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::ChatServer>>,
    queue: web::Data<Mutex<queue::Queue>>
) -> Result<HttpResponse, ResErr> {
    let mut guard = queue.lock().unwrap();
    let queue = &mut *guard;
    let room = queue.reserve();
    match room {
        Some(x) => 
            ws::start(
                session::WsChatSession {
                    id: 0,
                    hb: Instant::now(),
                    room: x.to_owned(),
                    name: None,
                    addr: srv.get_ref().clone(),
                },
                &req,
                stream,
            ).map_err(|_| ResErr::BadClientData("something wrong")),
        None => Err(ResErr::BadClientData("full queue"))
    }
}