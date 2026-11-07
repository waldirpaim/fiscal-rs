//! The DANFE NFC-e: the receipt printed for consumer invoices (model 65).
//!
//! The receipt is one *endless* page (as tall as its content) made of data bands that draw
//! themselves and report the space they used.

use std::sync::Arc;

use fiscal_pdf::report::{BandState, BandType, DrawArgs, Engine, Report, ReportPdf, image_size};
use fiscal_pdf::{Color, NamedColor, Orientation, QrEcc, Unit, round_to};

use crate::context::DanfeContext;
use crate::conversions;
use crate::decimal::Decimal;
use crate::error::DanfeError;
use crate::format;
use crate::model::{Nfe, TpAmb, TpEmis, TpImp};
use crate::options::DanfeOptions;

/// Font family of the receipt.
const FONT_FAMILY: &str = "Arial";
/// Width of the dashes of the separators.
const DASH_WIDTH: f64 = 1.0;
/// Header of the issuing copy.
const VIA: &str = "Via Consumidor";

/// Everything the bands share.
struct Nfce {
    ctx: Arc<DanfeContext>,
    paper_width: f64,
    logo: Vec<u8>,
    qr_lateral: bool,
    mensagem_rodape: String,
}

type Ctx = Arc<Nfce>;

impl Nfce {
    fn nfe(&self) -> &Nfe {
        &self.ctx.nfe
    }

    fn simplificado_tipo2(&self) -> bool {
        self.nfe().ide.tp_imp == TpImp::SimplificadoTipo2 || self.ctx.options.simplificado_tipo2
    }

    /// `GetTextoBlocoCabecalho`.
    fn texto_cabecalho(&self) -> String {
        let emit = &self.nfe().emit;
        let e = &emit.ender_emit;
        let nro = if e.nro.is_empty() {
            String::new()
        } else {
            format!(", {}", e.nro)
        };
        let nome_fantasia = self.ctx.options.exibir_nome_fantasia && !emit.x_fant.is_empty();
        if self.simplificado_tipo2() {
            let nome = if nome_fantasia {
                &emit.x_fant
            } else {
                &emit.x_nome
            };
            format!(
                "CNPJ: {} {nome}\n{}{nro}, {}, {}, {}",
                emit.cnpj_cpf, e.x_lgr, e.x_bairro, e.x_mun, e.uf
            )
        } else {
            let mut result = String::new();
            if nome_fantasia {
                result.push_str(&format!("{}\n", emit.x_fant));
            }
            result.push_str(&format!(
                "{}\n{}{nro}, {}, {}, {}\nCNPJ: {}    IE: {}",
                emit.x_nome,
                e.x_lgr,
                e.x_bairro,
                e.x_mun,
                e.uf,
                format::cnpj_ou_cpf(&emit.cnpj_cpf),
                emit.ie
            ));
            result
        }
    }

    /// `GetTextoBlocoConsumidor`.
    fn texto_consumidor(&self) -> String {
        let dest = &self.nfe().dest;
        let mut result;
        if !dest.cnpj_cpf.is_empty() {
            result = match dest.cnpj_cpf.len() {
                14 => format!("CONSUMIDOR - CNPJ: {}", format::cnpj_ou_cpf(&dest.cnpj_cpf)),
                11 => format!("CONSUMIDOR - CPF: {}", format::cnpj_ou_cpf(&dest.cnpj_cpf)),
                _ => String::new(),
            };
            if self.ctx.options.exibir_consumidor_nome && !dest.x_nome.is_empty() {
                result.push_str(&format!("\n{}", dest.x_nome));
            }
        } else {
            result = "CONSUMIDOR NÃO IDENTIFICADO".to_owned();
        }
        let a = &dest.ender_dest;
        if self.ctx.options.exibir_consumidor_endereco && !a.x_lgr.is_empty() {
            result.push_str(&format!(
                "\n{}, {} {} {} {}-{}",
                a.x_lgr, a.nro, a.x_cpl, a.x_bairro, a.x_mun, a.uf
            ));
        }
        result
    }
}

/// Splits like the original `Split` (left-trimming every piece).
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split('\n')
        .map(|l| l.trim_start_matches(|c: char| c <= ' '))
        .collect()
}

/// An item with its wrapped description and the height it takes.
struct Item {
    height: f64,
    x_prod: String,
}

