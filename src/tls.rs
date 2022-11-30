use anyhow::{ensure, Context, Result};
use rustls::{Certificate, PrivateKey, RootCertStore, ClientConfig, ServerName};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio_rustls::TlsConnector;

pub async fn connect(host: &str, port: u16) -> Result<TlsStream<TcpStream>> {
    let server_name = ServerName::try_from(host)?;
    let tls_connector = create_connector()?;
    let conn = TcpStream::connect((host, port)).await?;
    let conn = tls_connector.connect(server_name, conn).await?;

    Ok(conn)
}

pub fn create_connector() -> Result<TlsConnector> {
    let root_certs = load_root_certs()?;
    let certs = load_certs()?;
    let key = load_key()?;

    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_certs)
        .with_single_cert(certs, key)
        .context("bad certificate/key")?;

    let tls_connector = TlsConnector::from(Arc::new(config));

    Ok(tls_connector)
}

pub fn load_root_certs() -> Result<RootCertStore> {
    let certs = File::open("data/ca.pem")?;
    let mut certs = BufReader::new(certs);
    let certs = rustls_pemfile::certs(&mut certs)?;

    let mut root_certs = RootCertStore::empty();

    root_certs.add_parsable_certificates(&certs);

    Ok(root_certs)
}

pub fn load_certs() -> Result<Vec<Certificate>> {
    let certs = File::open("data/client.pem")?;
    let mut certs = BufReader::new(certs);
    let certs = rustls_pemfile::certs(&mut certs)?;
    let certs = certs.into_iter().map(Certificate).collect::<Vec<_>>();

    Ok(certs)
}

pub fn load_key() -> Result<PrivateKey> {
    let keys = File::open("data/client.key")?;
    let mut keys = BufReader::new(keys);
    let mut keys = rustls_pemfile::rsa_private_keys(&mut keys)?;
    let key = keys.pop().context("certificate key missing")?;

    ensure!(keys.is_empty(), "can not use more than one certificate key");

    Ok(PrivateKey(key))
}
