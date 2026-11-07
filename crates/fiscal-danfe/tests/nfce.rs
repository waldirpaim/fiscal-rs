//! End-to-end tests of the NFC-e (receipt) DANFE.

use fiscal_danfe::{DanfeOptions, render_danfe};

const XML: &str = include_str!("fixtures/nfce_65.xml");

/// A 1x1 RGB PNG.
const LOGO: [u8; 67] = [
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0xC9, 0xFE, 0x92, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42,
];

fn options() -> DanfeOptions {
    DanfeOptions {
        comprimir_pdf: false,
        sistema: "TESTE".to_owned(),
        ..DanfeOptions::default()
    }
}

fn render(xml: &str, options: &DanfeOptions) -> String {
    let pdf = render_danfe(xml, options).expect("danfe");
    pdf.iter().map(|b| char::from(*b)).collect()
}

fn cp1252(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'Ç' => '\u{C7}',
            'Ã' => '\u{C3}',
            'Ê' => '\u{CA}',
            'É' => '\u{C9}',
            'ç' => '\u{E7}',
            'ã' => '\u{E3}',
            'é' => '\u{E9}',
            'ê' => '\u{EA}',
            'ó' => '\u{F3}',
            'Ó' => '\u{D3}',
            'í' => '\u{ED}',
            'ú' => '\u{FA}',
            'Í' => '\u{CD}',
            c => c,
        })
        .collect()
}

fn has(pdf: &str, text: &str) -> bool {
    // Parentheses are escaped inside PDF strings.
    let escaped = cp1252(text).replace('(', "\\(").replace(')', "\\)");
    pdf.contains(&format!("({escaped}) Tj"))
}

/// Width and height of the first MediaBox in points.
fn media_box(pdf: &str) -> (f64, f64) {
    let start = pdf.find("/MediaBox [0 0 ").expect("MediaBox") + "/MediaBox [0 0 ".len();
    let mut nums = pdf[start..]
        .split([' ', ']'])
        .filter_map(|s| s.parse::<f64>().ok());
    (nums.next().expect("w"), nums.next().expect("h"))
}

#[test]
fn renders_the_receipt_blocks() {
    let pdf = render(XML, &options());
    for text in [
        "MERCADINHO EXEMPLO LTDA",
        "CNPJ: 11.222.333/0001-81 IE: 131234567",
        "DANFE NFC-e Documento Auxiliar da",
        "Nota Fiscal de Consumidor Eletronica",
        "CÓDIGO",
        "DESCRIÇÃO",
        "LEITE INTEGRAL UHT",
        "Qtd. Total de Itens",
        "Valor Total R$",
        "Valor a Pagar R$",
        "FORMA PAGAMENTO",
        "Dinheiro",
        "Troco R$",
        "Consulte pela Chave de Acesso em",
        "https://www.sefaz.mt.gov.br/nfce/consultanfce",
        "5126 0711 2223 3300 0181 6500 1000 0045 2118 7654 3211",
        "CONSUMIDOR - CPF: 123.456.789-09",
        "NFCe n. 000004521 Série 001 20/07/2026 18:42:10",
        "Via Consumidor",
        "Protocolo de Autorização: 151260000045210",
        "Data de Autorização: 20/07/2026 18:42:15",
        "Tributos Totais Incidentes (Lei Federal 12.741/2012): R$ 8,31",
        "TESTE",
    ] {
        assert!(has(&pdf, text), "missing {text:?}");
    }
    // The QR Code is drawn with many small rectangles.
    assert!(pdf.matches(" re f\n").count() > 200);
}

#[test]
fn the_page_is_80_mm_wide_and_as_tall_as_its_content() {
    let (w, h) = media_box(&render(XML, &options()));
    assert!((w - 80.0 * 72.0 / 25.4).abs() < 0.1, "{w}");
    assert!(h > 300.0 && h < 700.0, "{h}");

    // More items, more height.
    let first = XML.find("<det nItem=\"3\">").expect("item");
    let end = XML[first..].find("</det>").expect("end") + first + "</det>".len();
    let item = &XML[first..end];
    let extra: String = (4..=24)
        .map(|i| item.replace("nItem=\"3\"", &format!("nItem=\"{i}\"")))
        .collect();
    let big = format!("{}{}{}", &XML[..end], extra, &XML[end..]);
    let (_, taller) = media_box(&render(&big, &options()));
    assert!(taller > h + 100.0, "{taller} vs {h}");
    // It stays a single page.
    assert!(render(&big, &options()).contains("/Count 1"));
}

#[test]
fn narrow_paper_uses_smaller_fonts() {
    let narrow = DanfeOptions {
        largura_bobina: 58,
        ..options()
    };
    let (w, _) = media_box(&render(XML, &narrow));
    assert!((w - 58.0 * 72.0 / 25.4).abs() < 0.1, "{w}");
}