/// `GetNFCeItem`: the description (cut so it fits two lines) and its height.
fn nfce_item(prod_x_prod: &str, width: f64, pdf: &mut ReportPdf<'_>) -> Item {
    let font_height = pdf.current_font_size();
    let chars: Vec<char> = prod_x_prod.chars().collect();
    let mut limit = 45usize;
    let mut x_prod: String = chars.iter().take(limit).collect();
    let mut n = pdf.word_wrap(&mut x_prod, width, 0.0);
    while n > 2 && limit > 0 {
        limit -= 1;
        x_prod = chars.iter().take(limit).collect();
        let mut p = x_prod.clone();
        n = pdf.word_wrap(&mut p, width, 0.0);
    }
    Item {
        height: font_height * n as f64 + 0.5,
        x_prod,
    }
}

fn bloco_cabecalho(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    band.auto_height = true;
    let width = band.width;
    let final_pass = args.final_pass;
    let pdf = &mut args.pdf;

    let mut y = 0.0;
    let inc_y = 0.5;
    let (x_rs, w_rs, align_h);
    let logo_size = if c.logo.is_empty() {
        None
    } else {
        image_size(&c.logo)
    };
    if let Some((logo_w, logo_h)) = logo_size {
        let max_img_h = 14.0;
        let (x_img, y_img) = (0.0, 1.0);
        let logo_w_mm = f64::from(logo_w) / 72.0 * 25.4;
        let logo_h_mm = f64::from(logo_h) / 72.0 * 25.4;
        let mut n_img_w = round_to(width / 4.0, 0);
        let mut n_img_h = round_to(logo_h_mm * (n_img_w / logo_w_mm), 0);
        if n_img_h > max_img_h {
            n_img_h = max_img_h;
            n_img_w = round_to(logo_w_mm * (n_img_h / logo_h_mm), 0);
        }
        x_rs = n_img_w;
        w_rs = width - n_img_w;
        align_h = 'L';
        if final_pass {
            pdf.image(x_img, y_img, n_img_w, n_img_h, &c.logo, 'C', 'C', false);
        }
        // The space is reserved in the first pass as well, so the page is tall enough.
        y += 5.0;
    } else {
        x_rs = 0.0;
        w_rs = width;
        align_h = 'C';
    }

    let texto = c.texto_cabecalho();
    pdf.set_font(FONT_FAMILY, "B", 8.0);
    for (i, line) in split_lines(&texto).into_iter().enumerate() {
        y += pdf.text_box_force(
            x_rs + 2.0,
            y,
            w_rs - 2.0,
            0.0,
            line,
            'T',
            align_h,
            0.0,
            false,
        );
        if i == 0 {
            pdf.set_font(FONT_FAMILY, "", 7.0);
        }
    }

    y += 3.0;
    pdf.dashed_line(0.0, y, width, y, DASH_WIDTH);
    y += 2.0;

    let texto = if c.simplificado_tipo2() {
        "DANFE Simplificado Tipo 2"
    } else {
        "DANFE NFC-e Documento Auxiliar da\nNota Fiscal de Consumidor Eletronica"
    };
    pdf.set_font_size_style(8.0, "B");
    y += pdf.text_box_force(0.0, y, width, 0.0, texto, 'T', 'C', 0.0, false);
    y += inc_y;
    y += 2.0;
    pdf.dashed_line(0.0, y, width, y, DASH_WIDTH);
}

fn bloco_chave_acesso(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    band.auto_height = true;
    let width = band.width;
    let nfe = c.nfe();
    let pdf = &mut args.pdf;

    let mut y1 = 0.0;
    pdf.set_font_size_style(8.0, "B");
    y1 += pdf.text_box_force(
        0.0,
        y1,
        width,
        2.0,
        "Consulte pela Chave de Acesso em",
        'T',
        'C',
        0.0,
        false,
    );
    y1 += 0.5;

    pdf.set_font_size_style(7.0, "");
    y1 += pdf.text_box_force(
        0.0,
        y1,
        width,
        2.0,
        &nfe.inf_nfe_supl.url_chave,
        'T',
        'C',
        0.0,
        false,
    );
    y1 += 0.5;

    pdf.set_font_size_style(7.0, "");
    pdf.text_box_force(
        0.0,
        y1,
        width,
        2.0,
        &format::chave_acesso(&nfe.id),
        'T',
        'C',
        0.0,
        true,
    );
}

