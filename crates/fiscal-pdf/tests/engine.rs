//! Behavioural tests of the PDF engine: object structure, content streams and text layout.

use fiscal_pdf::{
    Align, Border, CellOpts, Color, DateTime, Fpdf, ImageOpts, Link, Ln, NamedColor, Orientation,
    PageFormat, Unit,
};

fn doc() -> Fpdf {
    let mut pdf = Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4);
    pdf.set_compression(false);
    pdf.set_creation_date(DateTime {
        year: 2026,
        month: 10,
        day: 21,
        hour: 9,
        minute: 30,
        second: 0,
    });
    pdf
}

fn text(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

/// Verifies the cross-reference table: every offset points at `<n> 0 obj`.
fn assert_valid_structure(bytes: &[u8]) {
    let find_last = |needle: &[u8]| {
        bytes
            .windows(needle.len())
            .rposition(|w| w == needle)
            .expect("marker present")
    };
    assert!(bytes.starts_with(b"%PDF-1."), "header");
    assert!(bytes.ends_with(b"%%EOF\n"), "trailer");
    let start = find_last(b"startxref\n") + "startxref\n".len();
    let tail = String::from_utf8_lossy(&bytes[start..]).into_owned();
    let xref_pos: usize = tail
        .lines()
        .next()
        .expect("offset")
        .parse()
        .expect("number");
    assert!(bytes[xref_pos..].starts_with(b"xref\n"), "xref offset");
    // The cross-reference table itself is plain ASCII.
    let table = String::from_utf8_lossy(&bytes[xref_pos..]).into_owned();
    let mut lines = table.lines().skip(1);
    let count: usize = lines
        .next()
        .expect("count")
        .split(' ')
        .nth(1)
        .expect("size")
        .parse()
        .expect("number");
    assert_eq!(lines.next(), Some("0000000000 65535 f "));
    for n in 1..count {
        let entry = lines.next().expect("entry");
        let offset: usize = entry[..10].parse().expect("offset");
        let expected = format!("{n} 0 obj\n");
        assert!(
            bytes[offset..].starts_with(expected.as_bytes()),
            "object {n} at {offset}"
        );
    }
}

#[test]
fn hello_world_has_valid_structure_and_exact_content() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Arial", "B", 16.0);
    pdf.cell(40.0, 10.0, "Hello World!");
    let bytes = pdf.output().expect("pdf");
    assert_valid_structure(&bytes);
    let s = text(&bytes);
    assert!(s.starts_with("%PDF-1.3\n"));
    assert!(
        s.contains("2 J\n0.57 w\nBT /F1 16.00 Tf ET\nBT 31.19 794.57 Td (Hello World!) Tj ET\n")
    );
    assert!(s.contains("/BaseFont /Helvetica-Bold"));
    assert!(s.contains("/Encoding /WinAnsiEncoding"));
    assert!(s.contains("/CreationDate (D:20261021093000Z)"));
    assert!(s.contains("/MediaBox [0 0 595.28 841.89]"));
    assert!(s.contains("/Count 1"));
}

#[test]
fn output_is_deterministic_with_fixed_creation_date() {
    let build = || {
        let mut pdf = doc();
        pdf.add_page();
        pdf.set_font("Times", "", 12.0);
        pdf.cell(0.0, 6.0, "Mesmo conteúdo");
        pdf.output().expect("pdf")
    };
    assert_eq!(build(), build());
}

#[test]
fn text_is_encoded_in_cp1252_and_escaped() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Helvetica", "", 10.0);
    pdf.text(10.0, 20.0, "Ação (nº 1) \\ €");
    let bytes = pdf.output().expect("pdf");
    let needle: &[u8] = b"(A\xE7\xE3o \\(n\xBA 1\\) \\\\ \x80) Tj";
    assert!(bytes.windows(needle.len()).any(|w| w == needle));
}

#[test]
fn string_width_uses_core_font_metrics() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Helvetica", "", 10.0);
    // H=722 e=556 l=222 l=222 o=556 -> 2278 / 1000 * 10pt, converted to mm.
    let w = pdf.get_string_width("Hello");
    assert!((w - 22.78 / (72.0 / 25.4)).abs() < 1e-9, "{w}");
    pdf.set_font("Courier", "", 10.0);
    assert!((pdf.get_string_width("abc") - 18.0 / (72.0 / 25.4)).abs() < 1e-9);
    // The widest line wins.
    assert_eq!(
        pdf.get_string_width("abc\nabcdef"),
        pdf.get_string_width("abcdef")
    );
}

#[test]
fn drawing_operators() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_draw_color(NamedColor::Red);
    pdf.set_fill_color(Color::Gray(128));
    pdf.set_line_width(1.0);
    pdf.line(10.0, 10.0, 100.0, 10.0);
    pdf.rect(10.0, 20.0, 50.0, 30.0, "FD");
    let s = text(&pdf.output().expect("pdf"));
    assert!(s.contains("1.000 0.000 0.000 RG\n0.502 g\n2.83 w\n"));
    assert!(s.contains("28.35 813.54 m 283.46 813.54 l S\n"));
    assert!(s.contains("28.35 785.20 141.73 -85.04 re B\n"));
}

