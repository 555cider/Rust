use anyhow::{Context, Result};
use clap::Parser;
use clap::builder::PossibleValuesParser;
use ipnet::IpNet;
use log::{LevelFilter, debug, error, info, warn};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, lookup_host};
use tokio::signal::ctrl_c;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

// SOCKS5 protocol constants
const SOCKS_VERSION: u8 = 5;
const CONNECT_COMMAND: u8 = 1;
const BIND_COMMAND: u8 = 2;
const UDP_ASSOCIATE_COMMAND: u8 = 3;
const NO_AUTH_METHOD: u8 = 0;
const USER_PASS_AUTH_METHOD: u8 = 2;
const ADDR_TYPE_IPV4: u8 = 1;
const ADDR_TYPE_DOMAIN: u8 = 3;
const ADDR_TYPE_IPV6: u8 = 4;
const REPLY_SUCCEEDED: u8 = 0;

// SOCKS5 error reply codes (refer to RFC 1928)
const REPLY_GENERAL_FAILURE: u8 = 0x01;
const REPLY_CONNECTION_REFUSED: u8 = 0x05;
const REPLY_TTL_EXPIRED: u8 = 0x06; // TTL expired (defined but not used)
const REPLY_COMMAND_NOT_SUPPORTED: u8 = 0x07;
const REPLY_ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;
const REPLY_HOST_UNREACHABLE: u8 = 0x04;

#[derive(Debug)]
enum ProxyError {
    InvalidVersion,
    AuthenticationRequired,
    AuthenticationFailed,
    UnsupportedCommand,
    UnsupportedAddressType,
    ConnectionFailed(String),
    NetworkError(String),
    Timeout(String),
    ConfigError(String),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyError::InvalidVersion => write!(f, "Unsupported SOCKS version"),
            ProxyError::AuthenticationRequired => write!(f, "Authentication required"),
            ProxyError::AuthenticationFailed => write!(f, "Authentication failed"),
            ProxyError::UnsupportedCommand => write!(f, "Unsupported command"),
            ProxyError::UnsupportedAddressType => write!(f, "Unsupported address type"),
            ProxyError::ConnectionFailed(details) => write!(f, "Connection failed: {}", details),
            ProxyError::NetworkError(details) => write!(f, "Network error: {}", details),
            ProxyError::Timeout(details) => write!(f, "Timeout: {}", details),
            ProxyError::ConfigError(details) => write!(f, "Configuration error: {}", details),
        }
    }
}

impl std::error::Error for ProxyError {}

// Command-line argument parsing
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// IP address to bind to (IPv4 or IPv6)
    #[clap(long, default_value = "127.0.0.1")]
    bind_ip: String,

    /// Port to bind to
    #[clap(long, default_value = "1080")]
    bind_port: u16,

    /// Maximum number of concurrent connections
    #[clap(long, default_value = "1000")]
    max_connections: usize,

    /// Connection timeout (seconds)
    #[clap(long, default_value = "60")]
    timeout_seconds: u64,

    /// Whether to use authentication
    #[clap(long)]
    use_auth: bool,

    /// Path to authentication file (lines in username:password format)
    #[clap(long, default_value = "auth.txt")]
    auth_file: String,

    /// Log level setting
    #[clap(long, default_value = "info", value_parser = PossibleValuesParser::new(&["error", "warn", "info", "debug", "trace"]))]
    log_level: String,

    /// Log file path (if not specified, output to console only)
    #[clap(long)]
    log_file: Option<PathBuf>,

    /// List of allowed IP addresses or CIDR ranges (comma-separated)
    #[clap(long)]
    allowed_ips: Option<String>,

    /// Whether to enable TLS
    #[clap(long)]
    use_tls: bool,

    /// Path to TLS certificate file
    #[clap(long)]
    tls_cert: Option<PathBuf>,

    /// Path to TLS key file
    #[clap(long)]
    tls_key: Option<PathBuf>,

    /// DNS lookup cache TTL (seconds, 0 to disable cache)
    #[clap(long, default_value = "300")]
    dns_cache_ttl: u64,
}

