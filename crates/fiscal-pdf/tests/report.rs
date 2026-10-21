//! Tests of the band-based report engine.

use std::sync::{Arc, Mutex};

use fiscal_pdf::report::{
    Band, BandHandler, BandState, BandType, DrawArgs, Engine, InitArgs, Report,
};
use fiscal_pdf::{Orientation, PageFormat, Unit};

fn text(bytes: &[u8]) -> String {
    bytes.iter().map(|b| char::from(*b)).collect()
}

fn render(report: &mut Report) -> String {
    let mut engine = Engine::new(report);
    engine.set_compressed(false);
    text(&engine.render().expect("pdf"))
}

fn a4() -> Report {
    let mut report = Report::new();
    report.set_font("Helvetica");
    report.set_margins(10.0, 10.0, None, None);
    report.add_page(Orientation::Portrait, Unit::Mm, PageFormat::A4);
    report
}

/// A data band that prints a label in a fixed-height cell.
struct Row(usize);

impl BandHandler for Row {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.height = 20.0;
    }

    fn on_draw(&mut self, _band: &mut BandState, args: &mut DrawArgs<'_>) {
        args.pdf.set_font("Helvetica", "", 10.0);
        args.pdf.text_box(
            0.0,
            0.0,
            50.0,
            18.0,
            &format!("ROW-{:03}", self.0),
            'C',
            'C',
            1.0,
        );
    }
}

#[test]
fn data_bands_flow_across_pages_with_footer_and_total() {
    let mut report = a4();
    report.engine_options.double_pass = true;
    report
        .add_anonymous_band(BandType::PageHeader, 10.0, |_, args| {
            args.pdf.set_font("Helvetica", "B", 12.0);
            let label = format!("Folha {}/{}", args.pdf.current_page(), args.total_pages);
            args.pdf
                .text_box(0.0, 0.0, 60.0, 8.0, &label, 'T', 'L', 0.0);
        })
        .expect("header");
    for i in 0..30 {
        report
            .add_band(Band::new(BandType::Data, Row(i)))
            .expect("row");
    }
    report
        .add_anonymous_band(BandType::PageFooter, 6.0, |_, args| {
            args.pdf.set_font("Helvetica", "I", 8.0);
            args.pdf
                .text_box(0.0, 0.0, 60.0, 5.0, "RODAPE", 'T', 'L', 0.0);
        })
        .expect("footer");

    let s = render(&mut report);
    // 297 - 20 margins - 10 header - 6 footer = 261 mm -> 13 rows of 20 mm per page.
    assert!(s.contains("/Count 3"), "{}", &s[..200.min(s.len())]);
    for i in 0..30 {
        assert!(
            s.contains(&format!("(ROW-{i:03}) Tj")),
            "row {i} is missing"
        );
    }
    // Rows keep their order.
    let first = s.find("(ROW-000) Tj").expect("first");
    let last = s.find("(ROW-029) Tj").expect("last");
    assert!(first < last);
    assert_eq!(s.matches("(RODAPE) Tj").count(), 3);
    for page in 1..=3 {
        assert!(
            s.contains(&format!("(Folha {page}/3) Tj")),
            "page {page} label"
        );
    }
}

#[test]
fn single_pass_does_not_know_the_total() {
    let mut report = a4();
    report
        .add_anonymous_band(BandType::PageHeader, 10.0, |_, args| {
            let label = format!("total={}", args.total_pages);
            args.pdf.set_font("Helvetica", "", 8.0);
            args.pdf
                .text_box(0.0, 0.0, 60.0, 8.0, &label, 'T', 'L', 0.0);
        })
        .expect("header");
    report
        .add_band(Band::new(BandType::Data, Row(0)))
        .expect("row");
    let s = render(&mut report);
    assert!(s.contains("(total=0) Tj"));
}

/// A flowing band that draws one line per item, asking to be drawn again when the page is
/// full (the products table of the DANFE works this way).
struct Table {
    next: usize,
    items: usize,
    log: Arc<Mutex<Vec<(usize, usize)>>>,
}

impl BandHandler for Table {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        self.next = 0;
        band.auto_height = true;
        band.height = 12.0;
    }

    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let free = args.free_space;
        if free <= 0.0 {
            return;
        }
        band.height = free;
        args.pdf.set_font("Helvetica", "", 8.0);
        let mut y = 0.0;
        let page = args.pdf.current_page();
        while self.next < self.items {
            if y + 6.0 > free {
                break;
            }
            args.pdf.text_box(
                0.0,
                y,
                40.0,
                6.0,
                &format!("ITEM-{:03}", self.next),
                'T',
                'L',
                1.0,
            );
            y += 6.0;
            self.next += 1;
            self.log.lock().expect("log").push((page, self.next));
        }
        args.draw_again = self.next < self.items;
    }
}

