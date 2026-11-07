//! Renders the extension features (barcodes, QR Code, rotation, text boxes, layers...) to
//! the path given as first argument (default: `ext_showcase.pdf`).

use fiscal_pdf::{
    Fpdf, HAlign, NamedColor, Orientation, PageFormat, QrEcc, TextBoxOpts, Unit, VAlign,
};

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ext_showcase.pdf".to_owned());
    let mut pdf = Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4);
    pdf.set_title("fiscal-pdf extensions");
    let hidden = pdf.add_layer("Camada oculta", false);
    let visible = pdf.add_layer("Camada visível", true);
    pdf.add_page();
    pdf.set_font("Helvetica", "", 8.0);

    // Barcodes.
    let key = "35260512345678000195550010000012341000012343";
    pdf.text(15.0, 14.0, "Code 128 (chave de acesso, conjunto C)");
    pdf.code128(key, 15.0, 16.0, 12.0, 100.0);
    pdf.text(15.0, 36.0, "Code 128 (texto)");
    pdf.code128("NFe-123 abc", 15.0, 38.0, 10.0, 60.0);
    pdf.text(15.0, 55.0, "EAN-13");
    pdf.code_ean13("400638133393", 15.0, 57.0, 14.0, 0.35);
    pdf.text(80.0, 55.0, "EAN-8");
    pdf.code_ean8("9638507", 80.0, 57.0, 14.0, 0.35);
    pdf.text(15.0, 78.0, "Code 39");
    pdf.code39("FISCAL-RS", 15.0, 80.0, 10.0, 0.6);
    pdf.text(80.0, 78.0, "Interleaved 2 of 5");
    pdf.code_i25("12345678", 80.0, 80.0, 10.0, 0.6);
    pdf.text(15.0, 98.0, "QR Code");
    let side = pdf.qr_code(
        15.0,
        100.0,
        "https://www.sefaz.mt.gov.br/nfce/consultanfce?p=51260710459473000105650010000000719000100031|2|1|1|ABC",
        0.8,
        QrEcc::Low,
    );
    println!("qr side = {side:.2} mm");

    // Shapes.
    pdf.set_draw_color(NamedColor::Navy);
    pdf.rounded_rect(110.0, 100.0, 40.0, 25.0, 5.0, "1234", "D");
    pdf.set_fill_color(NamedColor::Yellow);
    pdf.rounded_rect(155.0, 100.0, 40.0, 25.0, 4.0, "13", "FD");
    pdf.dashed_line(110.0, 130.0, 195.0, 130.0, 1.5);
    pdf.dashed_rect(110.0, 133.0, 85.0, 12.0, "D", 0.8);
    pdf.rotate(30.0, Some(60.0), Some(160.0));
    pdf.text(60.0, 160.0, "Texto rotacionado 30°");
    pdf.rotate(0.0, None, None);

    // Text boxes.
    pdf.set_font("Helvetica", "", 9.0);
    let long = "Este texto é quebrado automaticamente dentro da caixa, como nos campos do DANFE.";
    pdf.text_box(15.0, 175.0, 55.0, 22.0, long, &TextBoxOpts::default());
    pdf.text_box(
        75.0,
        175.0,
        55.0,
        22.0,
        long,
        &TextBoxOpts {
            halign: HAlign::Center,
            valign: VAlign::Center,
            ..TextBoxOpts::default()
        },
    );
    pdf.text_box(
        135.0,
        175.0,
        60.0,
        22.0,
        long,
        &TextBoxOpts {
            halign: HAlign::Right,
            valign: VAlign::Bottom,
            scale: true,
            ..TextBoxOpts::default()
        },
    );

    // Layers and HTML.
    pdf.begin_layer(visible);
    pdf.text(15.0, 210.0, "Esta linha está na camada visível");
    pdf.begin_layer(hidden);
    pdf.text(15.0, 216.0, "ESTA LINHA ESTÁ NA CAMADA OCULTA");
    pdf.end_layer();
    pdf.set_xy(15.0, 222.0);
    pdf.write_html(
        "Texto com <b>negrito</b>, <i>itálico</i>, <u>sublinhado</u> e <a href=\"https://example.com\">um link</a>.<br>Segunda linha.",
    );

    #[cfg(feature = "protection")]
    if std::env::args().any(|a| a == "--protect") {
        // Printing allowed, everything else restricted; empty user password.
        pdf.set_protection(
            fiscal_pdf::Permissions {
                print: true,
                ..fiscal_pdf::Permissions::NONE
            },
            "",
            "owner",
        );
    }

    pdf.save_to_file(&out).expect("write showcase");
    println!("{out}");
}
