use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

mod config;
mod entity;
mod logger;
mod route;
mod throttle;

#[tokio::main]
async fn main() {
    logger::init_logger();
    log::info!("Initialized the logger");

    log::info!("Load the configuation");
    let gateway_config: config::GatewayConfig = config::load_config("config.yaml");
    let route_config_arr: Vec<config::Route> = gateway_config.route;

    log::info!("Initialize the throttle");
    let throttle_pool: Arc<Mutex<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>> =
        throttle::init_throttle().expect("Failed to initialize the throttle!");

    log::info!("Create the TCP listener");
    let gateway_addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let gateway_listener: tokio::net::TcpListener = tokio::net::TcpListener::bind(gateway_addr)
        .await
        .expect("Failed to create the TCP listener!");

    log::info!("Listening on http://{}", &gateway_addr);
    loop {
        tokio::task::spawn({
            let (gateway_stream, _) = gateway_listener
                .accept()
                .await
                .expect("Failed to accepts a connection from this listener!");
            route::run(
                gateway_stream,
                route_config_arr.clone(),
                throttle_pool.clone(),
            )
        });
    }
}
