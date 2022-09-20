use std::sync::Mutex;

use actix::*;
use actix_files::{Files};
use actix_web::{
    middleware::Logger, web, App, HttpServer,
    web::Data
};

mod server;
mod session;
mod queue;
mod reserr;
mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let queue = Data::new(Mutex::new(queue::Queue::new(10000)));

    // start chat server actor
    let server = server::ChatServer::new(queue.clone()).start();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::clone(&queue))
            .app_data(web::Data::new(server.clone()))
            .service(web::resource("/").to(routes::index))
            .route("/ws", web::get().to(routes::chat_route))
            .service(Files::new("/static", "./static"))
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}