use super::parse_sefaz_service;
use fiscal_sefaz::services::SefazService;

#[test]
fn service_aliases_accept_camel_case_lowercase_and_uppercase() {
    for (alias, expected) in [
        ("statusServico", SefazService::StatusServico),
        ("retAutorizacao", SefazService::RetAutorizacao),
        ("consultaProtocolo", SefazService::ConsultaProtocolo),
        ("recepcaoEvento", SefazService::RecepcaoEvento),
    ] {
        for input in [
            alias.to_string(),
            alias.to_lowercase(),
            alias.to_uppercase(),
        ] {
            assert_eq!(parse_sefaz_service(&input).unwrap(), expected);
        }
    }
}

#[test]
fn unknown_service_is_rejected() {
    assert!(parse_sefaz_service("not-a-sefaz-service").is_err());
}

#[test]
fn request_validation_preserves_null_and_errors() {
    let xml = r#"<enviNFe xmlns="http://www.portalfiscal.inf.br/nfe" versao="4.00"><idLote>1</idLote></enviNFe>"#;
    assert_eq!(
        crate::validate_request_xml(xml.into(), "4.00".into(), "enviNFe".into()).unwrap(),
        serde_json::Value::Null,
    );
    assert!(crate::validate_request_xml(String::new(), "4.00".into(), "enviNFe".into()).is_err());
}
