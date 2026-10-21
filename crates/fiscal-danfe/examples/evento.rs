//! Renders the DANFE of an event: `evento <nfe.xml> <procEventoNFe.xml> [out.pdf]`.

use fiscal_danfe::{DanfeOptions, render_evento_xml};

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: evento <nfe.xml> <procEventoNFe.xml> [out.pdf]";
    let nfe = args.next().expect(usage);
    let evento = args.next().expect(usage);
    let output = args.next().unwrap_or_else(|| "evento.pdf".to_owned());
    let options = DanfeOptions {
        sistema: "fiscal-rs".to_owned(),
        ..DanfeOptions::default()
    };
    let pdf = render_evento_xml(
        &std::fs::read(nfe).expect("read the NF-e"),
        &std::fs::read(evento).expect("read the event"),
        &options,
    )
    .expect("render the event");
    std::fs::write(&output, pdf).expect("write the PDF");
    println!("{output}");
}