fn bloco_consumidor(c: &Ctx, width: f64, pdf: &mut ReportPdf<'_>, y: &mut f64) {
    let nfe = c.nfe();
    let mut x1 = 0.0;
    let mut y1 = *y;
    if c.qr_lateral {
        y1 += 1.0;
    }
    let mut w = width;
    let mut h_align = 'C';
    if c.qr_lateral {
        w *= 0.7;
        x1 += (width - w) + 2.0;
        h_align = 'L';
    }
    let sub_size = if c.paper_width < 70.0 { 1.5 } else { 0.0 };
    let texto = c.texto_consumidor();
    if nfe.ide.tp_emis == TpEmis::OffLine {
        pdf.set_font_size_style(7.0 - sub_size, "");
    } else {
        pdf.set_font_size_style(7.0, "");
    }
    *y += pdf.text_box_force(x1, y1, w, 0.0, &texto, 'T', h_align, 0.0, false);
    *y += 1.0;
}

/// Position and style shared by the lines of the identification block.
#[derive(Clone, Copy)]
struct Place {
    x: f64,
    w: f64,
    h_align: char,
    sub_size: f64,
}

/// Authorisation protocol lines; returns the height used.
fn bloco_protocolo(
    pdf: &mut ReportPdf<'_>,
    place: Place,
    y: f64,
    protocolo: &str,
    dh_recbto: &str,
) -> f64 {
    let Place {
        x,
        w,
        h_align,
        sub_size,
    } = place;
    let mut y1 = y;
    pdf.set_font_size_style(8.0 - sub_size, "");
    let text = format!("Protocolo de Autorização: {protocolo}");
    y1 += pdf.text_box_force(x, y1, w, 4.0, &text, 'T', h_align, 0.0, true);
    y1 += 0.5;
    pdf.set_font_size_style(8.0 - sub_size, "");
    let text = format!("Data de Autorização: {dh_recbto}");
    y1 += pdf.text_box_force(x, y1, w, 4.0, &text, 'T', h_align, 0.0, true);
    y1 += 0.5;
    y1 - y
}

