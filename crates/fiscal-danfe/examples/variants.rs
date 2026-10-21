//! Renders layout variants of the fixture for visual inspection:
//! `variants <nfe.xml> <out-dir>`.

use fiscal_danfe::options::PosCanhoto;
use fiscal_danfe::{DanfeOptions, render_danfe};

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("usage: variants <nfe.xml> <out-dir>");
    let out = args.next().expect("usage: variants <nfe.xml> <out-dir>");
    let xml = std::fs::read_to_string(&input).expect("read");
    let base = DanfeOptions {
        sistema: "fiscal-rs|DANFE|variantes".to_owned(),
        ..DanfeOptions::default()
    };

    let first = xml.find("<det nItem=\"1\">").expect("item");
    let end = xml.find("</det>").expect("end") + "</det>".len();
    let item = &xml[first..end];
    let many: String = (1..=70)
        .map(|i| {
            item.replace("nItem=\"1\"", &format!("nItem=\"{i}\""))
                .replace("<cProd>0001</cProd>", &format!("<cProd>ITEM{i:03}</cProd>"))
        })
        .collect();
    let last_end = xml.rfind("</det>").expect("last") + "</det>".len();
    let big = format!("{}{}{}", &xml[..first], many, &xml[last_end..]);

    let cases: Vec<(&str, String, DanfeOptions)> = vec![
        ("multi", big, base.clone()),
        (
            "landscape",
            xml.replace("<tpImp>1</tpImp>", "<tpImp>2</tpImp>"),
            base.clone(),
        ),
        (
            "left",
            xml.clone(),
            DanfeOptions {
                pos_canhoto: PosCanhoto::Esquerda,
                ..base.clone()
            },
        ),
        (
            "bottom",
            xml.clone(),
            DanfeOptions {
                pos_canhoto: PosCanhoto::Rodape,
                ..base.clone()
            },
        ),
        (
            "homolog",
            xml.replace("<tpAmb>1</tpAmb><finNFe>", "<tpAmb>2</tpAmb><finNFe>"),
            base.clone(),
        ),
        (
            "cancelled",
            xml.replace("<cStat>100</cStat>", "<cStat>101</cStat>"),
            DanfeOptions {
                exibe_total_tributos_item: true,
                exibe_campo_de_pagamento: fiscal_danfe::options::InformacoesDePagamento::Quadro,
                ..base.clone()
            },
        ),
        (
            "contingency",
            xml.replace("<tpEmis>1</tpEmis>", "<tpEmis>5</tpEmis>"),
            base,
        ),
    ];
    for (name, xml, options) in cases {
        let pdf = render_danfe(&xml, &options).expect(name);
        std::fs::write(format!("{out}/{name}.pdf"), pdf).expect("write");
    }
}
