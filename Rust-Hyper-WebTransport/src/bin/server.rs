use anyhow::Result;
use clap::Parser;
use sec_http3::sec_http3_quinn::Connection;
use std::{net, path, sync::Arc, time::Duration};
use tracing::{error, info};
use yobara::constant::MAX_WEBTRANSPORT_SESSION;

#[derive(Parser, Debug)]
#[clap(name = "server")]
struct Opt {
    /// Address to listen on
    #[clap(
        short = 'l',
        long = "listen",
        default_value = "[::1]:4433",
        help = "Address to listen on"
    )]
    listen: net::SocketAddr,
    /// Root directory of the files to serve
    #[clap(
        short = 'r',
        long = "root",
        default_value = "./",
        help = "Root directory of the files to serve"
    )]
    root: path::PathBuf,
    /// TLS certificate encoded in PKCS8
    #[clap(
        short = 'c',
        long = "cert",
        default_value = "./asset/localhost.crt",
        requires = "key"
    )]
    cert: path::PathBuf,
    /// TLS private key encoded in PKCS8
    #[clap(
        short = 'k',
        long = "key",
        default_value = "./asset/localhost.key",
        requires = "cert"
    )]
    key: path::PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    yobara::log::init_logging();

    let opt: Opt = Opt::parse();
    info!("Opt: {:#?}", opt);

    if let Err(e) = run(opt).await {
        error!("ERROR: {}", e);
    }

    Ok(())
}

async fn run(opt: Opt) -> Result<(), Box<dyn std::error::Error>> {
    let (cert, key) = yobara::cert::load_cert_key(&opt.cert, &opt.key)?;

    let mut tls_config: rustls::ServerConfig = rustls::ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(cert, key)?;
    tls_config.max_early_data_size = u32::MAX;
    tls_config.alpn_protocols = vec![
        b"h3".to_vec(),
        b"h3-32".to_vec(),
        b"h3-31".to_vec(),
        b"h3-30".to_vec(),
        b"h3-29".to_vec(),
    ];

    let mut transport_config: quinn::TransportConfig = quinn::TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(2)));
    // transport_config.max_concurrent_uni_streams(0_u8.into());

    let mut server_config: quinn::ServerConfig =
        quinn::ServerConfig::with_crypto(Arc::new(tls_config));
    server_config.transport = Arc::new(transport_config);
    let endpoint: quinn::Endpoint = quinn::Endpoint::server(server_config, opt.listen)?; // Bind this endpoint to a UDP socket on the given server address.
    info!("listening on {}", endpoint.local_addr()?);

    // Start iterating over incoming connections.
    while let Some(connecting) = endpoint.accept().await {
        info!("connection incoming");

        tokio::spawn(async move {
            match connecting.await {
                Ok(connection) => {
                    info!("new http3 established");

                    let conn: sec_http3::server::Connection<Connection, bytes::Bytes> =
                        sec_http3::server::builder()
                            .enable_webtransport(true)
                            .enable_connect(true)
                            .enable_datagram(true)
                            .max_webtransport_sessions(MAX_WEBTRANSPORT_SESSION)
                            .send_grease(true)
                            .build(Connection::new(connection))
                            .await
                            .unwrap();

                    tokio::spawn(async move {
                        if let Err(err) = yobara::connection::handle_connection(conn).await {
                            error!("Failed to handle connection: {err:?}");
                        }
                    });
                }
                Err(err) => {
                    error!("accepting connection failed: {:?}", err);
                }
            }
        });
    }

    endpoint.wait_idle().await;

    Ok(())
}
