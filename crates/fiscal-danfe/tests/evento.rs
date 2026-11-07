//! Tests of the DANFE of an NF-e event (correction letter, cancellation...).

use fiscal_danfe::{DanfeError, DanfeOptions, Evento, Nfe, render_evento, render_evento_xml};

const NFE: &str = include_str!("fixtures/nfe_55.xml");
const CCE: &str = include_str!("fixtures/evento_cce.xml");
const CANCELAMENTO: &str = include_str!("fixtures/evento_cancelamento.xml");

fn options() -> DanfeOptions {
    DanfeOptions {
        comprimir_pdf: false,
        sistema: "TESTE|centro|direita".to_owned(),
        ..DanfeOptions::default()
    }
}

/// The PDF as text, one char per byte (the content streams are plain when uncompressed).
fn render(evento_xml: &str) -> String {
    let pdf = render_evento_xml(NFE.as_bytes(), evento_xml.as_bytes(), &options()).expect("event");
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.ends_with(b"%%EOF\n") || pdf.ends_with(b"%%EOF"));
    pdf.iter().map(|b| char::from(*b)).collect()
}

/// Encodes accented text like the engine does (code page 1252) for searching the streams.
fn cp1252(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'Ç' => '\u{C7}',
            'Ã' => '\u{C3}',
            'Õ' => '\u{D5}',
            'É' => '\u{C9}',
            'Ô' => '\u{D4}',
            'Ú' => '\u{DA}',
            'Á' => '\u{C1}',
            'ç' => '\u{E7}',
            'ã' => '\u{E3}',
            'é' => '\u{E9}',
            'õ' => '\u{F5}',
            'á' => '\u{E1}',
            'ú' => '\u{FA}',
            c => c,
        })
        .collect()
}

fn has(pdf: &str, text: &str) -> bool {
    // PDF strings escape the parentheses.
    let escaped = cp1252(text).replace('(', "\\(").replace(')', "\\)");
    pdf.contains(&format!("({escaped}) Tj"))
}

fn pages(pdf: &str) -> usize {
    pdf.matches("/Type /Page\n").count() + pdf.matches("/Type /Page ").count()
}

#[test]
fn reads_the_correction_letter() {
    let evento = Evento::from_xml(CCE).expect("cce");
    let inf = &evento.inf;
    assert_eq!(inf.c_orgao, 35);
    assert_eq!(inf.tp_evento, "110110");
    assert_eq!(inf.ch_nfe, "35260711222333000181550010000012341123456780");
    assert_eq!(inf.cnpj_cpf, "11222333000181");
    assert_eq!(inf.n_seq_evento, 1);
    assert_eq!(inf.versao_evento, "1.00");
    assert_eq!(inf.det.desc_evento, "Carta de Correcao");
    assert!(inf.det.x_correcao.starts_with("O endereco de entrega"));
    assert!(inf.det.x_cond_uso.contains("com: I - as variaveis"));
    assert_eq!(
        inf.dh_evento.expect("date").datetime_br(),
        "21/07/2026 09:15:30"
    );
    assert_eq!(evento.ret.c_stat, 135);
    assert_eq!(evento.ret.x_motivo, "Evento registrado e vinculado a NF-e");
    assert_eq!(evento.ret.n_prot, "135260000098765");
    assert_eq!(
        evento.ret.dh_reg_evento.expect("date").datetime_br(),
        "21/07/2026 09:15:42"
    );
}

#[test]
fn reads_the_cancellation_and_the_bare_event() {
    let evento = Evento::from_xml(CANCELAMENTO).expect("cancelamento");
    assert_eq!(evento.inf.tp_evento, "110111");
    assert_eq!(evento.inf.det.n_prot, "135260000012345");
    assert!(evento.inf.det.x_just.starts_with("Erro na digitacao"));
    assert_eq!(evento.inf.tp_amb, fiscal_danfe::model::TpAmb::Homologacao);

    // Without `procEventoNFe` there is no answer from the tax authority.
    let start = CANCELAMENTO.find("<evento ").expect("evento");
    let end = CANCELAMENTO.find("</evento>").expect("end") + "</evento>".len();
    let bare = Evento::from_xml(&CANCELAMENTO[start..end]).expect("bare");
    assert_eq!(bare.inf.tp_evento, "110111");
    assert_eq!(bare.ret.c_stat, 0);
    assert!(bare.ret.n_prot.is_empty());
}

#[test]
fn rejects_other_documents() {
    assert!(matches!(
        Evento::from_xml(NFE),
        Err(DanfeError::InvalidNfe(message)) if message.contains("nfeProc")
    ));
    let empty = "<procEventoNFe><evento></evento></procEventoNFe>";
    assert!(matches!(
        Evento::from_xml(empty),
        Err(DanfeError::InvalidNfe(_))
    ));
    assert!(matches!(
        Evento::from_xml("<evento"),
        Err(DanfeError::Xml(_))
    ));
}

#[test]
fn reads_windows_1252_bytes() {
    let xml = CCE.replace("Rua das Palmeiras", "Rua José");
    let latin1: Vec<u8> = xml
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect();
    let evento = Evento::from_xml_bytes(&latin1).expect("latin1");
    assert!(evento.inf.det.x_correcao.contains("Rua José"));
}

