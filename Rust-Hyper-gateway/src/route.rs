use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use crate::{config, entity, throttle, trace};

pub async fn run(
    gateway_stream: tokio::net::TcpStream,
    route_config_arr_clone: Arc<[config::Route]>,
    throttle_pool_clone: Arc<Mutex<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>>,
) -> Result<(), entity::GatewayError> {
    let service_fn = hyper::service::service_fn(
        move |incoming_request: http::Request<hyper::body::Incoming>| {
            log::info!("incoming_request = {:?}", &incoming_request);

            let throttle_status: http::StatusCode = throttle::run(
                &throttle_pool_clone.lock().unwrap().get().unwrap(),
                incoming_request.headers(),
            )
            .unwrap();
            log::info!("throttle_status = {:?}", throttle_status);

            let (outgoing_request, addr) = route_request(incoming_request, &route_config_arr_clone);

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

                let incoming_response: http::Response<hyper::body::Incoming> = sender
                    .send_request(outgoing_request)
                    .await
                    .expect("Failed to send a request!");
                let outgoing_response: Result<http::Response<entity::GatewayBody>, http::Error> =
                    http::Response::builder()
                        .status(http::StatusCode::OK)
                        .body(entity::GatewayBody::Incoming(incoming_response.into_body()));
                log::info!("outgoing_response = {:?}", outgoing_response);
                outgoing_response
            }
        },
    );

    match hyper::server::conn::http1::Builder::new()
        .serve_connection(gateway_stream, service_fn)
        .await
    {
        Ok(res) => Ok(res),
        Err(err) => {
            log::error!("Failed to bind a connection with a service: {:?}", err);
            Err(entity::GatewayError::from(err))
        }
    }
}

pub fn route_request(
    incoming_request: http::Request<hyper::body::Incoming>,
    route_config_arr: &[config::Route],
) -> (http::Request<hyper::body::Incoming>, SocketAddr) {
    log::info!("incoming_request = {:?}", &incoming_request);

    let route_config: config::Route =
        config::get_route(incoming_request.uri().path(), route_config_arr)
            .expect("Failed to get the routing configuration for the request!")
            .clone();

    let route_addr: SocketAddr = SocketAddr::from((
        route_config.authority.host.parse::<IpAddr>().unwrap(),
        route_config.authority.port.parse::<u16>().unwrap(),
    ));
    log::info!("Routed to {}://{:?}", route_config.scheme, &route_addr);

    (
        build_request(incoming_request, route_config).expect("Failed to create a routing request!"),
        route_addr,
    )
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

    let traceparent: trace::Traceparent =
        trace::extract(request.headers()).expect("Failed to extract trace context!");

    let mut request_builder: http::request::Builder = hyper::Request::builder()
        .uri(uri)
        .method(request.method())
        .version(request.version());
    *request_builder.headers_mut().unwrap() = request.headers().clone();
    request_builder
        .headers_mut()
        .unwrap()
        .append("traceparent", traceparent.as_headervalue());
    request_builder.body(request.into_body())
}
