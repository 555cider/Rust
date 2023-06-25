#[derive(Debug)]
pub enum GatewayBody {
    Empty,
    Incoming(hyper::body::Incoming),
}

impl hyper::body::Body for GatewayBody {
    type Data = hyper::body::Bytes;
    type Error = GatewayError;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::option::Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>>
    {
        match &mut *self.get_mut() {
            Self::Empty => std::task::Poll::Ready(None),
            Self::Incoming(incoming) => std::pin::Pin::new(incoming)
                .poll_frame(cx)
                .map_err(|err| GatewayError::Done),
        }
    }
}

#[derive(Debug)]
pub enum GatewayError {
    HttpStatus(http::StatusCode),
    Done,
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let printable = match self {
            GatewayError::HttpStatus(code) => format!(
                "[{} {}]",
                code.as_u16(),
                code.canonical_reason().unwrap_or("Unknown")
            ),
            GatewayError::Done => format!("Request finished early"),
        };
        write!(f, "{}", printable)
    }
}

impl std::error::Error for GatewayError {}
unsafe impl Send for GatewayError {}
unsafe impl Sync for GatewayError {}
