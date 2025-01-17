pub mod db_connection;
pub mod handler;
pub mod model;
pub mod router;
pub mod schema;

#[macro_use]
extern crate diesel;
extern crate actix;
extern crate actix_web;
extern crate dotenv;
extern crate serde;

use actix_web::{middleware::Logger, web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    println!("Starting HTTP server: localhost:8080");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(web::scope("/product").configure(router::product::config))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