fn bloco_identificacao(c: &Ctx, width: f64, pdf: &mut ReportPdf<'_>, y: &mut f64) {
    let nfe = c.nfe();
    let mut x1 = 0.0;
    let mut y1 = *y;
    let mut w = width;
    let mut h_align = 'C';
    if c.qr_lateral {
        w = w * 0.7 - 2.0;
        x1 += width - w;
        h_align = 'L';
    }
    let sub_size = if c.paper_width < 70.0 || c.qr_lateral {
        1.0
    } else {
        0.0
    };
    let emissao = nfe.ide.d_emi.map(|d| d.datetime_br()).unwrap_or_default();
    let numero = format::zero_pad(nfe.ide.n_nf, 9);
    let serie = format::zero_pad(u64::from(nfe.ide.serie), 3);
    let mut texto = if c.simplificado_tipo2() {
        format!("NFe n. {numero} Série {serie}")
    } else {
        format!("NFCe n. {numero} Série {serie}")
    };
    if !c.qr_lateral {
        texto = format!("{texto} {emissao}");
    }

    pdf.set_font_size_style(8.0 - sub_size, "B");
    y1 += pdf.text_box_flags(x1, y1, w, 4.0, &texto, 'T', h_align, false, true, false);
    if c.qr_lateral {
        y1 += pdf.text_box_flags(
            x1,
            y1,
            w,
            4.0,
            &format!("Emissão: {emissao}"),
            'T',
            h_align,
            false,
            true,
            false,
        );
    }

    if nfe.ide.tp_amb == TpAmb::Homologacao {
        y1 += 2.0;
        pdf.set_font_size_style(10.0 - sub_size, "B");
        let texto = "EMITIDA EM AMBIENTE DE HOMOLOGAÇÃO - SEM VALOR FISCAL";
        let h = pdf.get_string_height(texto, w, 0.0, 0.0);
        y1 += pdf.text_box_flags(x1, y1, w, h, texto, 'T', h_align, false, true, false);
        y1 += 1.0;
    }

    let dh_recbto = nfe
        .proc_nfe
        .dh_recbto
        .map(|d| d.datetime_br())
        .unwrap_or_default();
    if nfe.ide.tp_emis == TpEmis::OffLine {
        y1 += pdf.text_box_force(x1, y1, w, 4.0, VIA, 'T', h_align, 0.0, true);

        y1 += 2.0;
        pdf.set_font_size_style(10.0 - sub_size, "B");
        y1 += pdf.text_box_force(
            x1,
            y1,
            w,
            4.0,
            "EMITIDA EM CONTINGÊNCIA",
            'B',
            h_align,
            0.0,
            true,
        );

        if nfe.proc_nfe.n_prot.is_empty() {
            pdf.set_font_size_style(8.0 - sub_size, "I");
            pdf.text_box_force(
                x1,
                y1,
                w,
                3.0,
                "Pendente de autorização",
                'B',
                h_align,
                0.0,
                true,
            );
        } else {
            y1 += bloco_protocolo(
                pdf,
                Place {
                    x: x1,
                    w,
                    h_align,
                    sub_size,
                },
                y1,
                &nfe.proc_nfe.n_prot,
                &dh_recbto,
            );
        }
    } else if nfe.ide.tp_emis == TpEmis::Dpec {
        pdf.set_font_size_style(10.0 - sub_size, "B");
        let texto = "DANFE-NFC-e Impresso em contingência - EPEC";
        let h = pdf.get_string_height(texto, w, 0.0, 0.0);
        y1 += pdf.text_box_flags(x1, y1, w, h, texto, 'T', h_align, true, true, false);

        pdf.set_font_size_style(8.0 - sub_size, "");
        let texto = "Regularmente recebido pela administração tributária autorizadora";
        let h = pdf.get_string_height(texto, w, 0.0, 0.0);
        y1 += pdf.text_box_flags(x1, y1, w, h, texto, 'T', h_align, true, true, false);

        if let Some(dh_cont) = nfe.ide.dh_cont {
            pdf.set_font_size_style(7.0 - sub_size, "");
            let texto = format!("Data de entrada em contingência: {}", dh_cont.datetime_br());
            let h = pdf.get_string_height(&texto, w, 0.0, 0.0);
            pdf.text_box_flags(x1, y1, w, h, &texto, 'B', h_align, false, true, false);
        }
    } else {
        let h = pdf.get_string_height(VIA, w, 0.0, 0.0);
        y1 += pdf.text_box_force(x1, y1, w, h, VIA, 'T', h_align, 0.0, true);

        if nfe.proc_nfe.n_prot.is_empty() {
            let texto = "NFCe sem Autorização de Uso da SEFAZ";
            pdf.set_font_size_style(8.0, "I");
            let h = pdf.get_string_height(texto, w, 0.0, 0.0);
            y1 += pdf.text_box_force(x1, y1, w, h, texto, 'B', h_align, 0.0, true);
        } else {
            y1 += bloco_protocolo(
                pdf,
                Place {
                    x: x1,
                    w,
                    h_align,
                    sub_size,
                },
                y1,
                &nfe.proc_nfe.n_prot,
                &dh_recbto,
            );
        }
    }
    *y = y1;
}

fn bloco_qrcode(c: &Ctx, width: f64, pdf: &mut ReportPdf<'_>, y: f64) {
    let w = width;
    let y1 = y + 2.0;
    let qr_size = if c.qr_lateral {
        width * 0.3
    } else {
        width * 0.7
    };
    let qr_y = y1;
    let qr_x = if c.qr_lateral {
        0.0
    } else {
        w / 2.0 - qr_size / 2.0
    };
    // Without `infNFeSupl` (an offline NFC-e saved before its QR Code was built) the original
    // divides by zero (an exception under Free Pascal) and draws nothing sensible: no QR Code
    // here, and the space stays.
    if c.nfe().inf_nfe_supl.qr_code.trim().is_empty() {
        return;
    }
    pdf.set_fill_color(Color::Rgb(0, 0, 0));
    pdf.qr_code_sized(
        qr_x,
        qr_y,
        qr_size,
        &c.nfe().inf_nfe_supl.qr_code,
        QrEcc::Low,
    );
}

fn bloco_qrcode_centralizado_ou_lateral(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    let mut y = 1.5;
    band.auto_height = true;
    let width = band.width;
    let pdf = &mut args.pdf;

    let qr_y = y;
    bloco_consumidor(c, width, pdf, &mut y);
    bloco_identificacao(c, width, pdf, &mut y);
    if c.qr_lateral {
        y = qr_y;
    }
    bloco_qrcode(c, width, pdf, y);
}

