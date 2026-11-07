//! "DADOS DOS PRODUTOS / SERVIÇOS": the items table that flows across pages.

use std::sync::Arc;

use fiscal_pdf::report::{BandHandler, BandState, DrawArgs, InitArgs, ReportPdf};
use fiscal_pdf::round_to;

use crate::context::DanfeContext;

pub(super) struct ProdutosServicos {
    pub(super) ctx: Arc<DanfeContext>,
    h_cabec_itens: f64,
    current_item: usize,
}

impl ProdutosServicos {
    pub(super) fn new(ctx: Arc<DanfeContext>) -> Self {
        Self {
            ctx,
            h_cabec_itens: 4.0,
            current_item: 0,
        }
    }

    /// Height one item takes: the lines of its description times the font size.
    fn calculate_height_produto(&self, pdf: &mut ReportPdf<'_>, width: f64, item: usize) -> f64 {
        let w2 = round_to(width * 0.28, 0);
        pdf.set_font_size_style(7.0, "");
        let texto_produto = self.ctx.manter_xprod(item);
        let numlinhas = pdf.get_num_lines(&texto_produto, w2, 0.0) as f64;
        round_to(numlinhas * pdf.current_font_size() + numlinhas * 0.5, -2)
    }
}

impl BandHandler for ProdutosServicos {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        self.current_item = 0;
        band.auto_height = true;
        // Minimum height.
        band.height = 12.0;
    }

    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let ctx = Arc::clone(&self.ctx);
        let nfe = &ctx.nfe;
        let width = band.width;
        let free = if band.height <= args.free_space - args.reserved_space {
            args.free_space - args.reserved_space
        } else {
            args.free_space
        };
        if free <= 0.0 {
            return;
        }
        band.height = free;
        let pdf = &mut args.pdf;

        let mut x = 0.0;
        let mut y = 1.0;
        let old_x = x;
        let old_y = y;

        // DADOS DOS PRODUTOS / SERVIÇOS
        let w = width;
        let mut h = 4.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, w, h, "DADOS DOS PRODUTOS / SERVIÇOS", 'T', 'L', 0.0);
        y += 3.0;
        // The box of the items.
        let h_max = free - y;
        pdf.text_box_frame(x, y, w, h_max);

        // ---- header -------------------------------------------------------------------
        let cst_label = if nfe.emit.crt == crate::model::Crt::RegimeNormal {
            "O/CST"
        } else {
            "O/CSOSN"
        };
        h = 4.0;
        let columns: [(&str, f64); 13] = [
            ("CÓDIGO\nPRODUTO", 0.09),
            ("DESCRIÇÃO DO PRODUTO / SERVIÇO", 0.28),
            ("NCM/SH", 0.06),
            (cst_label, 0.05),
            ("CFOP", 0.04),
            ("UN", 0.03),
            ("QUANT", 0.07),
            ("VALOR UNIT", 0.06),
            ("VALOR TOTAL", 0.06),
            ("B.CÁLC ICMS", 0.06),
            ("VALOR ICMS", 0.06),
            ("VALOR IPI", 0.05),
            ("ALÍQ. ICMS", 0.035),
        ];
        let mut ws = [0.0f64; 14];
        for (i, (title, fraction)) in columns.iter().enumerate() {
            let wi = round_to(w * fraction, 0);
            ws[i] = wi;
            pdf.set_font_size_style(6.0, "");
            pdf.text_box_force(x, y, wi, h + 2.0, title, 'C', 'C', 0.0, false);
            pdf.line(x + wi, y, x + wi, y + h_max);
            x += wi;
        }
        // ALÍQ. IPI takes the remaining width.
        let w14 = w - ws[..13].iter().sum::<f64>();
        ws[13] = w14;
        pdf.text_box_force(x, y, w14, h + 2.0, "ALÍQ. IPI", 'C', 'C', 0.0, false);
        pdf.line(old_x, y + h + 1.0, old_x + w, y + h + 1.0);
        y += 5.0;

        // ---- items ----------------------------------------------------------------------
        let mut h_usado = self.h_cabec_itens;
        pdf.set_font_size_style(7.0, "");
        while self.current_item < nfe.det.len() {
            let item = self.current_item;
            let det = &nfe.det[item];

            let texto_produto = ctx.manter_xprod(item);
            h = self.calculate_height_produto(pdf, width, item);
            h_usado += h;
            if h_usado >= h_max {
                // It does not fit the page: the rest goes to the next ones.
                break;
            }
            let y_linha = y + h;

            // Line between items.
            if y_linha <= old_y + h_max {
                pdf.dashed_line(old_x, y_linha, old_x + w, y_linha, 1.0);
            }
            x = old_x;
            // Product code.
            let texto = ctx.manter_codigo(&det.prod.c_ean, &det.prod.c_prod);
            pdf.text_box(x, y, ws[0], h, &texto, 'T', 'C', 0.0);
            x += ws[0];
            // Description.
            pdf.text_box_force(x, y, ws[1], h, &texto_produto, 'T', 'L', 0.0, false);
            x += ws[1];
            // NCM.
            pdf.text_box(x, y, ws[2], h, &det.prod.ncm, 'T', 'C', 0.0);
            x += ws[2];
            // CST / CSOSN.
            let texto = format!(
                "{}/{}",
                det.imposto.icms.orig,
                ctx.manter_cst(nfe.emit.crt, &det.imposto.icms.csosn, &det.imposto.icms.cst)
            );
            pdf.text_box(x, y, ws[3], h, &texto, 'T', 'C', 0.0);
            // CFOP.
            x += ws[3];
            pdf.text_box(x, y, ws[4], h, &det.prod.cfop, 'T', 'C', 0.0);
            // Unit.
            x += ws[4];
            pdf.text_box(x, y, ws[5], h, &det.prod.u_com, 'T', 'C', 0.0);
            x += ws[5];
            // Quantity.
            pdf.text_box(
                x,
                y,
                ws[6],
                h,
                &det.prod.q_com.format_br(4, true),
                'T',
                'R',
                0.0,
            );
            x += ws[6];
            // Unit value.
            pdf.text_box(
                x,
                y,
                ws[7],
                h,
                &det.prod.v_un_com.format_br(4, true),
                'T',
                'R',
                0.0,
            );
            x += ws[7];
            // Product value.
            let texto = ctx.manter_vprod(det.prod.v_prod, det.prod.v_desc);
            pdf.text_box(x, y, ws[8], h, &texto, 'T', 'R', 0.0);
            // ICMS base.
            x += ws[8];
            pdf.text_box(
                x,
                y,
                ws[9],
                h,
                &det.imposto.icms.v_bc.money(),
                'T',
                'R',
                0.0,
            );
            // ICMS value.
            x += ws[9];
            pdf.text_box(
                x,
                y,
                ws[10],
                h,
                &det.imposto.icms.v_icms.money(),
                'T',
                'R',
                0.0,
            );
            // IPI value.
            x += ws[10];
            pdf.text_box(
                x,
                y,
                ws[11],
                h,
                &det.imposto.ipi.v_ipi.money(),
                'T',
                'R',
                0.0,
            );
            // ICMS rate.
            x += ws[11];
            pdf.text_box(
                x,
                y,
                ws[12],
                h,
                &det.imposto.icms.p_icms.money(),
                'T',
                'C',
                0.0,
            );
            // IPI rate.
            x += ws[12];
            pdf.text_box(
                x,
                y,
                ws[13],
                h,
                &det.imposto.ipi.p_ipi.money(),
                'T',
                'C',
                0.0,
            );

            y += h;
            self.current_item += 1;
        }

        args.draw_again = self.current_item < nfe.det.len();
    }
}
