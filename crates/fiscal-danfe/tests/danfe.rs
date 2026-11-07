//! End-to-end tests of the NF-e DANFE rendering.

use fiscal_danfe::{DanfeOptions, render_danfe};

const XML: &str = include_str!("fixtures/nfe_55.xml");

fn options() -> DanfeOptions {
    DanfeOptions {
        comprimir_pdf: false,
        sistema: "TESTE|centro|direita".to_owned(),
        ..DanfeOptions::default()
    }
}

/// The PDF as text, one char per byte (the content streams are plain when uncompressed).
fn render(xml: &str, options: &DanfeOptions) -> String {
    let pdf = render_danfe(xml, options).expect("danfe");
    assert!(pdf.starts_with(b"%PDF-"));
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
            'Ê' => '\u{CA}',
            'Í' => '\u{CD}',
            'Á' => '\u{C1}',
            'Ó' => '\u{D3}',
            'Ú' => '\u{DA}',
            'Â' => '\u{C2}',
            'á' => '\u{E1}',
            'é' => '\u{E9}',
            'í' => '\u{ED}',
            'ç' => '\u{E7}',
            'ã' => '\u{E3}',
            'ô' => '\u{F4}',
            'º' => '\u{BA}',
            c => c,
        })
        .collect()
}

fn has(pdf: &str, text: &str) -> bool {
    pdf.contains(&format!("({}) Tj", cp1252(text)))
}

/// An NF-e with `n` items cloned from the first one.
fn with_items(n: usize) -> String {
    let start = XML.find("<det nItem=\"1\">").expect("first item");
    let end = XML.find("</det>").expect("end") + "</det>".len();
    let first = &XML[start..end];
    let items: String = (1..=n)
        .map(|i| {
            first
                .replace("nItem=\"1\"", &format!("nItem=\"{i}\""))
                .replace("<cProd>0001</cProd>", &format!("<cProd>ITEM{i:04}</cProd>"))
        })
        .collect();
    let last_end = XML.rfind("</det>").expect("last") + "</det>".len();
    format!("{}{}{}", &XML[..start], items, &XML[last_end..])
}

#[test]
fn renders_a_one_page_danfe_with_every_block() {
    let pdf = render(XML, &options());
    assert!(pdf.contains("/Count 1"));
    for text in [
        "DANFE",
        "IDENTIFICAÇÃO DO EMITENTE",
        "NATUREZA DA OPERAÇÃO",
        "VENDA DE MERCADORIA ADQUIRIDA DE TERCEIROS",
        "CHAVE DE ACESSO",
        "3526 0711 2223 3300 0181 5500 1000 0012 3411 2345 6780",
        "PROTOCOLO DE AUTORIZAÇÃO DE USO",
        "135260000012345 20/07/2026 10:33:40",
        "DESTINATÁRIO / REMETENTE",
        "CLIENTE MODELO DISTRIBUIDORA S/A",
        "99.888.777/0001-66",
        "FATURA / DUPLICATA",
        "CÁLCULO DO IMPOSTO",
        "664,75",
        "TRANSPORTADOR / VOLUMES TRANSPORTADOS",
        "1 - DESTINATARIO",
        "DADOS DOS PRODUTOS / SERVIÇOS",
        "0001",
        "73181500",
        "DADOS ADICIONAIS",
        "INFORMAÇÕES COMPLEMENTARES",
        "RESERVADO AO FISCO",
        "Folha 1/1",
        "DATA DE RECEBIMENTO",
    ] {
        assert!(has(&pdf, text), "missing {text:?}");
    }
    assert!(has(&pdf, "TESTE"), "footer left");
    assert!(has(&pdf, "centro") && has(&pdf, "direita"));
    // No watermark for an authorised production invoice.
    assert!(!pdf.contains("SEM VALOR FISCAL"));
}

#[test]
fn the_default_output_is_compressed() {
    let compressed = render_danfe(XML, &DanfeOptions::default()).expect("danfe");
    let plain = render_danfe(XML, &options()).expect("danfe");
    assert!(compressed.len() < plain.len());
}

