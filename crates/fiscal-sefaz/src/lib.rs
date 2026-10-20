//! SEFAZ web-service integration for NF-e / NFC-e authorization and events.
//!
//! This crate provides:
//!
//! - [`urls`] — endpoint URL resolution for all SEFAZ web services by state and environment.
//! - [`request_builders`] — XML request builders for authorization, cancellation, events, etc.
//! - [`response_parsers`] — SEFAZ XML response parsing and typed result structs.
//! - [`services`] — SOAP metadata (method, operation, version) for each web service.
//! - [`client`] — async SEFAZ client with mTLS authentication (feature-gated).
//!
//! The [`status_codes`] module from `fiscal-core` is re-exported for convenience.
//!
//! ## Transporte TLS
//!
//! O cliente exige HTTPS e TLS 1.2 ou superior, valida o certificado e o hostname
//! do servidor e transmite os intermediários do PFX no mTLS. Redirecionamentos
//! não são seguidos: o XML fiscal só é enviado ao endpoint solicitado.
//!
//! No macOS, o backend é o TLS nativo, como na versão 0.6 (issue upstream #74).
//! Nas demais plataformas, rustls continua sendo o padrão. No Linux, a feature
//! `openssl-tls` permite selecionar OpenSSL e requer seus arquivos de desenvolvimento.
//! No Windows, rustls permanece ativo inclusive nos builds com todas as features,
//! para preservar o envio dos intermediários sem depender do repositório de CAs
//! do Schannel.
//!
//! As CAs do sistema são usadas por padrão. CAs privadas ou adicionais podem ser
//! fornecidas por [`client::SefazClient::with_root_certificates`] (feature `client`),
//! sem desativar a validação do servidor. A cadeia do cliente no PFX não é
//! automaticamente considerada confiável para autenticar o servidor.

/// XML request builders for SEFAZ web services.
pub mod request_builders;
/// SEFAZ XML response parsers and typed result structs.
pub mod response_parsers;
/// SOAP metadata for each NF-e/NFC-e web service operation.
pub mod services;
/// SEFAZ endpoint URL resolution by UF and environment.
pub mod urls;
/// NF-e XML validation: pre-send structural checks and post-authorization SEFAZ verification.
pub mod validate;

mod soap;

/// Async SEFAZ web service client with mTLS authentication.
///
/// Requires the `client` feature (enabled by default).
#[cfg(feature = "client")]
pub mod client;

// Re-export status_codes from fiscal-core for convenience
pub use fiscal_core::status_codes;