fn bloco_mensagem_fiscal(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    let width = band.width;
    let nfe = c.nfe();
    let pdf = &mut args.pdf;
    let y = 0.0;
    let altura = 9.5;
    let box_text = |pdf: &mut ReportPdf<'_>, texto: &str| -> (f64, f64) {
        let texto_w = pdf.get_string_width(texto) + 2.0;
        let margem = (width - texto_w) / 2.0;
        pdf.set_fill_color(Color::Rgb(224, 224, 224));
        pdf.rect(0.0, y, width, altura, "F");
        pdf.set_fill_color(NamedColor::White);
        pdf.rect(margem, y + 1.0, texto_w, altura - 2.0, "F");
        (texto_w, margem)
    };

    if nfe.ide.tp_amb == TpAmb::Homologacao {
        pdf.set_font_size_style(8.0, "I");
        box_text(pdf, "EMITIDA EM AMBIENTE DE HOMOLOGAÇÃO");
        let mut y1 = y + 1.0;
        pdf.set_font_size_style(10.0, "B");
        y1 += pdf.text_box_force(0.0, y1, width, 4.0, "SEM VALOR FISCAL", 'T', 'C', 0.0, true);
        pdf.set_font_size_style(8.0, "I");
        y1 += pdf.text_box_force(
            0.0,
            y1,
            width,
            3.0,
            "EMITIDA EM AMBIENTE DE HOMOLOGAÇÃO",
            'T',
            'C',
            0.0,
            true,
        );
        y1 += 2.0;
        pdf.dashed_line(0.0, y1, width, y1, DASH_WIDTH);
    } else if nfe.ide.tp_emis == TpEmis::OffLine {
        pdf.set_font_size_style(10.0, "B");
        let texto = "EMITIDA EM CONTINGÊNCIA";
        box_text(pdf, texto);
        let mut y1 = y + 1.0;
        y1 += pdf.text_box_force(0.0, y1, width, 4.0, texto, 'T', 'C', 0.0, true);
        pdf.set_font_size_style(8.0, "I");
        y1 += pdf.text_box_force(
            0.0,
            y1,
            width,
            3.0,
            "Pendente de autorização",
            'T',
            'C',
            0.0,
            true,
        );
        y1 += 2.0;
        pdf.dashed_line(0.0, y1, width, y1, DASH_WIDTH);
    }
}

fn bloco_itens(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    band.auto_height = true;
    let width = band.width;
    let nfe = c.nfe();
    let pdf = &mut args.pdf;

    let matrix = [0.14, 0.36, 0.09, 0.09, 0.156, 0.156];
    let fsize = if c.paper_width < 70.0 { 5.0 } else { 7.0 };
    pdf.set_font_size_style(fsize, "B");

    let mut y = 1.0;
    let x = 0.0;
    pdf.text_box_force(x, y, width * matrix[0], 3.0, "CÓDIGO", 'T', 'L', 0.0, true);
    let x1 = x + width * matrix[0];
    pdf.text_box_force(
        x1,
        y,
        width * matrix[1],
        3.0,
        "DESCRIÇÃO",
        'T',
        'L',
        0.0,
        true,
    );
    let x2 = x1 + width * matrix[1];
    pdf.text_box_force(x2, y, width * matrix[2], 3.0, "QTDE", 'T', 'C', 0.0, true);
    let x3 = x2 + width * matrix[2];
    pdf.text_box_force(x3, y, width * matrix[3], 3.0, "UN", 'T', 'C', 0.0, true);
    let x4 = x3 + width * matrix[3];
    pdf.text_box_force(
        x4,
        y,
        width * matrix[4],
        3.0,
        "VL UNIT",
        'T',
        'C',
        0.0,
        true,
    );
    let x5 = x4 + width * matrix[4];
    let y1 = pdf.text_box_force(
        x5,
        y,
        width * matrix[5],
        3.0,
        "VL TOTAL",
        'T',
        'R',
        0.0,
        true,
    );

    y += 0.5;
    pdf.set_font_size_style(fsize, "");
    let mut y2 = y + y1;
    for det in &nfe.det {
        let prod = &det.prod;
        let item = nfce_item(&prod.x_prod, width * matrix[1], pdf);
        pdf.text_box_force(
            x,
            y2,
            width * matrix[0],
            item.height,
            &prod.c_prod,
            'T',
            'L',
            0.0,
            false,
        );
        pdf.text_box_force(
            x1,
            y2,
            width * matrix[1],
            item.height,
            &item.x_prod,
            'T',
            'L',
            0.0,
            false,
        );
        pdf.text_box_force(
            x2,
            y2,
            width * matrix[2],
            item.height,
            &prod.q_com.money(),
            'T',
            'R',
            0.0,
            false,
        );
        let unit: String = prod.u_com.chars().take(2).collect();
        pdf.text_box_force(
            x3,
            y2,
            width * matrix[3],
            item.height,
            &unit,
            'T',
            'C',
            0.0,
            false,
        );
        pdf.text_box_force(
            x4,
            y2,
            width * matrix[4],
            item.height,
            &prod.v_un_com.money(),
            'T',
            'R',
            0.0,
            false,
        );
        pdf.text_box_force(
            x5,
            y2,
            width * matrix[5],
            item.height,
            &prod.v_prod.money(),
            'T',
            'R',
            0.0,
            false,
        );
        y2 += item.height;
    }
    y = y2 + 1.0;
    pdf.dashed_line(0.0, y, width, y, DASH_WIDTH);
}