#[test]
fn cell_borders_fill_and_cursor_movement() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Helvetica", "", 12.0);
    let (x0, y0) = (pdf.x(), pdf.y());
    pdf.cell_with(
        30.0,
        8.0,
        "",
        &CellOpts {
            border: Border::FRAME,
            fill: true,
            ..CellOpts::default()
        },
    );
    assert!((pdf.x() - (x0 + 30.0)).abs() < 1e-9);
    pdf.cell_with(
        30.0,
        8.0,
        "",
        &CellOpts {
            ln: Ln::NextLine,
            ..CellOpts::default()
        },
    );
    assert!((pdf.x() - x0).abs() < 1e-9);
    assert!((pdf.y() - (y0 + 8.0)).abs() < 1e-9);
    pdf.cell_with(
        "LT".len() as f64 * 0.0 + 20.0,
        5.0,
        "",
        &CellOpts {
            border: "LT".into(),
            ..CellOpts::default()
        },
    );
    let s = text(&pdf.output().expect("pdf"));
    assert!(
        s.contains(" re B \n") || s.contains(" re B "),
        "frame+fill uses the B operator"
    );
    assert_eq!(s.matches(" l S ").count(), 2);
}

#[test]
fn alignment_moves_the_text_origin() {
    let draw = |align: Align| {
        let mut pdf = doc();
        pdf.add_page();
        pdf.set_font("Courier", "", 10.0);
        pdf.cell_with(
            50.0,
            10.0,
            "ab",
            &CellOpts {
                align,
                ..CellOpts::default()
            },
        );
        text(&pdf.output().expect("pdf"))
    };
    let origin = |s: &str| {
        let line = s
            .lines()
            .find(|l| l.contains("Td (ab)"))
            .expect("text op")
            .to_owned();
        line.split(' ')
            .nth(1)
            .expect("x")
            .parse::<f64>()
            .expect("number")
    };
    let (left, center, right) = (
        origin(&draw(Align::Left)),
        origin(&draw(Align::Center)),
        origin(&draw(Align::Right)),
    );
    assert!(left < center && center < right, "{left} {center} {right}");
}

#[test]
fn multi_cell_wraps_and_justifies() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Helvetica", "", 10.0);
    let before = pdf.y();
    pdf.multi_cell(
        40.0,
        5.0,
        "Texto bem comprido que precisa quebrar em varias linhas dentro da celula",
        Border::NONE,
        Align::Justify,
        false,
    );
    let lines = ((pdf.y() - before) / 5.0).round() as i32;
    assert!(lines >= 4, "wrapped into {lines} lines");
    assert!(
        (pdf.x() - 10.0016).abs() < 0.01,
        "x returns to the left margin"
    );
    let s = text(&pdf.output().expect("pdf"));
    assert!(
        s.contains(" Tw\n") || s.contains(" Tw "),
        "justified lines set word spacing"
    );
    assert!(s.contains("0 Tw"));
}

#[test]
fn multi_cell_keeps_the_last_char_after_an_inner_newline() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Helvetica", "", 10.0);
    pdf.multi_cell(80.0, 5.0, "ab\nc", Border::NONE, Align::Left, false);
    let s = text(&pdf.output().expect("pdf"));
    assert!(s.contains("(ab) Tj"));
    assert!(
        s.contains("(c) Tj"),
        "the final character must not be dropped"
    );
}

#[test]
fn multi_cell_borders_cover_first_middle_and_last_lines() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Helvetica", "", 10.0);
    pdf.multi_cell(
        30.0,
        5.0,
        "um\ndois\ntres",
        Border::FRAME,
        Align::Left,
        false,
    );
    let s = text(&pdf.output().expect("pdf"));
    // 3 lines: top+left+right, left+right, left+right+bottom -> 3+2+3 = 8 segments.
    assert_eq!(s.matches(" l S ").count(), 8);
}

#[test]
fn automatic_page_break_and_footer_alias() {
    let mut pdf = doc();
    pdf.set_alias_nb_pages("{nb}");
    pdf.set_footer(|p| {
        p.set_y(-15.0, true);
        p.set_font("Helvetica", "I", 8.0);
        let label = format!("Pagina {}/{{nb}}", p.page_no());
        p.cell(0.0, 10.0, &label);
    });
    pdf.add_page();
    pdf.set_font("Helvetica", "", 12.0);
    for i in 0..80 {
        pdf.cell_with(
            0.0,
            6.0,
            &format!("Linha {i}"),
            &CellOpts {
                ln: Ln::NextLine,
                ..CellOpts::default()
            },
        );
    }
    assert!(pdf.page_no() >= 2);
    let pages = pdf.page_no();
    let bytes = pdf.output().expect("pdf");
    assert_valid_structure(&bytes);
    let s = text(&bytes);
    assert!(s.contains(&format!("(Pagina 1/{pages}) Tj")));
    assert!(s.contains(&format!("/Count {pages}")));
    assert!(!s.contains("{nb}"));
}

