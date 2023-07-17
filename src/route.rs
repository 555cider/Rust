use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use crate::{config, entity, throttle};

pub async fn run(
    gateway_stream: tokio::net::TcpStream,
    route_config_arr_clone: Vec<config::Route>,
    throttle_pool_clone: Arc<Mutex<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>>,
) -> Result<(), entity::GatewayError> {
    let service_fn =
        hyper::service::service_fn(move |request: http::Request<hyper::body::Incoming>| {
            log::info!("request = {:?}", &request);

            let throttle_status: http::StatusCode = throttle::run(
                &throttle_pool_clone.lock().unwrap().get().unwrap(),
                &request.headers(),
            )
            .unwrap();
            log::info!("throttle_status = {:?}", throttle_status);

            let (resp, addr) = route_request(request, &route_config_arr_clone);

            async move {
                if !throttle_status.is_success() {
                    return entity::get_gateway_response(throttle_status);
                }

                let route_stream: tokio::net::TcpStream = tokio::net::TcpStream::connect(addr)
                    .await
                    .expect("Failed to open a TCP connection to route!");
                let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
                    .handshake(route_stream)
                    .await
                    .expect("Failed to construct a connection!");

                tokio::task::spawn(async move {
                    if let Err(err) = conn.await {
                        log::error!("Failed to spawn a connection: {:?}", err);
                    }
                });

                let response_body: http::Response<hyper::body::Incoming> = sender
                    .send_request(resp)
                    .await
                    .expect("Failed to send a request");
                let response: Result<http::Response<entity::GatewayBody>, http::Error> =
                    http::Response::builder()
                        .status(http::StatusCode::OK)
                        .body(entity::GatewayBody::Incoming(response_body.into_body()));
                log::info!("response = {:?}", response);
                return response;
            }
        });

    match hyper::server::conn::http1::Builder::new()
        .serve_connection(gateway_stream, service_fn)
        .await
    {
        Ok(res) => return Ok(res),
        Err(err) => {
            log::error!("Failed to bind a connection with a service: {:?}", err);
            return Err(entity::GatewayError::from(err));
        }
    }
}

pub fn route_request(
    request: http::Request<hyper::body::Incoming>,
    route_config_arr: &Vec<config::Route>,
) -> (http::Request<hyper::body::Incoming>, SocketAddr) {
    log::info!("request = {:?}", &request);

    let route_config: config::Route = config::get_route(request.uri().path(), route_config_arr)
        .expect("Failed to get the routing configuration for the request!")
        .clone();

    let route_addr: SocketAddr = SocketAddr::from((
        route_config.authority.host.parse::<IpAddr>().unwrap(),
        route_config.authority.port.parse::<u16>().unwrap(),
    ));
    log::info!("Routed to {}://{:?}", route_config.scheme, &route_addr);

    return (
        build_request(request, route_config).expect("Failed to create a routing request!"),
        route_addr,
    );
}

pub fn build_request(
    request: hyper::Request<hyper::body::Incoming>,
    route_config: config::Route,
) -> Result<hyper::Request<hyper::body::Incoming>, http::Error> {
    let uri: String = format!(
        "http://{}:{}{}",
        route_config.authority.host,
        route_config.authority.port,
        request.uri().path()
    );

    let mut request_builder: http::request::Builder = hyper::Request::builder()
        .uri(uri)
        .method(request.method())
        .version(request.version());
    *request_builder.headers_mut().unwrap() = request.headers().clone();

    request_builder.body(request.into_body())
}
