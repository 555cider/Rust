# Rust SOCKS5 Proxy Server

A simple SOCKS5 proxy server written in Rust.

## Features

- SOCKS5 Protocol Support (CONNECT command)
- TLS/SSL Support (optional)
- Username/Password Authentication (optional)
- IP Filtering (optional)
- Connection/Operation Timeout Management
- DNS Caching (configurable TTL)

## Installation

### Building from source

```bash
# Clone the repository
git clone https://github.com/555cider/rust-socks5-proxy.git
cd rust-socks5-proxy

# Build in release mode
cargo build --release
```

The compiled binary will be available at `target/release/rust-socks5-proxy`.

## Usage

```bash
# Basic usage (default: 127.0.0.1:1080)
./rust-socks5-proxy

# Bind to specific address and port
./rust-socks5-proxy --bind-ip 0.0.0.0 --bind-port 8888

# Enable authentication
./rust-socks5-proxy --use-auth --auth-file users.txt

# Enable TLS/SSL
./rust-socks5-proxy --use-tls --tls-cert cert.pem --tls-key key.pem

# Restrict access by IP
./rust-socks5-proxy --allowed-ips "192.168.1.0/24,10.0.0.0/8"

# Detailed logging to file
./rust-socks5-proxy --log-level debug --log-file proxy.log
```

### Full Command-line Options

```
OPTIONS:
    --bind-ip <IP>               IP address to bind to [default: 127.0.0.1]
    --bind-port <PORT>           Port to listen on [default: 1080]
    --max-connections <NUM>      Maximum concurrent connections [default: 1000]
    --timeout-seconds <SEC>      Connection/operation timeout in seconds [default: 60]
    --use-auth                   Enable username/password authentication
    --auth-file <FILE>           Path to authentication file [default: auth.txt]
    --log-level <LEVEL>          Logging level (error, warn, info, debug, trace) [default: info]
    --log-file <FILE>            Log to file instead of console
    --allowed-ips <IP-RANGES>    Comma-separated list of allowed IP addresses or CIDR ranges
    --use-tls                    Enable TLS/SSL encryption
    --tls-cert <FILE>            Path to TLS certificate file (required with --use-tls)
    --tls-key <FILE>             Path to TLS key file (required with --use-tls)
    --dns-cache-ttl <SEC>        DNS cache time-to-live in seconds [default: 300]
    -h, --help                   Print help information
    -V, --version                Print version information
```

### Authentication

Authentication requires a file containing username:password pairs, one per line:

```txt
# auth.txt example
user1:password1
user2:password2
admin:strongpassword
```

### TLS/SSL Setup

To enable TLS/SSL, you need to provide a certificate and private key. You can generate self-signed certificates for testing purposes using OpenSSL:

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

Then, run the proxy with the --use-tls, --tls-cert, and --tls-key options:

```bash
./rust-socks5-proxy --use-tls --tls-cert cert.pem --tls-key key.pem
```

## License

[**MIT License**](LICENSE)
