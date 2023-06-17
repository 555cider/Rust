use std::sync::Arc;

mod config;
mod logger;
mod route;

#[tokio::main]
async fn main() {
    logger::init_logger();
    log::info!("Initialized the logger");

    log::info!("Load the configuation");
    let gateway_config: config::GatewayConfig = config::load_config("config.yaml");
    let route_config_array: std::sync::Arc<[config::Route]> = gateway_config.route;

    log::info!("Create the TCP listener");
    let gateway_addr: std::net::SocketAddr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    let gateway_listener: tokio::net::TcpListener = tokio::net::TcpListener::bind(gateway_addr)
        .await
        .expect("Failed to create the TCP listener!");

    log::info!("Listening on http://{}", &gateway_addr);
    loop {
        let (gateway_stream, _) = gateway_listener.accept().await.expect("Failed to accept!");
        let route_config_array = Arc::clone(&route_config_array);

        let service_fn =
            hyper::service::service_fn(move |request: http::Request<hyper::body::Incoming>| {
                log::info!("request = {:?}", &request);

                let route_config: &config::Route =
                    config::get_route(&request.uri().path(), &route_config_array)
                        .expect("Failed to get the routing configuration for the request!");

                let route_addr: std::net::SocketAddr = std::net::SocketAddr::from((
                    route_config
                        .authority
                        .host
                        .parse::<std::net::IpAddr>()
                        .unwrap(),
                    route_config.authority.port.parse::<u16>().unwrap(),
                ));
                log::info!("Routed to {}://{:?}", route_config.scheme, &route_addr);

                let (parts, body) = request.into_parts();
                let route_request: http::Request<hyper::body::Incoming> =
                    route::request_builder(parts, body, route_config)
                        .expect("Failed to create a routing request!");

                async move {
                    let route_stream: tokio::net::TcpStream =
                        tokio::net::TcpStream::connect(route_addr)
                            .await
                            .expect("Failed to open a TCP connection to route!");
                    let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
                        .handshake(route_stream)
                        .await
                        .expect("Failed to constructs a connection!");

                    tokio::task::spawn(async move {
                        if let Err(err) = conn.await {
                            log::error!("Failed to spawn a executor: {:?}", err);
                        }
                    });
                    sender.send_request(route_request).await
                }
            });

        tokio::task::spawn(async move {
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(gateway_stream, service_fn)
                .await
            {
                log::error!("Failed to bind a connection with a service: {:?}", err);
            }
        });
    }
}
