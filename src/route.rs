use crate::config;

pub fn request_builder(
    parts: http::request::Parts,
    body: hyper::body::Incoming,
    route_config: &config::Route,
) -> Result<hyper::Request<hyper::body::Incoming>, http::Error> {
    let request: hyper::Request<hyper::body::Incoming> = http::Request::from_parts(parts, body);
    let uri: String = format!(
        "http://{}:{}{}",
        route_config.authority.host,
        route_config.authority.port,
        request.uri().path()
    );

    let mut request_builder = hyper::Request::builder()
        .uri(uri)
        .method(request.method())
        .version(request.version());
    *request_builder.headers_mut().unwrap() = request.headers().clone();

    request_builder.body(request.into_body())
}
