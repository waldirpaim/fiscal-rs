//! Behavioural tests of the engine extensions.

use fiscal_pdf::{
    Fpdf, HAlign, Link, NamedColor, Orientation, PageFormat, QrEcc, TextBoxOpts, Unit, VAlign,
};

fn doc() -> Fpdf {
    let mut pdf = Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4);
    pdf.set_compression(false);
    pdf.add_page();
    pdf.set_font("Helvetica", "", 10.0);
    pdf
}

fn text(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

fn count(s: &str, needle: &str) -> usize {
    s.matches(needle).count()
}

#[test]
fn code128_draws_one_rectangle_per_bar() {
    let mut pdf = doc();
    pdf.code128("1234", 10.0, 10.0, 10.0, 40.0);
    let s = text(&pdf.output().expect("pdf"));
    // Start C, 12, 34 and the check symbol have 3 bars each; the stop symbol has 3 bars and
    // the terminating pseudo-symbol 1.
    assert_eq!(count(&s, " re f\n"), 4 * 3 + 3 + 1);
}

#[test]
fn nfe_access_key_fits_the_requested_width() {
    let mut pdf = doc();
    let key = "35260512345678000195550010000012341000012343";
    pdf.code128(key, 10.0, 10.0, 12.0, 100.0);
    let s = text(&pdf.output().expect("pdf"));
    let k = 72.0 / 25.4;
    let mut max_right: f64 = 0.0;
    for line in s.lines().filter(|l| l.ends_with(" re f")) {
        let n: Vec<f64> = line.split(' ').filter_map(|t| t.parse().ok()).collect();
        max_right = max_right.max(n[0] + n[2]);
    }
    let width_mm = max_right / k - 10.0;
    assert!(width_mm > 90.0 && width_mm <= 100.0, "{width_mm}");
}

#[test]
fn invalid_barcodes_report_errors() {
    for build in [
        |p: &mut Fpdf| p.code_ean13("4006381333930", 0.0, 0.0, 0.0, 0.0),
        |p: &mut Fpdf| p.code39("a~b", 0.0, 0.0, 0.0, 0.0),
        |p: &mut Fpdf| p.code_i25("12x4", 0.0, 0.0, 0.0, 0.0),
        |p: &mut Fpdf| p.code128("\u{ff}", 0.0, 0.0, 0.0, 0.0),
    ] {
        let mut pdf = doc();
        build(&mut pdf);
        assert!(pdf.output().is_err());
    }
}

#[test]
fn qr_code_includes_a_quiet_zone_and_returns_its_size() {
    let mut pdf = doc();
    let side = pdf.qr_code(10.0, 10.0, "HELLO", 1.0, QrEcc::Low);
    // Version 1 is 21 modules; the quiet zone adds one on each side.
    assert_eq!(side, 23.0);
    pdf.qr_code_sized(50.0, 10.0, 30.0, "HELLO", QrEcc::High);
    assert!(pdf.error().is_none());
    let s = text(&pdf.output().expect("pdf"));
    assert!(count(&s, " re f\n") > 100);
}

#[test]
fn rotation_is_closed_at_the_end_of_the_page() {
    let mut pdf = doc();
    pdf.rotate(45.0, Some(50.0), Some(50.0));
    pdf.text(50.0, 50.0, "girado");
    let s = text(&pdf.output().expect("pdf"));
    assert!(s.contains("q 0.70711 0.70711 -0.70711 0.70711 "));
    assert!(s.contains(" cm 1 0 0 1 "));
    assert_eq!(count(&s, "\nQ\n"), 1, "the open rotation is closed");
}

#[test]
fn rounded_rect_corners_use_curves() {
    let mut pdf = doc();
    pdf.rounded_rect(10.0, 10.0, 40.0, 20.0, 4.0, "1234", "D");
    pdf.rounded_rect(60.0, 10.0, 40.0, 20.0, 4.0, "", "F");
    let s = text(&pdf.output().expect("pdf"));
    assert_eq!(count(&s, " c \n"), 4);
    assert!(s.contains("\nS\n") && s.contains("\nf\n"));
}

#[test]
fn dashes_are_set_and_reset() {
    let mut pdf = doc();
    pdf.dashed_line(10.0, 10.0, 100.0, 10.0, 2.0);
    let s = text(&pdf.output().expect("pdf"));
    let dash = s.find("[5.669 5.669] 0 d").expect("dash on");
    let line = s.find(" l S\n").expect("line");
    let reset = s.find("[] 0 d").expect("dash off");
    assert!(dash < line && line < reset);
}

#[test]
fn layers_are_registered_in_resources_and_catalog() {
    let mut pdf = doc();
    let on = pdf.add_layer("Visível", true);
    let off = pdf.add_layer("Oculta", false);
    pdf.begin_layer(on);
    pdf.text(10.0, 10.0, "a");
    pdf.begin_layer(off);
    pdf.text(10.0, 20.0, "b");
    pdf.open_layer_pane();
    let s = text(&pdf.output().expect("pdf"));
    assert!(s.starts_with("%PDF-1.5"));
    assert_eq!(count(&s, "/Type /OCG"), 2);
    assert!(s.contains("/OC /OC0 BDC") && s.contains("/OC /OC1 BDC"));
    assert_eq!(count(&s, "\nEMC\n"), 2, "each layer is closed");
    assert!(s.contains("/Properties <<"));
    assert!(s.contains("/OCProperties"));
    assert!(s.contains("/PageMode /UseOC"));
    // Invalid ids are errors.
    let mut bad = doc();
    bad.begin_layer(3);
    assert!(bad.output().is_err());
}

#[test]
fn write_html_applies_styles_and_links() {
    let mut pdf = doc();
    pdf.write_html("a <b>negrito</b> e <a href=\"https://example.com\">link</a><br>fim");
    let s = text(&pdf.output().expect("pdf"));
    assert!(s.contains("/BaseFont /Helvetica-Bold"));
    assert!(s.contains("/URI (https://example.com)"));
    assert!(s.contains("(fim) Tj"));
    // Malformed markup must terminate.
    let mut pdf = doc();
    pdf.write_html("texto <> mais < sem fim");
    assert!(pdf.output().is_ok());
}

#[test]
fn text_box_returns_the_used_height_and_respects_alignment() {
    let mut pdf = doc();
    let opts = TextBoxOpts {
        border: false,
        ..TextBoxOpts::default()
    };
    let one = pdf.text_box(10.0, 10.0, 80.0, 30.0, "uma linha", &opts);
    let three = pdf.text_box(10.0, 50.0, 80.0, 30.0, "a\nb\nc", &opts);
    assert!(three > one * 2.5, "{one} {three}");
    pdf.text_box(
        10.0,
        90.0,
        80.0,
        30.0,
        "centro",
        &TextBoxOpts {
            halign: HAlign::Center,
            valign: VAlign::Center,
            ..opts
        },
    );
    assert!(pdf.error().is_none());
}

#[test]
fn links_inside_text_boxes_and_pages_still_form_valid_documents() {
    let mut pdf = doc();
    pdf.set_draw_color(NamedColor::Red);
    pdf.text_box(10.0, 10.0, 50.0, 10.0, "caixa", &TextBoxOpts::default());
    let id = pdf.add_link();
    pdf.set_link(id, Some(0.0), None);
    pdf.link(10.0, 10.0, 50.0, 10.0, &Link::Internal(id));
    assert!(pdf.output().is_ok());
}

#[cfg(feature = "protection")]
#[test]
fn protected_documents_are_encrypted() {
    use fiscal_pdf::Permissions;

    let mut pdf = doc();
    pdf.set_title("Titulo secreto");
    pdf.text(10.0, 10.0, "conteudo secreto");
    pdf.set_protection(
        Permissions {
            print: true,
            ..Permissions::NONE
        },
        "",
        "dono",
    );
    let bytes = pdf.output().expect("pdf");
    let s = text(&bytes);
    assert!(s.contains("/Filter /Standard") && s.contains("/V 1") && s.contains("/R 2"));
    assert!(s.contains("/Encrypt ") && s.contains("/ID [()()]"));
    assert!(!s.contains("conteudo secreto"), "stream is encrypted");
    assert!(!s.contains("Titulo secreto"), "info strings are encrypted");
}
