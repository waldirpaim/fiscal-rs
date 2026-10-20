use super::{
    SefazClient,
    tests::{TEST_PASSWORD, test_pfx},
};
use crate::request_builders;
use fiscal_core::{FiscalError, types::SefazEnvironment};

// Transporte restrito aos testes: captura a URL já resolvida e o SOAP pronto,
// sem transmitir documentos nem depender da disponibilidade da SEFAZ.
pub(super) struct TestTransport {
    request: std::sync::Mutex<Option<(String, String)>>,
    response: String,
}

impl TestTransport {
    pub(super) fn send(&self, url: &str, envelope: &str) -> String {
        *self.request.lock().unwrap() = Some((url.to_string(), envelope.to_string()));
        self.response.clone()
    }
}

const NFCE_ACCESS_KEY: &str = "41250106157250000116650010000000011000000013";
const CANCEL_RESPONSE: &str = concat!(
    "<retEnvEvento><cStat>128</cStat><retEvento><infEvento>",
    "<cStat>135</cStat><xMotivo>Evento registrado</xMotivo>",
    "<nProt>141250000000017</nProt></infEvento></retEvento></retEnvEvento>"
);

fn nfce_client(response: &str) -> SefazClient {
    let mut client = SefazClient::new(&test_pfx(), TEST_PASSWORD).expect("client builds");
    client.test_transport = Some(TestTransport {
        request: std::sync::Mutex::new(None),
        response: response.to_string(),
    });
    client
}

fn captured_request(client: &SefazClient) -> (String, String) {
    client
        .test_transport
        .as_ref()
        .unwrap()
        .request
        .lock()
        .unwrap()
        .take()
        .unwrap()
}

#[tokio::test]
async fn cancel_nfce_signs_and_routes_the_public_method() {
    let client = nfce_client(CANCEL_RESPONSE);
    // A UF da chave sintética é PR; o dígito verificador também é válido.
    assert_eq!(&NFCE_ACCESS_KEY[20..22], "65");
    for (environment, host) in [
        (
            SefazEnvironment::Homologation,
            "homologacao.nfce.sefa.pr.gov.br",
        ),
        (SefazEnvironment::Production, "nfce.sefa.pr.gov.br"),
    ] {
        let response = client
            .cancel_nfce(
                "PR",
                environment,
                NFCE_ACCESS_KEY,
                "141250000000017",
                "Cancelamento de NFC-e para teste de regressão",
                "06157250000116",
            )
            .await
            .unwrap();
        let (url, envelope) = captured_request(&client);
        assert_eq!(url, format!("https://{host}/nfce/NFeRecepcaoEvento4"));
        assert!(envelope.contains("<tpEvento>110111</tpEvento>"));
        assert!(envelope.contains(&format!("<chNFe>{NFCE_ACCESS_KEY}</chNFe>")));
        assert!(envelope.contains(&format!("<tpAmb>{}</tpAmb>", environment.as_str())));
        assert!(envelope.contains(&response.signed_event_xml));
        assert!(response.signed_event_xml.contains("<Signature"));
        assert!(
            response
                .signed_event_xml
                .contains(&format!("Reference URI=\"#ID110111{NFCE_ACCESS_KEY}01\""))
        );
        let signature = response.signed_event_xml.find("<Signature").unwrap();
        let close = response.signed_event_xml.find("</evento>").unwrap();
        assert!(signature < close);
        assert_eq!(response.status_code, "135");
        assert_eq!(response.status_message, "Evento registrado");
        assert_eq!(response.protocol_number.as_deref(), Some("141250000000017"));
        assert_eq!(response.raw_response, CANCEL_RESPONSE);
    }
}

#[tokio::test]
async fn cancel_nfce_preserves_sefaz_rejection() {
    let raw = "<retEvento><infEvento><cStat>573</cStat><xMotivo>Duplicidade de Evento</xMotivo></infEvento></retEvento>";
    let client = nfce_client(raw);
    let response = client
        .cancel_nfce(
            "PR",
            SefazEnvironment::Homologation,
            NFCE_ACCESS_KEY,
            "141250000000017",
            "Cancelamento de NFC-e para teste de regressão",
            "06157250000116",
        )
        .await
        .unwrap();
    assert_eq!(response.status_code, "573");
    assert_eq!(response.status_message, "Duplicidade de Evento");
    assert_eq!(response.protocol_number, None);
    assert_eq!(response.raw_response, raw);
}

#[tokio::test]
async fn cancel_nfce_rejects_malformed_response() {
    let client = nfce_client("<retEvento/>");
    let result = client
        .cancel_nfce(
            "PR",
            SefazEnvironment::Homologation,
            NFCE_ACCESS_KEY,
            "141250000000017",
            "Cancelamento de NFC-e para teste de regressão",
            "06157250000116",
        )
        .await;
    assert!(matches!(result, Err(FiscalError::XmlParsing(_))));
}

#[tokio::test]
async fn inutilize_nfce_routes_and_preserves_signed_xml() {
    let raw = "<retInutNFe><infInut><cStat>102</cStat><xMotivo>Inutilizacao homologada</xMotivo></infInut></retInutNFe>";
    let client = nfce_client(raw);
    for (environment, host) in [
        (
            SefazEnvironment::Homologation,
            "homologacao.nfce.sefa.pr.gov.br",
        ),
        (SefazEnvironment::Production, "nfce.sefa.pr.gov.br"),
    ] {
        let unsigned = request_builders::build_inutilizacao_request(
            26,
            "06157250000116",
            "65",
            1,
            100,
            110,
            "Faixa não utilizada para teste de regressão",
            environment,
            "PR",
        );
        let signed = fiscal_crypto::certificate::sign_inutilizacao_xml(
            &unsigned,
            &client.private_key,
            &client.certificate,
        )
        .unwrap();
        let response = client
            .inutilize_nfce("PR", environment, &signed)
            .await
            .unwrap();
        let (url, envelope) = captured_request(&client);
        assert_eq!(url, format!("https://{host}/nfce/NFeInutilizacao4"));
        assert!(envelope.contains(&signed));
        assert!(envelope.contains("<mod>65</mod>"));
        assert!(envelope.contains("<nNFIni>100</nNFIni><nNFFin>110</nNFFin>"));
        assert_eq!(response, raw);
    }
}
