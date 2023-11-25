use anyhow::{Context, Result};
use bytes::{BufMut, Bytes, BytesMut};
use h3::ext;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{error, info};

use crate::constant::{CHUNK_SIZE, SLEEP_DURATION_IN_MILLIS};

pub async fn handle_connection(
    mut conn: sec_http3::server::Connection<sec_http3::sec_http3_quinn::Connection, Bytes>,
) -> Result<()> {
    loop {
        match conn.accept().await {
            Ok(Some((req, stream))) => {
                info!("new request: {:#?}", req);

                let ext = http::request::Request::extensions(&req);
                match req.method() {
                    &http::Method::CONNECT
                        if ext.get::<ext::Protocol>() == Some(&ext::Protocol::WEB_TRANSPORT) =>
                    {
                        info!("Peer wants to initiate a webtransport session");

                        let session = sec_http3::webtransport::server::WebTransportSession::accept(
                            req, stream, conn,
                        )
                        .await?;
                        info!("Established webtransport session");

                        handle_session_and_echo_all_inbound_messages(session).await?;

                        return Ok(());
                    }
                    _ => {
                        info!(?req, "Received request");
                    }
                }
            }

            Ok(None) => {
                break;
            }

            Err(err) => {
                error!("Error on accept {}", err);
                match err.get_error_level() {
                    sec_http3::error::ErrorLevel::ConnectionError => break,
                    sec_http3::error::ErrorLevel::StreamError => continue,
                }
            }
        }
    }
    Ok(())
}

macro_rules! log_result {
    ($expr:expr) => {
        if let Err(err) = $expr {
            error!("{err:?}");
        }
    };
}

async fn echo_stream<T, R>(send: T, recv: R) -> anyhow::Result<()>
where
    T: AsyncWrite,
    R: AsyncRead,
{
    tokio::pin!(send);
    tokio::pin!(recv);

    info!("Got stream");
    let mut buf: Vec<u8> = Vec::new();
    AsyncReadExt::read_to_end(&mut recv, &mut buf).await?;
    send_chunked(send, Bytes::from(buf)).await?;

    Ok(())
}

// Used to test that all chunks arrive properly as it is easy to write an impl which only reads and
// writes the first chunk.
async fn send_chunked(mut send: impl AsyncWrite + Unpin, data: Bytes) -> anyhow::Result<()> {
    for chunk in data.chunks(CHUNK_SIZE) {
        tokio::time::sleep(Duration::from_millis(SLEEP_DURATION_IN_MILLIS)).await;

        info!("Sending {chunk:?}");
        AsyncWriteExt::write_all(&mut send, chunk).await?;
    }

    Ok(())
}

async fn open_bidi_test<S>(mut stream: S) -> anyhow::Result<()>
where
    S: Unpin + AsyncRead + AsyncWrite,
{
    info!("Opening bidirectional stream");

    AsyncWriteExt::write_all(&mut stream, b"Hello from a server initiated bidi stream")
        .await
        .context("Failed to respond")?;

    let mut resp = Vec::new();
    AsyncWriteExt::shutdown(&mut stream).await?;
    AsyncReadExt::read_to_end(&mut stream, &mut resp).await?;
    info!("Got response from client: {resp:?}");

    Ok(())
}

async fn handle_session_and_echo_all_inbound_messages<C>(
    session: sec_http3::webtransport::server::WebTransportSession<C, Bytes>,
) -> anyhow::Result<()>
where
// Use trait bounds to ensure we only happen to use implementation that are only for the quinn
// backend.
    C: 'static
        + Send
        + sec_http3::quic::Connection<Bytes>
        + sec_http3::quic::RecvDatagramExt<Buf = Bytes>
        + sec_http3::quic::SendDatagramExt<Bytes>,
    <C::SendStream as sec_http3::quic::SendStream<Bytes>>::Error:
        'static + std::error::Error + Send + Sync + Into<std::io::Error>,
    <C::RecvStream as sec_http3::quic::RecvStream>::Error:
        'static + std::error::Error + Send + Sync + Into<std::io::Error>,
    sec_http3::webtransport::stream::BidiStream<C::BidiStream, Bytes>:
        sec_http3::quic::BidiStream<Bytes> + Unpin + tokio::io::AsyncWrite + tokio::io::AsyncRead,
    <sec_http3::webtransport::stream::BidiStream<C::BidiStream, Bytes> as sec_http3::quic::BidiStream<
        Bytes,
    >>::SendStream: Unpin + tokio::io::AsyncWrite + Send + Sync,
    <sec_http3::webtransport::stream::BidiStream<C::BidiStream, Bytes> as sec_http3::quic::BidiStream<
        Bytes,
    >>::RecvStream: Unpin + tokio::io::AsyncRead + Send + Sync,
    C::SendStream: Send + Unpin,
    C::RecvStream: Send + Unpin,
    C::BidiStream: Send + Unpin,
    sec_http3::webtransport::stream::SendStream<C::SendStream, Bytes>: tokio::io::AsyncWrite,
    C::BidiStream: sec_http3::quic::SendStreamUnframed<Bytes>,
    C::SendStream: sec_http3::quic::SendStreamUnframed<Bytes>,
{
    let session_id: sec_http3::webtransport::SessionId = session.session_id();

    // This will open a bidirectional stream and send a message to the client right after connecting!
    let stream = session.open_bi(session_id).await?;

    tokio::spawn(async move { log_result!(open_bidi_test(stream).await) });
    loop {
        tokio::select! {
            datagram = session.accept_datagram() => {
                let datagram = datagram?;
                if let Some((_, datagram)) = datagram {
                    info!("Responding with {datagram:?}");
                    // Put something before to make sure encoding and decoding works and don't just
                    // pass through
                    let mut resp = BytesMut::from(&b"Response: "[..]);
                    resp.put(datagram);

                    session.send_datagram(resp.freeze())?;
                    info!("Finished sending datagram");
                }
            }
            uni_stream = session.accept_uni() => {
                let (id, stream) = uni_stream?.unwrap();

                let send = session.open_uni(id).await?;
                tokio::spawn( async move { log_result!(echo_stream(send, stream).await); });
            }
            stream = session.accept_bi() => {
                if let Some(sec_http3::webtransport::server::AcceptedBi::BidiStream(_, stream)) = stream? {
                    let (send, recv) = sec_http3::quic::BidiStream::split(stream);
                    tokio::spawn( async move { log_result!(echo_stream(send, recv).await); });
                }
            }
            else => {
                break
            }
        }
    }

    info!("Finished handling session");

    Ok(())
}
