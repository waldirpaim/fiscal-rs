//! Ordenação da cadeia mTLS. Não estabelece confiança: isso cabe ao backend TLS.

use der::{Decode, Encode};
use fiscal_core::FiscalError;
use rsa::{RsaPublicKey, pkcs1v15::Pkcs1v15Sign, pkcs8::DecodePublicKey};
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha384, Sha512};
use x509_cert::{
    Certificate,
    ext::pkix::{AuthorityKeyIdentifier, SubjectKeyIdentifier},
};

use super::{pfx::x509_der_to_pem, pkcs12_parser::ParsedPkcs12};

pub(super) fn build_tls_chain(
    parsed: &ParsedPkcs12,
    leaf_pem: &str,
) -> Result<String, FiscalError> {
    let mut chain = leaf_pem.to_owned();
    let mut current = parse(&parsed.cert)?;
    let mut remaining = parsed
        .ca
        .iter()
        .map(|der| parse(der).map(|cert| (der, cert)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut visited = vec![parsed.cert.as_slice()];

    loop {
        // DN igual identifica um autoemitido, não necessariamente um autofirmado.
        if is_self_signed(&current)? {
            break;
        }
        let mut selected = None;
        for (index, (der, issuer)) in remaining.iter().enumerate() {
            if visited.contains(&der.as_slice()) {
                continue;
            }
            if let Some(score) = issuer_score(&current, issuer)? {
                if selected.is_none_or(|(_, best)| score > best) {
                    selected = Some((index, score));
                }
            }
        }
        let Some((index, _)) = selected else { break };
        let (der, issuer) = remaining.remove(index);
        if is_self_signed(&issuer)? {
            break;
        }
        chain.push_str(&x509_der_to_pem(der)?);
        visited.push(der.as_slice());
        current = issuer;
    }
    Ok(chain)
}

fn parse(der: &[u8]) -> Result<Certificate, FiscalError> {
    Certificate::from_der(der)
        .map_err(|e| FiscalError::Certificate(format!("Invalid certificate in TLS chain: {e}")))
}

fn is_self_signed(cert: &Certificate) -> Result<bool, FiscalError> {
    Ok(
        cert.tbs_certificate().subject() == cert.tbs_certificate().issuer()
            && verifies_signature(cert, cert)? == Some(true),
    )
}

fn issuer_score(cert: &Certificate, issuer: &Certificate) -> Result<Option<u8>, FiscalError> {
    if cert.tbs_certificate().issuer() != issuer.tbs_certificate().subject() {
        return Ok(None);
    }
    let aki = cert
        .tbs_certificate()
        .get_extension::<AuthorityKeyIdentifier>()
        .map_err(|e| FiscalError::Certificate(format!("Invalid authority key identifier: {e}")))?
        .map(|(_, extension)| extension);
    let ski = issuer
        .tbs_certificate()
        .get_extension::<SubjectKeyIdentifier>()
        .map_err(|e| FiscalError::Certificate(format!("Invalid subject key identifier: {e}")))?
        .map(|(_, extension)| extension.0);
    let mut key_match = false;
    if let Some(aki) = aki {
        if let (Some(expected), Some(actual)) = (aki.key_identifier, ski) {
            if expected != actual {
                return Ok(None);
            }
            key_match = true;
        }
        if let Some(serial) = aki.authority_cert_serial_number {
            if &serial != issuer.tbs_certificate().serial_number() {
                return Ok(None);
            }
        }
    }
    Ok(match verifies_signature(cert, issuer)? {
        Some(true) => Some(2),
        Some(false) => None,
        None => Some(u8::from(key_match)),
    })
}

// Verifica apenas a ligação criptográfica para ordenar/omitir certificados.
// Validade, constraints, revogação e confiança continuam com o backend TLS.
// Algoritmos não cobertos são preservados conservadoramente, inclusive raízes;
// AKI/SKI ajudam a ordenar, mas nunca concedem confiança a um certificado.
fn verifies_signature(
    cert: &Certificate,
    issuer: &Certificate,
) -> Result<Option<bool>, FiscalError> {
    let tbs = cert
        .tbs_certificate()
        .to_der()
        .map_err(|e| FiscalError::Certificate(format!("Invalid certificate encoding: {e}")))?;
    let (padding, digest) = match cert.signature_algorithm().oid.to_string().as_str() {
        "1.2.840.113549.1.1.5" => (Pkcs1v15Sign::new::<Sha1>(), Sha1::digest(&tbs).to_vec()),
        "1.2.840.113549.1.1.11" => (Pkcs1v15Sign::new::<Sha256>(), Sha256::digest(&tbs).to_vec()),
        "1.2.840.113549.1.1.12" => (Pkcs1v15Sign::new::<Sha384>(), Sha384::digest(&tbs).to_vec()),
        "1.2.840.113549.1.1.13" => (Pkcs1v15Sign::new::<Sha512>(), Sha512::digest(&tbs).to_vec()),
        _ => return Ok(None),
    };
    let spki = issuer
        .tbs_certificate()
        .subject_public_key_info()
        .to_der()
        .map_err(|e| FiscalError::Certificate(format!("Invalid issuer public key: {e}")))?;
    let Ok(key) = RsaPublicKey::from_public_key_der(&spki) else {
        return Ok(None);
    };
    let Some(signature) = cert.signature().as_bytes() else {
        return Ok(Some(false));
    };
    Ok(Some(key.verify(padding, &digest, signature).is_ok()))
}
