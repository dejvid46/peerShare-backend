use std::{env, sync::Mutex};

use actix::*;
use actix_files::Files;
use actix_web::{middleware::Logger, web, web::Data, App, HttpServer};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

mod queue;
mod reserr;
mod routes;
mod server;
mod session;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let queue = Data::new(Mutex::new(queue::Queue::new(env::var("QUEUE_LENGHT").unwrap().parse::<usize>().unwrap())));

    // start chat server actor
    let server = server::ChatServer::new(queue.clone()).start();

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file("server.key", SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file("server.crt").unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::clone(&queue))
            .app_data(web::Data::new(server.clone()))
            .route("/ws", web::get().to(routes::chat_route))
            .service(Files::new("/", "./static").index_file("index.html"))
            .wrap(Logger::default())
    })
    .bind_openssl(env::var("ADDR").unwrap(), builder)?
    .run()
    .await
}