#[test]
fn header_runs_on_each_page_and_restores_font() {
    let mut pdf = doc();
    pdf.set_header(|p| {
        p.set_font("Courier", "B", 9.0);
        p.cell(0.0, 5.0, "CABECALHO");
    });
    pdf.add_page();
    pdf.set_font("Helvetica", "", 12.0);
    pdf.add_page();
    let s = text(&pdf.output().expect("pdf"));
    assert_eq!(s.matches("(CABECALHO) Tj").count(), 2);
    assert!(s.contains("/BaseFont /Courier-Bold"));
    assert!(s.contains("/BaseFont /Helvetica"));
}

#[test]
fn landscape_pages_get_their_own_media_box() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.add_page_orientation(Orientation::Landscape);
    assert!((pdf.page_width() - 297.0).abs() < 0.01);
    let s = text(&pdf.output().expect("pdf"));
    assert!(s.contains("/MediaBox [0 0 841.89 595.28]") || s.contains("/MediaBox [0 0 841.889"));
}

#[test]
fn links_become_annotations() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Helvetica", "U", 12.0);
    pdf.cell_with(
        40.0,
        8.0,
        "site",
        &CellOpts {
            link: Some(Link::Uri("https://example.com".into())),
            ..CellOpts::default()
        },
    );
    let id = pdf.add_link();
    pdf.set_link(id, Some(0.0), None);
    pdf.link(10.0, 100.0, 20.0, 5.0, &Link::Internal(id));
    let bytes = pdf.output().expect("pdf");
    assert_valid_structure(&bytes);
    let s = text(&bytes);
    assert!(s.contains("/Subtype /Link"));
    assert!(s.contains("/A <</S /URI /URI (https://example.com)>>"));
    assert!(s.contains("/Dest ["));
    assert!(s.contains("/Annots ["));
}

#[test]
fn errors_are_sticky_and_reported_by_output() {
    let mut pdf = doc();
    pdf.add_page();
    pdf.cell(10.0, 5.0, "sem fonte");
    assert!(pdf.error().is_some());
    pdf.set_font("Helvetica", "", 10.0); // later calls are accepted but the error remains
    let err = pdf.output().expect_err("no font selected");
    assert!(err.to_string().contains("No font has been set"));

    let mut pdf = doc();
    pdf.add_page();
    pdf.set_font("Nope", "", 10.0);
    assert!(
        pdf.output()
            .expect_err("bad font")
            .to_string()
            .contains("Undefined font")
    );

    let mut pdf = doc();
    pdf.add_page();
    pdf.image_bytes(b"garbage", "png", &ImageOpts::default());
    assert!(pdf.output().is_err());
}

#[test]
fn embeds_png_with_alpha_as_soft_mask() {
    // 1x1 RGBA PNG built by hand (CRCs are not verified by the parser).
    let rgba = [0u8, 255, 0, 0, 128];
    let mut z = flate_encode(&rgba);
    let mut png = vec![137, b'P', b'N', b'G', 13, 10, 26, 10];
    let mut chunk = |kind: &[u8; 4], body: &[u8]| {
        png.extend_from_slice(&(body.len() as u32).to_be_bytes());
        png.extend_from_slice(kind);
        png.extend_from_slice(body);
        png.extend_from_slice(&[0; 4]);
    };
    chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]);
    chunk(b"IDAT", &std::mem::take(&mut z));
    chunk(b"IEND", &[]);

    let mut pdf = doc();
    pdf.add_page();
    let opts = ImageOpts {
        x: Some(10.0),
        y: Some(10.0),
        width: 20.0,
        height: 0.0,
        link: None,
    };
    pdf.image_bytes(&png, "png", &opts);
    pdf.image_bytes(
        &png,
        "png",
        &ImageOpts {
            x: Some(40.0),
            ..opts.clone()
        },
    );
    let bytes = pdf.output().expect("pdf");
    assert_valid_structure(&bytes);
    let s = text(&bytes);
    assert!(s.starts_with("%PDF-1.4"), "alpha needs PDF 1.4");
    assert!(s.contains("/SMask"));
    assert!(s.contains("/Group <</Type /Group /S /Transparency /CS /DeviceRGB>>"));
    assert_eq!(
        s.matches("/Subtype /Image").count(),
        2,
        "image + mask, embedded once"
    );
    assert_eq!(s.matches(" Do Q").count(), 2, "but drawn twice");
}

fn flate_encode(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data).expect("write");
    enc.finish().expect("finish")
}