// Target address information
struct TargetAddress {
    ip: IpAddr,
    domain: Option<String>,
    port: u16,
}

// Connection statistics tracking
struct Stats {
    active_connections: AtomicUsize,
    total_connections: AtomicUsize,
    start_time: Instant,
}

// User authentication information storage
struct Users {
    credentials: HashMap<String, String>,
}

// IP allow list
struct AllowedIPs {
    networks: Vec<IpNet>,
}

impl AllowedIPs {
    fn new(allowed_ips: &str) -> Result<Self, ipnet::AddrParseError> {
        let mut networks = Vec::new();

        for ip_str in allowed_ips.split(',') {
            let ip_str = ip_str.trim();
            if !ip_str.is_empty() {
                let network = IpNet::from_str(ip_str)?;
                networks.push(network);
            }
        }

        Ok(AllowedIPs { networks })
    }

    fn is_allowed(&self, addr: &IpAddr) -> bool {
        if self.networks.is_empty() {
            return true; // Allow all IPs if the list is empty
        }

        self.networks.iter().any(|network| network.contains(addr))
    }
}

// Simple DNS cache implementation
struct DnsCache {
    cache: Mutex<HashMap<String, (IpAddr, Instant)>>,
    ttl: Duration,
}

impl DnsCache {
    fn new(ttl_seconds: u64) -> Self {
        DnsCache {
            cache: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    fn get(&self, domain: &str) -> Option<IpAddr> {
        if self.ttl.as_secs() == 0 {
            return None; // Cache disabled
        }

        let mut cache = self.cache.lock().unwrap();
        if let Some((ip, timestamp)) = cache.get(domain) {
            if timestamp.elapsed() < self.ttl {
                return Some(*ip);
            }
            // TTL expired, remove entry
            cache.remove(domain);
        }
        None
    }

    fn set(&self, domain: String, ip: IpAddr) {
        if self.ttl.as_secs() == 0 {
            return; // Cache disabled
        }

        let mut cache = self.cache.lock().unwrap();
        cache.insert(domain, (ip, Instant::now()));
    }
}

// Logging setup function
fn setup_logging(args: &Args) -> Result<()> {
    let log_level = match args.log_level.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] {m}{n}",
        )))
        .build();

    let mut config_builder =
        Config::builder().appender(Appender::builder().build("stdout", Box::new(stdout)));

    let mut root_builder = Root::builder().appender("stdout");

    // File logging setup (if specified)
    if let Some(log_file_path) = &args.log_file {
        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S)} [{l}] {m}{n}",
            )))
            .build(log_file_path)
            .context(format!("Failed to create log file: {:?}", log_file_path))?;

        config_builder =
            config_builder.appender(Appender::builder().build("logfile", Box::new(logfile)));
        root_builder = root_builder.appender("logfile");
    }

    let config = config_builder
        .build(root_builder.build(log_level))
        .context("Failed to build log configuration")?;

    log4rs::init_config(config).context("Failed to initialize logging")?;

    Ok(())
}

// TLS setup function
async fn setup_tls(args: &Args) -> Result<TlsAcceptor> {
    if args.tls_cert.is_none() || args.tls_key.is_none() {
        return Err(anyhow::anyhow!(
            "TLS enabled but certificate or key file not specified"
        ));
    }

    let cert_file = File::open(args.tls_cert.as_ref().unwrap()).context(format!(
        "Failed to open TLS certificate file: {:?}",
        args.tls_cert
    ))?;
    let key_file = File::open(args.tls_key.as_ref().unwrap())
        .context(format!("Failed to open TLS key file: {:?}", args.tls_key))?;

    let cert_chain: Vec<CertificateDer> = certs(&mut BufReader::new(cert_file))
        .context("Failed to parse TLS certificate")?
        .into_iter()
        .map(CertificateDer::from)
        .collect();

    let mut keys: Vec<PrivatePkcs8KeyDer> = pkcs8_private_keys(&mut BufReader::new(key_file))
        .context("Failed to parse TLS key")?
        .into_iter()
        .map(PrivatePkcs8KeyDer::from)
        .collect();

    if keys.is_empty() {
        return Err(anyhow::anyhow!("No private key found in key file"));
    }

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, PrivateKeyDer::from(keys.remove(0)))
        .context("Failed to build TLS configuration")?;

    Ok(TlsAcceptor::from(Arc::new(config)))
}