#[test]
fn renders_the_correction_letter() {
    let pdf = render(CCE);
    assert_eq!(pages(&pdf), 1);
    assert!(has(&pdf, "Carta de Correcao"));
    assert!(has(
        &pdf,
        "Não possui valor fiscal, simples representação do fato indicado abaixo."
    ));
    assert!(has(
        &pdf,
        "CONSULTE A AUTENTICIDADE NO SITE DA SEFAZ AUTORIZADORA."
    ));
    // Invoice data.
    assert!(has(
        &pdf,
        "3526 0711 2223 3300 0181 5500 1000 0012 3411 2345 6780"
    ));
    assert!(has(&pdf, "000001234"));
    assert!(has(&pdf, "20/07/2026 10:33:28"));
    assert!(has(&pdf, "EMPRESA EXEMPLO COMERCIO E SERVICOS LTDA"));
    assert!(has(&pdf, "11.222.333/0001-81"));
    assert!(has(&pdf, "CLIENTE MODELO DISTRIBUIDORA S/A"));
    assert!(has(&pdf, "11:05:00"));
    // Event data.
    assert!(has(&pdf, "1 - PRODUÇÃO"));
    assert!(has(&pdf, "SP"));
    assert!(has(&pdf, "21/07/2026 09:15:30"));
    assert!(has(&pdf, "110110"));
    assert!(has(&pdf, "135 - Evento registrado e vinculado a NF-e"));
    assert!(has(&pdf, "135260000098765"));
    assert!(has(&pdf, "21/07/2026 09:15:42"));
    // Boxes of the correction letter and its footer.
    assert!(has(&pdf, "CONDIÇÕES DE USO"));
    assert!(has(&pdf, "CORREÇÕES A SEREM CONSIDERADAS"));
    // "com: I" breaks the line before the first item of the conditions.
    assert!(pdf.contains("relacionado com:) Tj"));
    assert!(pdf.contains("(I - as variaveis"));
    assert!(has(&pdf, "Demais dados permanecem inalterados."));
    assert!(pdf.contains("Este documento"));
    assert!(has(&pdf, "TESTE"));
    assert!(has(&pdf, "centro"));
    assert!(has(&pdf, "direita"));
}

#[test]
fn renders_the_cancellation() {
    let pdf = render(CANCELAMENTO);
    assert_eq!(pages(&pdf), 1);
    assert!(has(&pdf, "Cancelamento"));
    assert!(has(&pdf, "2 - HOMOLOGAÇÃO (SEM VALOR FISCAL)"));
    assert!(has(&pdf, "JUSTIFICATIVA DO CANCELAMENTO"));
    assert!(has(
        &pdf,
        "Erro na digitacao da quantidade dos itens; mercadoria nao saiu do estabelecimento."
    ));
    assert!(!pdf.contains(&cp1252("CONDIÇÕES DE USO")));
    assert!(pdf.contains("evento de NFe"));
}

#[test]
fn other_events_print_no_text_box() {
    let epec = CANCELAMENTO
        .replace("110111", "110140")
        .replace("<descEvento>Cancelamento", "<descEvento>EPEC");
    let pdf = render(&epec);
    assert_eq!(pages(&pdf), 1);
    assert!(has(&pdf, "EPEC"));
    assert!(has(&pdf, "110140"));
    assert!(!pdf.contains(&cp1252("JUSTIFICATIVA")));
    assert!(!pdf.contains(&cp1252("CORREÇÕES")));
    assert!(!pdf.contains("Este documento"));
}

#[test]
fn a_huge_text_does_not_break_the_document() {
    let huge = "texto longo ".repeat(2000);
    let xml = CANCELAMENTO.replace(
        "Erro na digitacao da quantidade dos itens; mercadoria nao saiu do estabelecimento.",
        &huge,
    );
    let pdf = render(&xml);
    assert!(pages(&pdf) >= 1);
}

#[test]
fn the_line_break_marker_follows_the_option() {
    let options = DanfeOptions {
        caractere_quebra_de_linha: "#".to_owned(),
        ..options()
    };
    let xml = CCE.replace("Ltda.|Demais", "Ltda.#Demais");
    let nfe = Nfe::from_xml(NFE).expect("nfe");
    let evento = Evento::from_xml(&xml).expect("evento");
    let pdf = render_evento(nfe, evento, &options).expect("render");
    let pdf: String = pdf.iter().map(|b| char::from(*b)).collect();
    assert!(has(&pdf, "Demais dados permanecem inalterados."));
    assert!(!pdf.contains("Ltda.#Demais"));
}

#[test]
fn the_event_pdf_is_compressed_by_default() {
    let nfe = Nfe::from_xml(NFE).expect("nfe");
    let evento = Evento::from_xml(CCE).expect("evento");
    let plain = render_evento(nfe.clone(), evento.clone(), &options()).expect("plain");
    let packed = render_evento(
        nfe,
        evento,
        &DanfeOptions {
            comprimir_pdf: true,
            ..options()
        },
    )
    .expect("packed");
    assert!(packed.len() < plain.len());
}
