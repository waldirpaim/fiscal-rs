//! Tax totals, carrier/transport and services totals bands.

use std::sync::Arc;

use fiscal_pdf::report::{BandHandler, BandState, DrawArgs, InitArgs, ReportPdf};
use fiscal_pdf::round_to;

use crate::context::DanfeContext;
use crate::conversions;
use crate::decimal::Decimal;
use crate::format;

/// "CÁLCULO DO IMPOSTO".
pub(super) struct CalculoImposto {
    pub(super) ctx: Arc<DanfeContext>,
}

impl BandHandler for CalculoImposto {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.height = 18.0;
    }

    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let width = band.width;
        let t = &self.ctx.nfe.total.icms_tot;
        let pdf = &mut args.pdf;

        let imposto = |pdf: &mut ReportPdf<'_>,
                       x: f64,
                       y: f64,
                       w: f64,
                       h: f64,
                       campo: &str,
                       valor: Decimal,
                       style: &str| {
            pdf.set_font_size_style(6.0, "");
            pdf.text_box(x, y, w - 1.0, h, campo, 'T', 'L', 0.0);
            pdf.set_font_size_style(10.0, style);
            pdf.text_box(x, y, w, h, &valor.money(), 'B', 'R', 1.0);
            x + w
        };

        let mut x = 0.0;
        let mut y = 1.0;
        let x_inicial = x;
        let campos_por_linha = 9.0;
        let title_size = 31.0;
        let w = width / campos_por_linha;

        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, title_size, 8.0, "CÁLCULO DO IMPOSTO", 'T', 'L', 0.0);
        y += 3.0;
        let h = 7.0;

        x = imposto(pdf, x, y, w, h, "BASE DE CÁLC. DO ICMS", t.v_bc, "");
        x = imposto(pdf, x, y, w, h, "VALOR DO ICMS", t.v_icms, "");
        x = imposto(pdf, x, y, w, h, "BASE DE CÁLC. ICMS S.T.", t.v_bc_st, "");
        x = imposto(pdf, x, y, w, h, "VALOR DO ICMS SUBST.", t.v_st, "");
        x = imposto(pdf, x, y, w, h, "V. IMP. IMPORTAÇÃO", t.v_ii, "");
        x = imposto(pdf, x, y, w, h, "V. ICMS UF REMET.", t.v_icms_uf_remet, "");
        x = imposto(pdf, x, y, w, h, "VALOR DO FCP", t.v_fcp_uf_dest, "");
        x = imposto(pdf, x, y, w, h, "VALOR DO PIS", t.v_pis, "");
        imposto(pdf, x, y, w, h, "V. TOTAL PRODUTOS", t.v_prod, "");

        y += h;
        x = x_inicial;

        x = imposto(pdf, x, y, w, h, "VALOR DO FRETE", t.v_frete, "");
        x = imposto(pdf, x, y, w, h, "VALOR DO SEGURO", t.v_seg, "");
        x = imposto(pdf, x, y, w, h, "DESCONTO", t.v_desc, "");
        x = imposto(pdf, x, y, w, h, "OUTRAS DESPESAS", t.v_outro, "");
        x = imposto(pdf, x, y, w, h, "VALOR TOTAL IPI", t.v_ipi, "");
        x = imposto(pdf, x, y, w, h, "V. ICMS UF DEST.", t.v_icms_uf_dest, "");
        x = imposto(pdf, x, y, w, h, "V. TOT. TRIB.", t.v_tot_trib, "");
        x = imposto(pdf, x, y, w, h, "VALOR DA COFINS", t.v_cofins, "");
        imposto(pdf, x, y, w, h, "V. TOTAL DA NOTA", t.v_nf, "B");
    }
}

/// "TRANSPORTADOR / VOLUMES TRANSPORTADOS".
pub(super) struct Transporte {
    pub(super) ctx: Arc<DanfeContext>,
}