fn bloco_totais(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    band.auto_height = true;
    let width = band.width;
    let band_height = band.height;
    let nfe = c.nfe();
    let t = &nfe.total.icms_tot;
    let pdf = &mut args.pdf;

    let mut y = 0.0;
    let valor = t.v_nf;
    let desconto = t.v_desc;
    let frete = t.v_frete;
    let bruto = valor + desconto - frete;
    let half = width / 2.0;

    pdf.set_font_size_style(8.0, "");
    pdf.text_box_force(
        0.0,
        y,
        half,
        3.0,
        "Qtd. Total de Itens",
        'T',
        'L',
        0.0,
        false,
    );
    let y1 = pdf.text_box_force(
        half,
        y,
        half,
        3.0,
        &Decimal::from_i64(nfe.det.len() as i64).format_br(0, true),
        'T',
        'R',
        0.0,
        false,
    );

    pdf.text_box_force(
        0.0,
        y + y1,
        half,
        3.0,
        "Valor Total R$",
        'T',
        'L',
        0.0,
        false,
    );
    let y2 = pdf.text_box_force(
        half,
        y + y1,
        half,
        3.0,
        &bruto.money(),
        'T',
        'R',
        0.0,
        false,
    );

    pdf.text_box_force(
        0.0,
        y + y1 + y2,
        half,
        3.0,
        "Desconto R$",
        'T',
        'L',
        0.0,
        false,
    );
    let y3 = pdf.text_box_force(
        half,
        y + y1 + y2,
        half,
        3.0,
        &desconto.money(),
        'T',
        'R',
        0.0,
        false,
    );

    pdf.text_box_force(
        0.0,
        y + y1 + y2 + y3,
        half,
        3.0,
        "Frete R$",
        'T',
        'L',
        0.0,
        false,
    );
    let texto = frete.money();
    let y4 = pdf.text_box_force(
        half,
        y + y1 + y2 + y3,
        half,
        3.0,
        &texto,
        'T',
        'R',
        0.0,
        false,
    );
    y += pdf.get_string_height(&texto, width, 0.0, 0.0);

    if !c.simplificado_tipo2() {
        let fsize = if c.paper_width < 70.0 { 8.0 } else { 10.0 };
        pdf.set_font_size_style(fsize, "B");
        pdf.text_box_force(
            0.0,
            y + y1 + y2 + y3 + y4,
            half,
            3.0,
            "Valor a Pagar R$",
            'T',
            'L',
            0.0,
            false,
        );
        pdf.text_box_force(
            half,
            y + y1 + y2 + y3 + y4,
            half,
            3.0,
            &valor.money(),
            'T',
            'R',
            0.0,
            false,
        );
    }

    pdf.dashed_line(0.0, band_height + y, width, band_height + y, DASH_WIDTH);
}

