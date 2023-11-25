use anyhow::{Context, Result};
use rustls::{Certificate, PrivateKey};
use std::{
    error::Error,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};
use tracing::info;

pub fn load_cert_key(cert_path: &Path, key_path: &Path) -> Result<(Vec<Certificate>, PrivateKey)> {
    let (cert, key) = match (cert_path.exists(), key_path.exists()) {
        (true, true) => (
            load_certificates_from_pem(cert_path)?,
            load_private_key_from_pem(key_path).unwrap(),
        ),
        _ => generate_cert_key_in_pem(None)?,
    };

    Ok((cert, key))
}

pub fn generate_cert_key_in_pem(dir_path: Option<&Path>) -> Result<(Vec<Certificate>, PrivateKey)> {
    info!("generating self-signed certificate");

    let certificate: rcgen::Certificate =
        rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert: String = certificate.serialize_pem().unwrap();
    let key: String = certificate.serialize_private_key_pem();

    let path: &Path = dir_path.unwrap_or(Path::new("./asset"));
    let cert_path: PathBuf = path.join("localhost.crt");
    let key_path: PathBuf = path.join("localhost.key");
    info!("cert_path: {:?}", cert_path);
    info!("key_path: {:?}", key_path);

    fs::create_dir_all(path).context("failed to create certificate directory")?;
    fs::write(&cert_path, cert.as_bytes()).context("failed to write certificate")?;
    fs::write(&key_path, key.as_bytes()).context("failed to write private key")?;
    info!("Certificate generated");

    Ok((
        load_certificates_from_pem(&cert_path).unwrap(),
        load_private_key_from_pem(&key_path).unwrap(),
    ))
}

pub fn load_certificates_from_pem(path: &Path) -> Result<Vec<Certificate>> {
    let file: File = File::open(path)?;
    let mut reader: BufReader<File> = BufReader::new(file);
    let certs: Vec<Vec<u8>> = rustls_pemfile::certs(&mut reader)?;

    Ok(certs.into_iter().map(Certificate).collect())
}

pub fn load_private_key_from_pem(path: &Path) -> Result<PrivateKey, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?;
    // let mut keys = rustls_pemfile::rsa_private_keys(&mut reader)?;

    match keys.len() {
        0 => Err(format!("No PKCS8-encoded private key found in {:?}", path).into()),
        1 => Ok(PrivateKey(keys.remove(0))),
        _ => Err(format!(
            "More than one PKCS8-encoded private key found in {:?}",
            path
        )
        .into()),
    }
}