impl BandHandler for Transporte {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.height = 25.0;
    }

    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let max_w = band.width;
        let nfe = &self.ctx.nfe;
        let t = &nfe.transp;
        let pdf = &mut args.pdf;

        let mut x = 0.0;
        let mut y = 1.0;
        let old_x = x;

        // TRANSPORTADOR / VOLUMES TRANSPORTADOS
        let mut w = max_w;
        let mut h = 7.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(
            x,
            y,
            w,
            h,
            "TRANSPORTADOR / VOLUMES TRANSPORTADOS",
            'T',
            'L',
            0.0,
        );
        // NOME / RAZÃO SOCIAL
        let mut w1 = max_w * 0.29;
        y += 3.0;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w1, h, "NOME / RAZÃO SOCIAL", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w1, h, &t.transporta.x_nome, 'B', 'L', 0.0);
        // FRETE POR CONTA
        x += w1;
        let mut w2 = max_w * 0.15;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w2, h, "FRETE POR CONTA", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(
            x,
            y,
            w2,
            h,
            conversions::mod_frete(t.mod_frete, nfe.versao),
            'B',
            'C',
            1.0,
        );
        // CÓDIGO ANTT
        x += w2;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w2, h, "CÓDIGO ANTT", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w2, h, &t.veic_transp.rntc, 'B', 'C', 0.0);
        // PLACA DO VEÍCULO
        x += w2;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w2, h, "PLACA DO VEÍCULO", 'T', 'L', 1.0);
        let mut texto = t.veic_transp.placa.clone();
        if texto.is_empty() {
            if let Some(r) = t.reboque.first() {
                texto = r.placa.clone();
            }
        }
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w2, h, &texto, 'B', 'C', 0.0);
        // UF
        x += w2;
        let mut w3 = round_to(max_w * 0.04, 0);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w3, h, "UF", 'T', 'L', 1.0);
        let mut texto = t.veic_transp.uf.clone();
        if texto.is_empty() {
            if let Some(r) = t.reboque.first() {
                texto = r.uf.clone();
            }
        }
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w3, h, &texto, 'B', 'C', 0.0);
        // CNPJ / CPF
        x += w3;
        w = max_w - (w1 + 3.0 * w2 + w3);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "CNPJ / CPF", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(
            x,
            y,
            w,
            h,
            &format::cnpj_ou_cpf(&t.transporta.cnpj_cpf),
            'B',
            'L',
            0.0,
        );

        // ENDEREÇO
        y += h;
        x = old_x;
        h = 7.0;
        w1 = max_w * 0.44;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w1, h, "ENDEREÇO", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w1, h, &t.transporta.x_ender, 'B', 'L', 0.0);
        // MUNICÍPIO
        x += w1;
        w2 = round_to(max_w * 0.30, 0);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w2, h, "MUNICÍPIO", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w2, h, &t.transporta.x_mun, 'B', 'L', 0.0);
        // UF
        x += w2;
        w3 = round_to(max_w * 0.04, 0);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w3, h, "UF", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w3, h, &t.transporta.uf, 'B', 'C', 0.0);
        // INSCRIÇÃO ESTADUAL
        x += w3;
        w = max_w - (w1 + w2 + w3);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "INSCRIÇÃO ESTADUAL", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &t.transporta.ie, 'B', 'C', 0.0);

        // Several volumes are summed; differing species/brand/numbering become "VARIAS".
        let mut quantidade: i64 = 0;
        let mut especie = String::new();
        let mut marca = String::new();
        let mut numero = String::new();
        let mut peso_bruto = Decimal::ZERO;
        let mut peso_liquido = Decimal::ZERO;
        for vol in &t.vol {
            quantidade += vol.q_vol;
            peso_bruto = peso_bruto + vol.peso_b;
            peso_liquido = peso_liquido + vol.peso_l;
            especie = if vol.esp != especie && !especie.is_empty() {
                "VARIAS".to_owned()
            } else {
                vol.esp.clone()
            };
            marca = if vol.marca != marca && !marca.is_empty() {
                "VARIAS".to_owned()
            } else {
                vol.marca.clone()
            };
            numero = if vol.n_vol != numero && !numero.is_empty() {
                "VARIAS".to_owned()
            } else {
                vol.n_vol.clone()
            };
        }

        // QUANTIDADE
        y += h;
        x = old_x;
        h = 7.0;
        w1 = round_to(max_w * 0.10, 0);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w1, h, "QUANTIDADE", 'T', 'L', 1.0);
        if quantidade > 0 {
            pdf.set_font_size_style(10.0, "");
            pdf.text_box(
                x,
                y,
                w1,
                h,
                &Decimal::from_i64(quantidade).format_br(0, true),
                'B',
                'C',
                0.0,
            );
        }
        // ESPÉCIE
        x += w1;
        w2 = round_to(max_w * 0.17, 0);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w2, h, "ESPÉCIE", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w2, h, &especie, 'B', 'C', 0.0);
        // MARCA
        x += w2;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w2, h, "MARCA", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w2, h, &marca, 'B', 'C', 0.0);
        // NUMERAÇÃO
        x += w2;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w2, h, "NUMERAÇÃO", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w2, h, &numero, 'B', 'C', 0.0);
        // PESO BRUTO
        x += w2;
        w3 = round_to(max_w * 0.20, 0);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w3, h, "PESO BRUTO (Kg)", 'T', 'L', 1.0);
        let texto = if peso_bruto.is_positive() {
            peso_bruto.format_br(3, true)
        } else {
            String::new()
        };
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w3, h, &texto, 'B', 'R', 0.0);
        // PESO LÍQUIDO
        x += w3;
        w = max_w - (w1 + 3.0 * w2 + w3);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "PESO LÍQUIDO (Kg)", 'T', 'L', 1.0);
        let texto = if peso_liquido.is_positive() {
            peso_liquido.format_br(3, true)
        } else {
            String::new()
        };
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &texto, 'B', 'R', 0.0);
    }
}

/// "CÁLCULO DO ISSQN".
pub(super) struct CalculoIssqn {
    pub(super) ctx: Arc<DanfeContext>,
}

impl BandHandler for CalculoIssqn {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        let t = &self.ctx.nfe.total.issqn_tot;
        band.visible = t.v_serv.is_positive();
        if t.v_bc.is_zero() {
            band.visible = false;
        }
        band.height = 11.0;
    }

    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let width = band.width;
        let nfe = &self.ctx.nfe;
        let t = &nfe.total.issqn_tot;
        let pdf = &mut args.pdf;

        let mut x = 0.0;
        let mut y = 1.0;
        let mut w = width;
        let h = 7.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, w, h, "CÁLCULO DO ISSQN", 'T', 'L', 0.0);
        // INSCRIÇÃO MUNICIPAL
        y += 3.0;
        w = round_to(width * 0.23, 0);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "INSCRIÇÃO MUNICIPAL", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &nfe.emit.im, 'B', 'L', 0.0);
        // VALOR TOTAL DOS SERVIÇOS
        x += w;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "VALOR TOTAL DOS SERVIÇOS", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &t.v_serv.money(), 'B', 'R', 0.0);
        // BASE DE CÁLCULO DO ISSQN
        x += w;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "BASE DE CÁLCULO DO ISSQN", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &t.v_bc.money(), 'B', 'R', 0.0);
        // VALOR TOTAL DO ISSQN
        x += w;
        w = width - 3.0 * w;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "VALOR TOTAL DO ISSQN", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &t.v_iss.money(), 'B', 'R', 0.0);
    }
}