fn bloco_informacoes_ibs_cbs(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    band.auto_height = true;
    let width = band.width;
    let t = &c.nfe().total.ibs_cbs_tot;
    let pdf = &mut args.pdf;

    let mut y = 0.0;
    pdf.set_font_size_style(8.0, "");
    let line = |pdf: &mut ReportPdf<'_>, label: &str, value: Decimal, y: &mut f64| {
        let h = pdf.get_string_height(label, width, 0.0, 0.0);
        pdf.text_box_force(3.0, *y, width, h, label, 'T', 'L', 0.0, false);
        let text = value.money();
        let h = pdf.get_string_height(&text, width, 0.0, 0.0);
        pdf.text_box_force(0.0, *y, width, h, &text, 'T', 'R', 0.0, false);
        *y += h;
    };
    line(pdf, "(+) CBS R$", t.v_cbs, &mut y);
    line(pdf, "(+) IBS R$", t.v_ibs, &mut y);
    if t.v_is.is_positive() {
        line(pdf, "(+) IS R$", t.v_is, &mut y);
    }
    pdf.dashed_line(0.0, y, width, y, DASH_WIDTH);
}

fn bloco_pagamentos(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    band.auto_height = true;
    let width = band.width;
    let nfe = c.nfe();
    let pdf = &mut args.pdf;

    let mut y = 0.0;
    if c.simplificado_tipo2() {
        let fsize = if c.paper_width < 70.0 { 8.0 } else { 10.0 };
        pdf.set_font_size_style(fsize, "B");
        pdf.text_box_force(
            0.0,
            y,
            width / 2.0,
            3.0,
            "Valor a Pagar R$",
            'T',
            'L',
            0.0,
            false,
        );
        let texto = nfe.total.icms_tot.v_nf.money();
        pdf.text_box_force(
            width / 2.0,
            y,
            width / 2.0,
            3.0,
            &texto,
            'T',
            'R',
            0.0,
            false,
        );
        y += pdf.get_string_height(&texto, width, 0.0, 0.0);
    }
    pdf.set_font_size_style(7.0, "");
    pdf.text_box_force(0.0, y, width, 4.0, "FORMA PAGAMENTO", 'T', 'L', 0.0, false);
    let y1 = pdf.text_box_force(0.0, y, width, 4.0, "VALOR PAGO R$", 'T', 'R', 0.0, false);

    let mut z = y + y1;
    for pag in &nfe.pag {
        pdf.text_box_force(
            0.0,
            z,
            width,
            3.0,
            &conversions::forma_pagamento(&pag.t_pag, &pag.x_pag),
            'T',
            'L',
            0.0,
            false,
        );
        let y2 = pdf.text_box_force(0.0, z, width, 3.0, &pag.v_pag.money(), 'T', 'R', 0.0, false);
        z += y2;
    }
    pdf.text_box_force(0.0, z, width, 3.0, "Troco R$", 'T', 'L', 0.0, false);
    pdf.text_box_force(
        0.0,
        z,
        width,
        3.0,
        &nfe.pag_v_troco.money(),
        'T',
        'R',
        0.0,
        false,
    );

    y = z + 3.0;
    pdf.dashed_line(0.0, y, width, y, DASH_WIDTH);
}

fn bloco_linha(band: &mut BandState, args: &mut DrawArgs<'_>) {
    args.pdf
        .dashed_line(0.0, band.height, band.width, band.height, DASH_WIDTH);
}

fn bloco_mensagem_contribuinte(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    band.auto_height = true;
    let width = band.width;
    let nfe = c.nfe();
    let pdf = &mut args.pdf;

    let y = 0.0;
    pdf.set_font_size_style(7.0, "");
    let v_tot_trib = nfe.total.icms_tot.v_tot_trib;
    let trib = if v_tot_trib.is_positive() {
        v_tot_trib.money()
    } else {
        "-----".to_owned()
    };
    let texto = format!("Tributos Totais Incidentes (Lei Federal 12.741/2012): R$ {trib}");
    pdf.set_font_size_style(7.0, "");
    let h = pdf.get_string_height(&texto, width, 0.0, 0.0);
    pdf.text_box_force(0.0, y, width, h, &texto, 'T', 'L', 0.0, true);
    if c.paper_width < 70.0 {
        pdf.set_font_size_style(5.0, "");
    }
    let texto = nfe
        .inf_adic
        .inf_cpl
        .replace(&c.ctx.options.caractere_quebra_de_linha, "\n");
    let h = pdf.get_string_height(&texto, width, 0.0, 0.0);
    pdf.text_box_force(0.0, y + 3.0, width, h, &texto, 'T', 'L', 0.0, false);
}

