//! Renders a small showcase of the engine to the path given as first argument
//! (default: `showcase.pdf`).

use fiscal_pdf::{
    Align, Border, CellOpts, Color, Fpdf, Link, Ln, NamedColor, Orientation, PageFormat, Unit,
};

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "showcase.pdf".to_owned());
    let mut pdf = Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4);
    pdf.set_title("Showcase fiscal-pdf");
    pdf.set_author("fiscal-rs");
    pdf.set_alias_nb_pages("{nb}");
    pdf.set_footer(|p| {
        p.set_y(-15.0, true);
        p.set_font("Helvetica", "I", 8.0);
        let label = format!("Página {}/{{nb}}", p.page_no());
        p.cell_with(
            0.0,
            10.0,
            &label,
            &CellOpts {
                align: Align::Center,
                ..CellOpts::default()
            },
        );
    });
    pdf.add_page();

    pdf.set_font("Helvetica", "B", 16.0);
    pdf.cell_with(
        0.0,
        10.0,
        "fiscal-pdf — motor de PDF estilo FPDF",
        &CellOpts {
            ln: Ln::NextLine,
            ..CellOpts::default()
        },
    );

    for (family, style) in [
        ("Helvetica", ""),
        ("Helvetica", "B"),
        ("Times", "I"),
        ("Courier", "BI"),
    ] {
        pdf.set_font(family, style, 12.0);
        pdf.cell_with(
            0.0,
            7.0,
            &format!("{family} {style}: Ação, coração, NF-e nº 12345 – R$ 1.234,56"),
            &CellOpts {
                ln: Ln::NextLine,
                ..CellOpts::default()
            },
        );
    }

    pdf.ln(3.0);
    pdf.set_font("Helvetica", "", 10.0);
    pdf.set_fill_color(NamedColor::LightGrey);
    pdf.set_draw_color(Color::Gray(100));
    pdf.cell_with(
        60.0,
        8.0,
        "Com borda e fundo",
        &CellOpts {
            border: Border::FRAME,
            fill: true,
            ..CellOpts::default()
        },
    );
    pdf.cell_with(
        60.0,
        8.0,
        "Só lados L e B",
        &CellOpts {
            border: "LB".into(),
            align: Align::Center,
            ..CellOpts::default()
        },
    );
    pdf.cell_with(
        60.0,
        8.0,
        "Direita",
        &CellOpts {
            border: "TR".into(),
            align: Align::Right,
            ln: Ln::NextLine,
            ..CellOpts::default()
        },
    );

    pdf.ln(3.0);
    let lorem = "Este parágrafo é justificado dentro de uma coluna estreita para exercitar a quebra automática de linhas do MultiCell, incluindo o espaçamento de palavras (Tw) e as bordas laterais.";
    pdf.multi_cell(90.0, 5.0, lorem, Border::FRAME, Align::Justify, false);
    pdf.ln(2.0);
    pdf.multi_cell_indent(90.0, 5.0, lorem, Border::NONE, Align::Left, false, 8.0);

    pdf.ln(4.0);
    pdf.set_text_color(Color::Rgb(0, 0, 200));
    pdf.set_font("Helvetica", "U", 11.0);
    pdf.write(
        6.0,
        "Link para o projeto fiscal-rs",
        Some(&Link::Uri("https://github.com/waldirpaim/fiscal-rs".into())),
    );
    pdf.set_text_color(Color::BLACK);

    pdf.set_font("Helvetica", "", 10.0);
    pdf.set_draw_color(NamedColor::Red);
    pdf.set_line_width(0.6);
    pdf.line(15.0, 200.0, 195.0, 200.0);
    pdf.rect(15.0, 205.0, 60.0, 20.0, "D");
    pdf.set_fill_color(NamedColor::Aqua);
    pdf.rect(80.0, 205.0, 60.0, 20.0, "F");

    pdf.add_page();
    pdf.set_font("Helvetica", "", 12.0);
    for i in 1..=60 {
        pdf.cell_with(
            0.0,
            6.0,
            &format!("Linha {i} — a segunda página nasce da quebra automática"),
            &CellOpts {
                ln: Ln::NextLine,
                ..CellOpts::default()
            },
        );
    }

    pdf.save_to_file(&out).expect("write showcase");
    println!("{out}");
}
