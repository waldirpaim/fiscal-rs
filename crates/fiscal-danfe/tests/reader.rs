//! Reading the synthetic fixture.

use fiscal_danfe::model::{Crt, TpAmb, TpEmis, TpImp, TpNf};
use fiscal_danfe::{Decimal, Nfe};

const XML: &str = include_str!("fixtures/nfe_55.xml");

fn d(s: &str) -> Decimal {
    Decimal::parse(s).expect("decimal")
}

#[test]
fn reads_the_header_blocks() {
    let nfe = Nfe::from_xml(XML).expect("nfe");
    assert_eq!(nfe.chave(), "35260711222333000181550010000012341123456780");
    assert!(nfe.id.starts_with("NFe"));
    assert_eq!(nfe.versao, 4.0);
    assert_eq!(nfe.ide.modelo, 55);
    assert_eq!(nfe.ide.serie, 1);
    assert_eq!(nfe.ide.n_nf, 1234);
    assert_eq!(nfe.ide.tp_nf, TpNf::Saida);
    assert_eq!(nfe.ide.tp_imp, TpImp::Retrato);
    assert_eq!(nfe.ide.tp_emis, TpEmis::Normal);
    assert_eq!(nfe.ide.tp_amb, TpAmb::Producao);
    assert_eq!(
        nfe.ide.d_emi.map(|d| d.date_br()).as_deref(),
        Some("20/07/2026")
    );
    assert_eq!(
        nfe.ide.d_sai_ent.map(|d| d.time_string()).as_deref(),
        Some("11:05:00")
    );
    assert_eq!(nfe.emit.crt, Crt::RegimeNormal);
    assert_eq!(nfe.emit.cnpj_cpf, "11222333000181");
    assert_eq!(nfe.emit.ender_emit.cep, 1001000);
    assert_eq!(nfe.emit.iest, "999888777666");
    assert_eq!(nfe.dest.x_nome, "CLIENTE MODELO DISTRIBUIDORA S/A");
    assert_eq!(nfe.dest.ender_dest.uf, "RJ");
    assert!(nfe.entrega.cnpj_cpf.is_empty());
}

#[test]
fn reads_items_and_taxes() {
    let nfe = Nfe::from_xml(XML).expect("nfe");
    assert_eq!(nfe.det.len(), 3);
    let item = &nfe.det[0];
    assert_eq!(item.n_item, 1);
    assert_eq!(
        item.prod.x_prod,
        "PARAFUSO SEXTAVADO ACO INOX 8MM CAIXA COM 100 UNIDADES"
    );
    assert_eq!(item.prod.q_com, d("10"));
    assert_eq!(item.prod.v_un_com.format_br(4, true), "45,5000");
    assert_eq!(item.imposto.icms.cst, "00");
    assert_eq!(item.imposto.icms.v_icms, d("81.90"));
    assert_eq!(item.imposto.ipi.v_ipi, d("22.75"));
    assert_eq!(item.imposto.v_tot_trib, d("82.90"));
    assert_eq!(item.inf_ad_prod, "Lote 2026-07 validade indeterminada");
    assert_eq!(nfe.det[1].prod.v_desc, d("1.70"));
    assert_eq!(nfe.det[1].imposto.icms.orig, "2");
    assert_eq!(nfe.det[2].prod.x_prod, "FRETE & INSTALAÇÃO");
    assert_eq!(nfe.det[2].imposto.icms.cst, "41");
}

#[test]
fn reads_totals_transport_billing_payment_and_protocol() {
    let nfe = Nfe::from_xml(XML).expect("nfe");
    assert_eq!(nfe.total.icms_tot.v_nf, d("664.75"));
    assert_eq!(nfe.total.icms_tot.v_ipi, d("22.75"));
    assert_eq!(nfe.transp.mod_frete, 1);
    assert_eq!(nfe.transp.transporta.x_nome, "TRANSPORTES RAPIDO LTDA");
    assert_eq!(nfe.transp.veic_transp.placa, "ABC1D23");
    assert_eq!(nfe.transp.vol[0].q_vol, 2);
    assert_eq!(nfe.transp.vol[0].peso_b, d("14.250"));
    assert_eq!(nfe.cobr.fat.n_fat, "001234");
    assert_eq!(nfe.cobr.dup.len(), 3);
    assert_eq!(
        nfe.cobr.dup[1].d_venc.map(|d| d.date_br()).as_deref(),
        Some("20/09/2026")
    );
    assert_eq!(nfe.pag.len(), 1);
    assert_eq!(nfe.pag[0].t_pag, "15");
    assert_eq!(nfe.inf_adic.obs_cont[0].x_campo, "Vendedor");
    assert_eq!(nfe.inf_adic.obs_cont[0].x_texto, "Maria Souza");
    assert!(nfe.inf_adic.inf_cpl.starts_with("Pedido de compra 4521."));
    assert_eq!(nfe.proc_nfe.n_prot, "135260000012345");
    assert_eq!(nfe.proc_nfe.c_stat, 100);
    assert_eq!(
        nfe.proc_nfe.dh_recbto.map(|d| d.datetime_br()).as_deref(),
        Some("20/07/2026 10:33:40")
    );
}

#[test]
fn reads_a_bare_nfe_without_protocol() {
    let start = XML.find("<NFe ").expect("NFe");
    let end = XML.find("</NFe>").expect("end") + "</NFe>".len();
    let nfe = Nfe::from_xml(&XML[start..end]).expect("nfe");
    assert!(nfe.proc_nfe.n_prot.is_empty());
    assert_eq!(nfe.det.len(), 3);
}

#[test]
fn rejects_documents_that_are_not_an_nfe() {
    assert!(Nfe::from_xml("<foo/>").is_err());
    assert!(Nfe::from_xml("<NFe/>").is_err());
    assert!(Nfe::from_xml("not xml").is_err());
}

#[test]
fn decodes_windows_1252_bytes() {
    let xml = XML
        .replace("FRETE &amp; INSTALAÇÃO", "TAÇA")
        .replace("UTF-8", "ISO-8859-1");
    // Ç and Ã in Windows-1252.
    let bytes: Vec<u8> = xml
        .chars()
        .map(|c| match c {
            'Ç' => 0xC7,
            'Ã' => 0xC3,
            c => c as u8,
        })
        .collect();
    let nfe = Nfe::from_xml_bytes(&bytes).expect("nfe");
    assert_eq!(nfe.det[2].prod.x_prod, "TAÇA");
}