async fn handle_client<T>(
    mut socket: T,
    addr: SocketAddr,
    users: Option<Arc<Users>>,
    timeout_duration: Duration,
    dns_cache: Arc<DnsCache>,
) -> Result<()>
where
    T: AsyncRead + AsyncWrite + Unpin + Send,
{
    debug!("Processing new client: {}", addr);

    // --- Authentication method negotiation ---
    let mut buf = [0u8; 2];
    socket
        .read_exact(&mut buf)
        .await
        .context("Failed to read authentication header")?;

    if buf[0] != SOCKS_VERSION {
        return Err(ProxyError::InvalidVersion.into());
    }

    let method_count = buf[1] as usize;
    let mut methods = vec![0u8; method_count];
    socket
        .read_exact(&mut methods)
        .await
        .context("Failed to read authentication methods")?;

    // Authentication processing
    if let Some(users_data) = &users {
        if !methods.contains(&USER_PASS_AUTH_METHOD) {
            socket
                .write_all(&[SOCKS_VERSION, 0xFF])
                .await
                .context("Failed to send auth rejection")?;
            return Err(ProxyError::AuthenticationRequired.into());
        }

        // Select user/password authentication method
        socket
            .write_all(&[SOCKS_VERSION, USER_PASS_AUTH_METHOD])
            .await
            .context("Failed to send auth method")?;

        // Authentication process (sub-negotiation version 0x01)
        let mut auth_header = [0u8; 2];
        socket
            .read_exact(&mut auth_header)
            .await
            .context("Failed to read auth header")?;

        if auth_header[0] != 1 {
            return Err(anyhow::anyhow!("Unsupported auth version"));
        }

        // Read username
        let ulen = auth_header[1] as usize;
        let mut username = vec![0u8; ulen];
        socket
            .read_exact(&mut username)
            .await
            .context("Failed to read username")?;
        let username = String::from_utf8(username).context("Username is not valid UTF-8")?;

        // Read password
        let mut plen = [0u8; 1];
        socket
            .read_exact(&mut plen)
            .await
            .context("Failed to read password length")?;
        let mut password = vec![0u8; plen[0] as usize];
        socket
            .read_exact(&mut password)
            .await
            .context("Failed to read password")?;
        let password = String::from_utf8(password).context("Password is not valid UTF-8")?;

        // Authentication verification
        let authenticated = users_data.credentials.get(&username) == Some(&password);

        if authenticated {
            socket
                .write_all(&[1, 0])
                .await
                .context("Failed to send auth success")?; // Success
            debug!("{} authenticated as {}", addr, username);
        } else {
            socket
                .write_all(&[1, 1])
                .await
                .context("Failed to send auth failure")?; // Failure
            return Err(ProxyError::AuthenticationFailed.into());
        }
    } else {
        // No authentication
        if !methods.contains(&NO_AUTH_METHOD) {
            socket
                .write_all(&[SOCKS_VERSION, 0xFF])
                .await
                .context("Failed to send no-auth rejection")?;
            return Err(anyhow::anyhow!("No supported authentication methods"));
        }

        socket
            .write_all(&[SOCKS_VERSION, NO_AUTH_METHOD])
            .await
            .context("Failed to send no-auth acceptance")?;
    }

    // --- Request processing ---
    let mut request = [0u8; 4];
    socket
        .read_exact(&mut request)
        .await
        .context("Failed to read request header")?;

    if request[0] != SOCKS_VERSION {
        return Err(ProxyError::InvalidVersion.into());
    }

    // Command processing
    match request[1] {
        CONNECT_COMMAND => {
            // Parse target address
            let target_addr = match request[3] {
                ADDR_TYPE_IPV4 => {
                    let mut ipv4 = [0u8; 4];
                    socket
                        .read_exact(&mut ipv4)
                        .await
                        .context("Failed to read IPv4 address")?;
                    let ip = IpAddr::V4(Ipv4Addr::from(ipv4));

                    // Read port
                    let mut port_buf = [0u8; 2];
                    socket
                        .read_exact(&mut port_buf)
                        .await
                        .context("Failed to read port")?;
                    let port = u16::from_be_bytes(port_buf);

                    TargetAddress {
                        ip,
                        domain: None,
                        port,
                    }
                }
                ADDR_TYPE_DOMAIN => {
                    let mut domain_length = [0u8; 1];
                    socket
                        .read_exact(&mut domain_length)
                        .await
                        .context("Failed to read domain length")?;
                    let domain_len = domain_length[0] as usize;
                    let mut domain = vec![0u8; domain_len];
                    socket
                        .read_exact(&mut domain)
                        .await
                        .context("Failed to read domain name")?;
                    let domain_str =
                        String::from_utf8(domain).context("Domain is not valid UTF-8")?;
                    debug!("Resolving domain: {}", domain_str);

                    // Read port
                    let mut port_buf = [0u8; 2];
                    socket
                        .read_exact(&mut port_buf)
                        .await
                        .context("Failed to read port")?;
                    let port = u16::from_be_bytes(port_buf);

                    // Check DNS cache
                    let ip = if let Some(cached_ip) = dns_cache.get(&domain_str) {
                        debug!("DNS cache hit for {}: {}", domain_str, cached_ip);
                        cached_ip
                    } else {
                        // Cache miss, perform DNS lookup
                        match timeout(
                            Duration::from_secs(5),
                            lookup_host((domain_str.as_str(), 0)),
                        )
                        .await
                        {
                            Ok(Ok(mut addrs)) => {
                                if let Some(addr) = addrs.next() {
                                    let ip = addr.ip();
                                    debug!("Resolved {} to {}", domain_str, ip);

                                    // Store in cache
                                    dns_cache.set(domain_str.clone(), ip);
                                    ip
                                } else {
                                    send_reply(&mut socket, REPLY_HOST_UNREACHABLE)
                                        .await
                                        .context("Failed to send host unreachable reply")?;
                                    return Err(ProxyError::ConnectionFailed(format!(
                                        "Domain resolution failed: {}",
                                        domain_str
                                    ))
                                    .into());
                                }
                            }
                            Ok(Err(e)) => {
                                send_reply(&mut socket, REPLY_HOST_UNREACHABLE)
                                    .await
                                    .context("Failed to send host unreachable reply")?;
                                return Err(ProxyError::ConnectionFailed(format!(
                                    "Domain resolution error: {}",
                                    e
                                ))
                                .into());
                            }
                            Err(_) => {
                                send_reply(&mut socket, REPLY_HOST_UNREACHABLE)
                                    .await
                                    .context("Failed to send host unreachable reply")?;
                                return Err(ProxyError::Timeout(format!(
                                    "Domain resolution timeout: {}",
                                    domain_str
                                ))
                                .into());
                            }
                        }
                    };

                    TargetAddress {
                        ip,
                        domain: Some(domain_str),
                        port,
                    }
                }
                ADDR_TYPE_IPV6 => {
                    let mut ipv6 = [0u8; 16];
                    socket
                        .read_exact(&mut ipv6)
                        .await
                        .context("Failed to read IPv6 address")?;
                    let ip = IpAddr::V6(Ipv6Addr::from(ipv6));

                    // Read port
                    let mut port_buf = [0u8; 2];
                    socket
                        .read_exact(&mut port_buf)
                        .await
                        .context("Failed to read port")?;
                    let port = u16::from_be_bytes(port_buf);

                    TargetAddress {
                        ip,
                        domain: None,
                        port,
                    }
                }
                _ => {
                    send_reply(&mut socket, REPLY_ADDRESS_TYPE_NOT_SUPPORTED)
                        .await
                        .context("Failed to send address type not supported reply")?;
                    return Err(ProxyError::UnsupportedAddressType.into());
                }
            };

            // Log remote server connection information
            if let Some(domain) = &target_addr.domain {
                info!(
                    "{} connecting to {}:{} ({})",
                    addr, domain, target_addr.port, target_addr.ip
                );
            } else {
                info!(
                    "{} connecting to {}:{}",
                    addr, target_addr.ip, target_addr.port
                );
            }

            // Connect to remote server (with timeout)
            let mut remote = match timeout(
                timeout_duration,
                TcpStream::connect((target_addr.ip, target_addr.port)),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    if let Some(domain) = &target_addr.domain {
                        info!(
                            "{} connected to {}:{} ({})",
                            addr, domain, target_addr.port, target_addr.ip
                        );
                    } else {
                        info!(
                            "{} connected to {}:{}",
                            addr, target_addr.ip, target_addr.port
                        );
                    }
                    stream
                }
                Ok(Err(e)) => {
                    // Send appropriate response code for connection failure
                    let reply_code = match e.kind() {
                        std::io::ErrorKind::ConnectionRefused => REPLY_CONNECTION_REFUSED,
                        std::io::ErrorKind::TimedOut => REPLY_HOST_UNREACHABLE,
                        _ => REPLY_GENERAL_FAILURE,
                    };
                    send_reply(&mut socket, reply_code)
                        .await
                        .context("Failed to send connection error reply")?;
                    return Err(ProxyError::ConnectionFailed(format!(
                        "Failed to connect to remote: {}",
                        e
                    ))
                    .into());
                }
                Err(_) => {
                    send_reply(&mut socket, REPLY_HOST_UNREACHABLE)
                        .await
                        .context("Failed to send timeout reply")?;

                    let target_info = if let Some(domain) = &target_addr.domain {
                        format!("{}:{} ({})", domain, target_addr.port, target_addr.ip)
                    } else {
                        format!("{}:{}", target_addr.ip, target_addr.port)
                    };

                    return Err(ProxyError::Timeout(format!(
                        "Connection timeout to {}",
                        target_info
                    ))
                    .into());
                }
            };

            // Send successful connection response
            send_reply(&mut socket, REPLY_SUCCEEDED)
                .await
                .context("Failed to send success reply")?;

            // Set up bidirectional data transfer
            let (mut ri, mut wi) = tokio::io::split(&mut socket);
            let (mut ro, mut wo) = remote.split();

            // Execute bidirectional copy (with timeout)
            match tokio::select! {
                result = tokio::io::copy(&mut ri, &mut wo) => { result.map(|_| ()) },
                result = tokio::io::copy(&mut ro, &mut wi) => { result.map(|_| ()) }
            } {
                Ok(_) => {
                    if let Some(domain) = &target_addr.domain {
                        info!(
                            "{} closed connection to {}:{} ({})",
                            addr, domain, target_addr.port, target_addr.ip
                        );
                    } else {
                        info!(
                            "{} closed connection to {}:{}",
                            addr, target_addr.ip, target_addr.port
                        );
                    }
                }
                Err(e) => {
                    let target_info = if let Some(domain) = &target_addr.domain {
                        format!("{}:{} ({})", domain, target_addr.port, target_addr.ip)
                    } else {
                        format!("{}:{}", target_addr.ip, target_addr.port)
                    };

                    warn!("{} error with {} - {}", addr, target_info, e);
                    return Err(
                        ProxyError::NetworkError(format!("Data transfer error: {}", e)).into(),
                    );
                }
            }

            Ok(())
        }
        BIND_COMMAND => {
            // BIND command implementation is omitted
            warn!("{} received BIND command, but it is not implemented.", addr);
            send_reply(&mut socket, REPLY_COMMAND_NOT_SUPPORTED)
                .await
                .context("Failed to send command not supported reply")?;
            Err(ProxyError::UnsupportedCommand.into())
        }
        UDP_ASSOCIATE_COMMAND => {
            // UDP ASSOCIATE command implementation is omitted
            warn!(
                "{} received UDP ASSOCIATE command, but it is not implemented.",
                addr
            ); // Log added
            send_reply(&mut socket, REPLY_COMMAND_NOT_SUPPORTED)
                .await
                .context("Failed to send command not supported reply")?;
            Err(ProxyError::UnsupportedCommand.into())
        }
        _ => {
            send_reply(&mut socket, REPLY_COMMAND_NOT_SUPPORTED)
                .await
                .context("Failed to send unknown command reply")?;
            Err(ProxyError::UnsupportedCommand.into())
        }
    }
}

