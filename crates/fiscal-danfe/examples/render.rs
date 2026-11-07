//! Renders the DANFE of an NF-e XML file: `render <nfe.xml> [out.pdf]`.

use fiscal_danfe::{DanfeOptions, render_danfe_bytes};

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("usage: render <nfe.xml> [out.pdf]");
    let output = args.next().unwrap_or_else(|| "danfe.pdf".to_owned());
    let xml = std::fs::read(&input).expect("read the XML");
    let options = DanfeOptions {
        sistema: "fiscal-rs".to_owned(),
        ..DanfeOptions::default()
    };
    let pdf = render_danfe_bytes(&xml, &options).expect("render the DANFE");
    std::fs::write(&output, pdf).expect("write the PDF");
    println!("{output}");
}