#[test]
fn a_band_can_ask_to_be_drawn_again_on_the_next_page() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut report = a4();
    report
        .add_band(Band::new(
            BandType::Data,
            Table {
                next: 0,
                items: 100,
                log: Arc::clone(&log),
            },
        ))
        .expect("table");
    report
        .add_anonymous_band(BandType::PageFooter, 6.0, |_, args| {
            args.pdf.set_font("Helvetica", "", 8.0);
            args.pdf.text_box(0.0, 0.0, 30.0, 5.0, "FIM", 'T', 'L', 0.0);
        })
        .expect("footer");
    let s = render(&mut report);
    for i in 0..100 {
        assert!(s.contains(&format!("(ITEM-{i:03}) Tj")), "item {i}");
    }
    let pages = log
        .lock()
        .expect("log")
        .iter()
        .map(|(p, _)| *p)
        .max()
        .unwrap_or(0);
    assert!(
        pages >= 3,
        "100 items of 6 mm do not fit in two pages ({pages})"
    );
    assert!(s.contains(&format!("/Count {pages}")));
}

#[test]
fn margin_bands_shift_the_body() {
    let mut report = a4();
    report
        .add_anonymous_band(BandType::TopMargin, 15.0, |_, args| {
            args.pdf.set_font("Helvetica", "", 8.0);
            args.pdf
                .text_box(0.0, 0.0, 40.0, 10.0, "TOPO", 'T', 'L', 0.0);
        })
        .expect("top");
    report
        .add_band(Band::new(BandType::Data, Row(1)))
        .expect("row");
    let s = render(&mut report);
    // Top margin 10 + band 15: the row text sits below y = 25 mm (297 - 25 = 272 mm -> 771 pt).
    let row_line = s.lines().find(|l| l.contains("(ROW-001) Tj")).expect("row");
    let y: f64 = row_line
        .split(' ')
        .nth(2)
        .expect("y")
        .parse()
        .expect("number");
    assert!(y < (297.0 - 25.0) * 72.0 / 25.4, "row at y={y}");
    assert!(s.contains("(TOPO) Tj"));
}

#[test]
fn landscape_pages_are_wider_than_tall() {
    let mut report = Report::new();
    report.set_font("Helvetica");
    report.set_margins(5.0, 5.0, None, None);
    report.add_page(Orientation::Landscape, Unit::Mm, PageFormat::A4);
    report
        .add_band(Band::new(BandType::Data, Row(7)))
        .expect("row");
    let s = render(&mut report);
    assert!(s.contains("/MediaBox [0 0 841.89 595.28]"), "{s}");
}

#[test]
fn bands_added_lazily_by_the_start_hook_are_drawn_once() {
    let mut report = Report::new();
    report.set_font("Helvetica");
    report.engine_options.double_pass = true;
    let initialised = Arc::new(Mutex::new(false));
    let flag = Arc::clone(&initialised);
    report.set_on_start(move |report, _| {
        let mut done = flag.lock().expect("flag");
        if *done {
            return;
        }
        report.set_margins(10.0, 10.0, None, None);
        report.add_page(Orientation::Portrait, Unit::Mm, PageFormat::A4);
        report
            .add_band(Band::new(BandType::Data, Row(5)))
            .expect("row");
        *done = true;
    });
    let s = render(&mut report);
    assert_eq!(s.matches("(ROW-005) Tj").count(), 1);
}

#[test]
fn zero_height_bands_are_rejected() {
    struct Broken;
    impl BandHandler for Broken {}
    let mut report = a4();
    report
        .add_band(Band::new(BandType::Data, Broken))
        .expect("band");
    let mut engine = Engine::new(&mut report);
    let err = engine.render().expect_err("height 0");
    assert!(
        err.to_string().contains("Height must be greater than 0"),
        "{err}"
    );
}

#[test]
fn a_band_that_never_fits_ends_with_an_error() {
    struct Giant;
    impl BandHandler for Giant {
        fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
            band.height = 10_000.0;
        }
    }
    let mut report = a4();
    report
        .add_band(Band::new(BandType::Data, Giant))
        .expect("band");
    let mut engine = Engine::new(&mut report);
    assert!(engine.render().is_err());
}

#[test]
fn endless_pages_grow_to_the_content() {
    let mut report = Report::new();
    report.set_font("Helvetica");
    report.set_margins(2.0, 2.0, None, None);
    let page = report.add_page_with_dimensions(Orientation::Portrait, Unit::Mm, 80.0, 100.0);
    page.endless_height = true;
    for i in 0..10 {
        report
            .add_band(Band::new(BandType::Data, Row(i)))
            .expect("row");
    }
    let s = render(&mut report);
    assert!(s.contains("/Count 1"));
    // 10 rows x 20 mm + 2 mm top margin + 2 mm bottom margin = 204 mm = 578.27 pt.
    let height = (200.0 + 2.0 + 2.0) * 72.0 / 25.4;
    assert!(
        s.contains(&format!("{:.2}]", height)) || s.contains("578.26") || s.contains("578.27"),
        "{s}"
    );
}