// SOCKS5 response sending helper function
async fn send_reply<T>(socket: &mut T, reply_code: u8) -> Result<()>
where
    T: AsyncWriteExt + Unpin,
{
    // Standard SOCKS5 response format: VER, REP, RSV, ATYP, BIND.ADDR, BIND.PORT
    socket
        .write_all(&[
            SOCKS_VERSION,
            reply_code,
            0x00, // Reserved field must be 0x00
            ADDR_TYPE_IPV4,
            0,
            0,
            0,
            0, // IP address (0.0.0.0)
            0,
            0, // Port (0)
        ])
        .await
        .context("Failed to send SOCKS5 reply")?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Command-line argument parsing
    let args = Args::parse();

    // Logging setup
    setup_logging(&args)?;

    // TLS setup
    let tls_acceptor = if args.use_tls {
        Some(setup_tls(&args).await.context("Failed to setup TLS")?)
    } else {
        None
    };

    // Load user authentication information (if needed)
    let users = if args.use_auth {
        let content = tokio::fs::read_to_string(&args.auth_file)
            .await
            .context(format!("Failed to read auth file: {}", args.auth_file))?;

        let mut credentials = HashMap::new();
        for line in content.lines() {
            if let Some((username, password)) = line.split_once(':') {
                credentials.insert(username.to_string(), password.to_string());
            }
        }

        Some(Arc::new(Users { credentials }))
    } else {
        None
    };

    // IP allow list setup
    let allowed_ips = if let Some(ip_list) = &args.allowed_ips {
        match AllowedIPs::new(ip_list) {
            Ok(allowed) => Some(Arc::new(allowed)),
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to parse allowed IPs: {}", e));
            }
        }
    } else {
        None
    };

    // DNS cache initialization
    let dns_cache = Arc::new(DnsCache::new(args.dns_cache_ttl));

    // Statistics information initialization
    let stats = Arc::new(Stats {
        active_connections: AtomicUsize::new(0),
        total_connections: AtomicUsize::new(0),
        start_time: Instant::now(),
    });

    // Server start
    let bind_addr = format!("{}:{}", args.bind_ip, args.bind_port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .context(format!("Failed to bind to {}", bind_addr))?;

    info!("SOCKS5 proxy running on {}", bind_addr);
    info!(
        "Authentication: {}",
        if users.is_some() {
            "Enabled"
        } else {
            "Disabled"
        }
    );
    info!("Max connections: {}", args.max_connections);
    info!("Connection timeout: {} seconds", args.timeout_seconds);
    info!(
        "DNS cache: {}",
        if args.dns_cache_ttl > 0 {
            format!("Enabled (TTL: {} seconds)", args.dns_cache_ttl)
        } else {
            "Disabled".to_string()
        }
    );

    if let Some(ips) = &allowed_ips {
        info!("IP restrictions enabled with {} rules", ips.networks.len());
    }

    if tls_acceptor.is_some() {
        info!("TLS encryption enabled");
    }

    // Channel creation for Graceful Shutdown
    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let shutdown_tx_clone = shutdown_tx.clone();

    // Ctrl+C signal handler
    tokio::spawn(async move {
        match ctrl_c().await {
            Ok(()) => {
                info!("Shutdown signal received, initiating graceful shutdown...");
                let _ = shutdown_tx_clone.send(());
            }
            Err(err) => {
                error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    // Periodic statistics reporting task
    let stats_clone = Arc::clone(&stats);
    let mut shutdown_rx = shutdown_tx.subscribe();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!(
                        "Stats: Active={}, Total={}, Uptime={}s",
                        stats_clone.active_connections.load(Ordering::Relaxed),
                        stats_clone.total_connections.load(Ordering::Relaxed),
                        stats_clone.start_time.elapsed().as_secs()
                    );
                }
                _ = shutdown_rx.recv() => {
                    debug!("Stats reporter received shutdown signal");
                    break;
                }
            }
        }
    });

    // Connection processing loop
    let mut active_tasks = JoinSet::new();
    let mut shutdown_rx = shutdown_tx.subscribe();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((socket, addr)) => {
                        // Check maximum number of connections
                        let active_count = stats.active_connections.load(Ordering::Relaxed);
                        if active_count >= args.max_connections {
                            warn!("Max connections ({}) reached, rejecting {}", args.max_connections, addr);
                            continue;
                        }

                        stats.active_connections.fetch_add(1, Ordering::SeqCst);
                        stats.total_connections.fetch_add(1, Ordering::SeqCst);

                        // Check IP restriction
                        if let Some(allowed) = &allowed_ips {
                            if !allowed.is_allowed(&addr.ip()) {
                                warn!("Connection from {} rejected (not in allowed IPs)", addr);
                                stats.active_connections.fetch_sub(1, Ordering::SeqCst);
                                continue;
                            }
                        }

                        // Connection processing
                        let users_clone = users.clone();
                        let stats_clone = Arc::clone(&stats);
                        let dns_cache_clone = Arc::clone(&dns_cache);
                        let timeout_duration = Duration::from_secs(args.timeout_seconds);
                        let tls_acceptor_clone = tls_acceptor.clone();
                        let mut shutdown_rx_task = shutdown_tx.subscribe();

                        debug!("New connection from {} (active: {})", addr,
                               stats.active_connections.load(Ordering::Relaxed));

                        active_tasks.spawn(async move {
                            let result = if let Some(tls) = tls_acceptor_clone {
                                match timeout(timeout_duration, tls.accept(socket)).await {
                                    Ok(Ok(tls_stream)) => {
                                        debug!("TLS handshake completed with {}", addr);
                                        tokio::select! {
                                            result = handle_client(tls_stream, addr, users_clone, timeout_duration, dns_cache_clone) => {
                                                result
                                            }
                                            _ = shutdown_rx_task.recv() => {
                                                info!("Client handler for {} received shutdown signal", addr);
                                                return Ok(()); // Shutdown signal received, gracefully exit
                                            }
                                        }
                                    },
                                    Ok(Err(e)) => {
                                        error!("TLS handshake failed with {}: {}", addr, e);
                                        Err(ProxyError::NetworkError(format!("TLS handshake error: {}", e)).into())
                                    },
                                    Err(_) => {
                                        error!("TLS handshake with {} timed out", addr);
                                        Err(ProxyError::Timeout("TLS handshake timeout".to_string()).into())
                                    }
                                }
                            } else {
                                tokio::select! {
                                    result = handle_client(socket, addr, users_clone, timeout_duration, dns_cache_clone) => {
                                        result
                                    }
                                    _ = shutdown_rx_task.recv() => {
                                        info!("Client handler for {} received shutdown signal", addr);
                                        return Ok(()); // Shutdown signal received, gracefully exit
                                    }
                                }
                            };

                            // Decrease active connection count upon connection closure
                            let prev_count = stats_clone.active_connections.fetch_sub(1, Ordering::SeqCst);

                            if let Err(ref e) = result {
                                error!("SOCKS5 error from {}: {} (active: {})",
                                      addr, e, prev_count - 1);
                            } else {
                                debug!("Connection from {} closed successfully (active: {})",
                                      addr, prev_count - 1);
                            }
                            result
                        });
                    }
                    Err(e) => {
                        error!("Error accepting connection: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Main loop received shutdown signal");
                break;
            }
        }
    }

    // Wait for all active connections to complete
    info!(
        "Waiting for {} active connections to complete...",
        active_tasks.len()
    );

    // Wait for all tasks to complete using JoinSet
    while let Some(result) = active_tasks.join_next().await {
        match result {
            Ok(Ok(_)) => debug!("Task completed successfully"),
            Ok(Err(e)) => error!("Task completed with error: {}", e),
            Err(e) => error!("Task join error: {}", e),
        }
    }

    info!("SOCKS5 proxy shutdown completed");
    Ok(())
}