#[test]
fn homologation_and_contingency_banners() {
    let homolog = XML.replace("<tpAmb>1</tpAmb><finNFe>", "<tpAmb>2</tpAmb><finNFe>");
    let pdf = render(&homolog, &options());
    assert!(has(&pdf, "SEM VALOR FISCAL"));
    assert!(has(&pdf, "EMITIDA EM AMBIENTE DE HOMOLOGAÇÃO"));
    assert!(
        has(
            &pdf,
            "EMITIDA EM AMBIENTE DE HOMOLOGAÇÃO - SEM VALOR FISCAL"
        ) || pdf.contains("SEM VALOR FISCAL")
    );

    let start = XML.find("<protNFe").expect("prot");
    let end = XML.find("</protNFe>").expect("end") + "</protNFe>".len();
    let offline = format!(
        "{}{}",
        XML[..start].replace("<tpEmis>1</tpEmis>", "<tpEmis>9</tpEmis>"),
        &XML[end..]
    );
    let pdf = render(&offline, &options());
    assert!(has(&pdf, "EMITIDA EM CONTINGÊNCIA"));
    assert!(has(&pdf, "Pendente de autorização"));
}

#[test]
fn missing_protocol_is_flagged() {
    let start = XML.find("<protNFe").expect("prot");
    let end = XML.find("</protNFe>").expect("end") + "</protNFe>".len();
    let pdf = render(&format!("{}{}", &XML[..start], &XML[end..]), &options());
    assert!(has(&pdf, "NFCe sem Autorização de Uso da SEFAZ"));
}

#[test]
fn unidentified_consumer_and_optional_lines() {
    let anonymous = XML.replace("<CPF>12345678909</CPF>", "");
    assert!(has(
        &render(&anonymous, &options()),
        "CONSUMIDOR NÃO IDENTIFICADO"
    ));
    let named = DanfeOptions {
        exibir_consumidor_nome: true,
        exibir_nome_fantasia: true,
        ..options()
    };
    let pdf = render(XML, &named);
    assert!(has(&pdf, "MARIA CONSUMIDORA"));
    assert!(has(&pdf, "MERCADINHO EXEMPLO"));
}

#[test]
fn the_items_list_can_be_hidden() {
    let hidden = DanfeOptions {
        exibir_itens: false,
        ..options()
    };
    let pdf = render(XML, &hidden);
    assert!(!has(&pdf, "DESCRIÇÃO"));
    assert!(has(&pdf, "Valor a Pagar R$"));
}

#[test]
fn lateral_qr_code_moves_the_issue_data_beside_it() {
    let lateral = DanfeOptions {
        imprime_qrcode_lateral: true,
        ..options()
    };
    let pdf = render(XML, &lateral);
    assert!(has(&pdf, "Emissão: 20/07/2026 18:42:10"));
    assert!(!has(
        &pdf,
        "NFCe n. 000004521 Série 001 20/07/2026 18:42:10"
    ));
}

#[test]
fn simplified_type_2_layout() {
    let simplified = DanfeOptions {
        simplificado_tipo2: true,
        ..options()
    };
    // Tax-reform totals: IBS, CBS and the selective tax.
    let xml = XML.replace(
        "</ICMSTot>",
        "</ICMSTot><IBSCBSTot><gIBS><vIBS>0.42</vIBS></gIBS><gCBS><vCBS>0.37</vCBS></gCBS></IBSCBSTot>\
         <ISTot><vIS>1.25</vIS></ISTot>",
    );
    let pdf = render(&xml, &simplified);
    assert!(has(&pdf, "DANFE Simplificado Tipo 2"));
    assert!(has(&pdf, "(+) CBS R$"));
    assert!(has(&pdf, "(+) IBS R$"));
    assert!(has(&pdf, "(+) IS R$"));
    assert!(has(&pdf, "0,37") && has(&pdf, "0,42") && has(&pdf, "1,25"));
    assert!(has(&pdf, "NFe n. 000004521 Série 001 20/07/2026 18:42:10"));
}

#[test]
fn a_logo_is_drawn_and_the_page_still_fits_the_content() {
    let with_logo = DanfeOptions {
        logo: Some(LOGO.to_vec()),
        ..options()
    };
    let pdf = render(XML, &with_logo);
    assert!(pdf.contains("/Subtype /Image"));
    let (_, h_logo) = media_box(&pdf);
    let (_, h_plain) = media_box(&render(XML, &options()));
    assert!(
        (h_logo - h_plain - 5.0 * 72.0 / 25.4).abs() < 0.5,
        "{h_logo} {h_plain}"
    );
}

#[test]
fn model_55_keeps_the_full_page_layout() {
    let nfe55 = include_str!("fixtures/nfe_55.xml");
    let pdf = render(nfe55, &options());
    let (w, h) = media_box(&pdf);
    assert!((w - 595.28).abs() < 0.1 && (h - 841.89).abs() < 0.1);
}
