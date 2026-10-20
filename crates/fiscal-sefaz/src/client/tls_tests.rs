//! Handshakes reais em loopback, com certificados exclusivamente sintéticos.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

use super::SefazClient;
use crate::services::SefazService;
use fiscal_core::FiscalError;

const ROOT: &[u8] = include_bytes!("../../../../tests/fixtures/tls/root.pem");
const SERVER: &[u8] = include_bytes!("../../../../tests/fixtures/tls/server.pem");
const EXPIRED: &[u8] = include_bytes!("../../../../tests/fixtures/tls/expired-server.pem");
const KEY: &[u8] = include_bytes!("../../../../tests/fixtures/tls/server-key.pem");
const PFX: &[u8] = include_bytes!("../../../../tests/fixtures/tls/client-with-chain.pfx");
const LEAF_ONLY: &[u8] = include_bytes!("../../../../tests/fixtures/tls/client-without-chain.pfx");
const PASSWORD: &str = "test-only";

struct Server {
    port: u16,
    task: thread::JoinHandle<Result<String, String>>,
}

impl Server {
    fn start(expired: bool) -> Self {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(CertificateDer::from_pem_slice(ROOT).unwrap())
            .unwrap();
        // O servidor conhece só a raiz: o cliente precisa enviar o intermediário.
        let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .unwrap();
        let config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![
                    CertificateDer::from_pem_slice(if expired { EXPIRED } else { SERVER }).unwrap(),
                ],
                PrivateKeyDer::from_pem_slice(KEY).unwrap(),
            )
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let task = thread::spawn(move || serve(listener, config));
        Self { port, task }
    }

    fn url(&self, host: &str) -> String {
        format!("https://{host}:{}/", self.port)
    }
}

fn serve(listener: TcpListener, config: rustls::ServerConfig) -> Result<String, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let socket = loop {
        match listener.accept() {
            Ok((socket, _)) => break socket,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    return Err("No TLS connection received".into());
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => return Err(error.to_string()),
        }
    };
    // No Windows, o socket aceito herda o modo não bloqueante do listener.
    socket.set_nonblocking(false).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let connection = rustls::ServerConnection::new(Arc::new(config)).unwrap();
    let mut stream = rustls::StreamOwned::new(connection, socket);
    let request = read_request(&mut stream).map_err(|error| error.to_string())?;
    let peer = stream
        .conn
        .peer_certificates()
        .ok_or("Missing client identity")?;
    if peer.len() != 2 {
        return Err(format!(
            "Expected leaf and intermediate, received {} certificates",
            peer.len()
        ));
    }
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n<ok/>")
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;
    Ok(request)
}

fn read_request(
    stream: &mut rustls::StreamOwned<rustls::ServerConnection, TcpStream>,
) -> std::io::Result<String> {
    let mut request = Vec::new();
    let mut byte = [0];
    while !request.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte)?;
        request.push(byte[0]);
    }
    let headers = String::from_utf8(request).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap();
    let mut body = vec![0; length];
    stream.read_exact(&mut body)?;
    Ok(String::from_utf8(body).unwrap())
}

async fn send(client: &SefazClient, url: &str) -> Result<String, FiscalError> {
    client
        .send_to_url(SefazService::StatusServico, url, "SP", "<probe/>")
        .await
}

#[tokio::test]
async fn mtls_sends_intermediate_and_preserves_soap() {
    let server = Server::start(false);
    let client = SefazClient::with_root_certificates(PFX, PASSWORD, &[ROOT]).unwrap();
    let response = send(&client, &server.url("localhost")).await;
    let request = server.task.join().unwrap();
    assert!(
        response.is_ok(),
        "client: {response:?}; server: {request:?}"
    );
    assert_eq!(response.unwrap(), "<ok/>");
    let request = request.unwrap();
    assert!(request.contains("<probe/>"));
    assert!(request.contains("Envelope"));
}

#[tokio::test]
async fn rejects_untrusted_server_and_keeps_tls_cause_without_url_secrets() {
    let server = Server::start(false);
    let client = SefazClient::new(PFX, PASSWORD).unwrap();
    let url = format!("{}?token=do-not-expose", server.url("localhost"));
    let error = send(&client, &url).await.unwrap_err();
    let FiscalError::Network(message) = error else {
        panic!("Expected transport error")
    };
    assert!(message.contains("connection/TLS"), "{message}");
    #[cfg(not(windows))]
    assert!(message.to_lowercase().contains("certificate"), "{message}");
    assert!(message.contains("client error (Connect):"), "{message}");
    assert!(!message.contains("do-not-expose"), "{message}");
    assert!(!message.contains("BEGIN PRIVATE KEY"));
    assert!(server.task.join().unwrap().is_err());
}

#[tokio::test]
async fn rejects_wrong_hostname_even_with_trusted_ca() {
    let server = Server::start(false);
    let client = SefazClient::with_root_certificates(PFX, PASSWORD, &[ROOT]).unwrap();
    assert!(matches!(
        send(&client, &server.url("127.0.0.1")).await,
        Err(FiscalError::Network(_))
    ));
    assert!(server.task.join().unwrap().is_err());
}

#[tokio::test]
async fn rejects_expired_server_even_with_trusted_ca() {
    let server = Server::start(true);
    let client = SefazClient::with_root_certificates(PFX, PASSWORD, &[ROOT]).unwrap();
    assert!(matches!(
        send(&client, &server.url("localhost")).await,
        Err(FiscalError::Network(_))
    ));
    assert!(server.task.join().unwrap().is_err());
}

#[tokio::test]
async fn server_rejects_client_without_intermediate() {
    let server = Server::start(false);
    let client = SefazClient::with_root_certificates(LEAF_ONLY, PASSWORD, &[ROOT]).unwrap();
    assert!(send(&client, &server.url("localhost")).await.is_err());
    assert!(server.task.join().unwrap().is_err());
}

#[test]
fn rejects_invalid_trust_anchor() {
    assert!(matches!(
        SefazClient::with_root_certificates(PFX, PASSWORD, &[b"invalid PEM"]),
        Err(FiscalError::Certificate(_))
    ));
}