fn bloco_rodape(c: &Ctx, band: &mut BandState, args: &mut DrawArgs<'_>) {
    band.auto_height = true;
    let width = band.width;
    let y = 2.0;
    if c.mensagem_rodape.is_empty() {
        return;
    }
    let parts: Vec<&str> = c.mensagem_rodape.split('|').collect();
    let pdf = &mut args.pdf;
    pdf.set_font_size_style(if c.paper_width < 70.0 { 4.0 } else { 6.0 }, "I");
    if let Some(text) = parts.first() {
        pdf.text_box_force(0.0, y, width, 0.0, text, 'T', 'L', 0.0, true);
    }
    if let Some(text) = parts.get(1) {
        pdf.text_box_force(0.0, y, width, 0.0, text, 'T', 'C', 0.0, true);
    }
    if let Some(text) = parts.get(2) {
        pdf.text_box_force(0.0, y, width, 0.0, text, 'T', 'R', 0.0, true);
    }
}

/// Adds the receipt page and bands to `report` (`OnStartReport` of the original).
fn build_report(report: &mut Report, c: &Ctx) {
    let page =
        report.add_page_with_dimensions(Orientation::Landscape, Unit::Mm, c.paper_width, 20.0);
    page.endless_height = true;

    fn add(
        report: &mut Report,
        height: f64,
        c: &Ctx,
        draw: fn(&Ctx, &mut BandState, &mut DrawArgs<'_>),
    ) {
        let c = Arc::clone(c);
        // The report has exactly one page at this point.
        let _ = report.add_anonymous_band(BandType::Data, height, move |band, args| {
            draw(&c, band, args)
        });
    }

    if c.simplificado_tipo2() {
        add(report, 10.0, c, bloco_cabecalho);
        add(report, 10.0, c, bloco_mensagem_fiscal);
        add(report, 10.0, c, bloco_itens);
        add(report, 10.0, c, bloco_totais);
        add(report, 10.0, c, bloco_informacoes_ibs_cbs);
        add(report, 16.0, c, bloco_pagamentos);
        add(report, 10.0, c, bloco_chave_acesso);
        add(report, 15.0, c, bloco_qrcode_centralizado_ou_lateral);
        let _ = report.add_anonymous_band(BandType::Data, 2.0, bloco_linha);
        add(report, 15.0, c, bloco_mensagem_contribuinte);
    } else {
        add(report, 10.0, c, bloco_cabecalho);
        add(report, 10.0, c, bloco_mensagem_fiscal);
        if c.ctx.options.exibir_itens {
            add(report, 10.0, c, bloco_itens);
        }
        add(report, 16.0, c, bloco_totais);
        add(report, 10.0, c, bloco_pagamentos);
        add(report, 10.0, c, bloco_chave_acesso);
        add(report, 10.0, c, bloco_qrcode_centralizado_ou_lateral);
        let _ = report.add_anonymous_band(BandType::Data, 2.0, bloco_linha);
        add(report, 10.0, c, bloco_mensagem_contribuinte);
    }
    if !c.mensagem_rodape.is_empty() {
        add(report, 5.0, c, bloco_rodape);
    }
}

/// Renders the DANFE NFC-e of an already parsed NFC-e.
pub fn render_nfce(nfe: Nfe, options: &DanfeOptions) -> Result<Vec<u8>, DanfeError> {
    let paper_width = if options.largura_bobina == 302 {
        80.0
    } else {
        f64::from(options.largura_bobina)
    };
    let compress = options.comprimir_pdf;
    let c = Arc::new(Nfce {
        paper_width,
        logo: options.logo.clone().unwrap_or_default(),
        qr_lateral: options.imprime_qrcode_lateral,
        mensagem_rodape: options.sistema.clone(),
        ctx: Arc::new(DanfeContext::new(nfe, options.clone())),
    });

    let mut report = Report::new();
    report.set_font(FONT_FAMILY);
    report.set_margins(2.0, 2.0, None, None);
    report.engine_options.double_pass = true;
    let mut initialised = false;
    report.set_on_start(move |report, _| {
        if !initialised {
            build_report(report, &c);
            initialised = true;
        }
    });

    let mut engine = Engine::new(&mut report);
    engine.set_compressed(compress);
    Ok(engine.render()?)
}
