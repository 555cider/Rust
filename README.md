### What this is
- my records of testing web transport.


### How to test HTTPS in localhost
- choose one of the tools below.

1. mkcert
    - Read [this](https://web.dev/articles/how-to-use-local-https?hl=ko). In brief,
    1. Create the root CA.
    2. Create the server certificate.

2. openssl
    - Read [this](https://hackernoon.com/how-to-get-sslhttps-for-localhost-i11s3342). In brief,
    1. Create the root CA key.
    2. Create the root CA cerificate.
    3. Create the server key.
    4. Create the server CSR.
    4. Create the server certificate.

### How to run
```bash
cargo run -bin server
```
or
```bash
cargo run -bin server -c {server cerificate path} -k {server key path}
```