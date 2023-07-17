use std::{
    error::Error,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug)]
pub enum GatewayBody {
    Empty,
    Incoming(hyper::body::Incoming),
}

impl hyper::body::Body for GatewayBody {
    type Data = hyper::body::Bytes;
    type Error = GatewayError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        match &mut *self.get_mut() {
            Self::Empty => Poll::Ready(None),
            Self::Incoming(incoming) => Pin::new(incoming)
                .poll_frame(cx)
                .map_err(|err: hyper::Error| GatewayError::from(err)),
        }
    }
}

pub type GatewayError = Box<dyn Error + Send + Sync + 'static>;

pub fn get_gateway_response(
    status_code: http::StatusCode,
) -> Result<http::Response<GatewayBody>, http::Error> {
    let response: Result<http::Response<GatewayBody>, http::Error> = http::Response::builder()
        .status(status_code)
        .body(GatewayBody::Empty);
    log::info!("response = {:?}", response);
    return response;
}