#[test]
fn long_item_lists_flow_across_pages() {
    let xml = with_items(120);
    let pdf = render(&xml, &options());
    let pages: usize = pdf
        .split("/Count ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .expect("page count");
    assert!(pages >= 3, "120 items need several pages, got {pages}");
    for i in 1..=120 {
        assert!(has(&pdf, &format!("ITEM{i:04}")), "item {i} is missing");
    }
    // Header and additional data repeat on every page, the receipt stub only once.
    assert_eq!(
        pdf.matches(&format!("({}) Tj", "DADOS ADICIONAIS")).count(),
        pages
    );
    assert_eq!(pdf.matches("(DANFE) Tj").count(), pages);
    assert_eq!(
        pdf.matches(&format!("({}) Tj", "DATA DE RECEBIMENTO"))
            .count(),
        1
    );
    // The total of pages is known in the first pass.
    assert!(has(&pdf, &format!("Folha 1/{pages}")));
    assert!(has(&pdf, &format!("Folha {pages}/{pages}")));
}

#[test]
fn homologation_and_missing_protocol_are_marked() {
    let homolog = XML.replace("<tpAmb>1</tpAmb><finNFe>", "<tpAmb>2</tpAmb><finNFe>");
    let pdf = render(&homolog, &options());
    assert!(pdf.contains("(SEM VALOR FISCAL) Tj"));
    assert!(has(&pdf, "AMBIENTE DE HOMOLOGAÇÃO"));

    let start = XML.find("<NFe ").expect("NFe");
    let end = XML.find("</NFe>").expect("end") + "</NFe>".len();
    let pdf = render(&XML[start..end], &options());
    assert!(pdf.contains("(SEM VALOR FISCAL) Tj"));
    assert!(has(&pdf, "FALTA PROTOCOLO DE AUTORIZAÇÃO DA SEFAZ"));
    assert!(has(&pdf, "NFe sem Autorização de Uso da SEFAZ"));
}

#[test]
fn canceled_and_denied_invoices_carry_their_watermarks() {
    let cancelled = XML.replace("<cStat>100</cStat>", "<cStat>101</cStat>");
    assert!(render(&cancelled, &options()).contains("(NOTA CANCELADA) Tj"));
    let denied = XML.replace("<cStat>100</cStat>", "<cStat>110</cStat>");
    let pdf = render(&denied, &options());
    assert!(pdf.contains("(NFe USO DENEGADO) Tj"));
    assert!(has(&pdf, "PROTOCOLO DE DENEGAÇÃO DE USO"));
    let flagged = DanfeOptions {
        cancelada: true,
        ..options()
    };
    assert!(render(XML, &flagged).contains("(NOTA CANCELADA) Tj"));
}

#[test]
fn contingency_prints_the_36_digit_key_as_a_second_barcode() {
    let xml = XML.replace("<tpEmis>1</tpEmis>", "<tpEmis>5</tpEmis>");
    let pdf = render(&xml, &options());
    assert!(has(&pdf, "DADOS DA NF-E"));
    assert!(has(&pdf, "DANFE Emitido em Contingência"));
    assert!(!has(
        &pdf,
        "Consulta de autenticidade no portal nacional da NF-e"
    ));
}

#[test]
fn landscape_documents_use_a_landscape_page() {
    let xml = XML.replace("<tpImp>1</tpImp>", "<tpImp>2</tpImp>");
    let pdf = render(&xml, &options());
    assert!(pdf.contains("/MediaBox [0 0 841.89 595.28]"));
    assert!(has(&pdf, "AO LADO") || pdf.contains("AO LADO"));
}

#[test]
fn receipt_stub_position_and_suppression() {
    let bottom = DanfeOptions {
        pos_canhoto: fiscal_danfe::options::PosCanhoto::Rodape,
        ..options()
    };
    assert!(has(&render(XML, &bottom), "DATA DE RECEBIMENTO"));
    let left = DanfeOptions {
        pos_canhoto: fiscal_danfe::options::PosCanhoto::Esquerda,
        ..options()
    };
    assert!(has(&render(XML, &left), "DATA DE RECEBIMENTO"));
    let none = DanfeOptions {
        pos_canhoto: fiscal_danfe::options::PosCanhoto::Nenhum,
        ..options()
    };
    assert!(!has(&render(XML, &none), "DATA DE RECEBIMENTO"));
}

#[test]
fn rejects_documents_that_are_not_an_nfe() {
    assert!(render_danfe("<x/>", &options()).is_err());
}

#[test]
fn output_is_valid_pdf_structure() {
    let pdf = render_danfe(XML, &DanfeOptions::default()).expect("danfe");
    assert!(pdf.starts_with(b"%PDF-1."));
    assert!(pdf.ends_with(b"%%EOF\n"));
}

#[test]
fn left_stub_widths_round_half_to_even_like_the_original() {
    // With the stub on the left the band is 185 mm wide and the phone box of the recipient is
    // `RoundTo((185 - 87 - 29 - 8) / 2, 0)` = `RoundTo(30.5, 0)`: 30 mm in the original (half to
    // even), which puts the end of that box at x = 414.43 pt. Rounding half away from zero made
    // it 31 mm (417.26 pt). Checked against the real ACBr (tests/acbr).
    let options = DanfeOptions {
        pos_canhoto: fiscal_danfe::options::PosCanhoto::Esquerda,
        margem_esquerda: 2.0,
        margem_superior: 2.0,
        margem_direita: 2.0,
        margem_inferior: 2.0,
        ..options()
    };
    let pdf = render(XML, &options);
    assert!(
        pdf.contains("414.43 654.80 l"),
        "phone box of the recipient"
    );
    assert!(!pdf.contains("417.26 654.80 l"));
}
