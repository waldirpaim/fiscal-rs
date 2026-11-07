//! Invoice/instalments and payment bands.

use std::sync::Arc;

use fiscal_pdf::report::{BandHandler, BandState, DrawArgs, InitArgs};
use fiscal_pdf::{Orientation, round_to};

use crate::context::DanfeContext;
use crate::conversions;
use crate::format;

const DASH_BLACK: f64 = 0.4;
const DASH_WHITE: f64 = 0.8;

/// "FATURA / DUPLICATA".
pub(super) struct FaturaDuplicatas {
    pub(super) ctx: Arc<DanfeContext>,
    pub(super) duplicatas_qtd_max: usize,
    pub(super) duplicatas_max_linhas: usize,
}

impl FaturaDuplicatas {
    pub(super) fn new(ctx: Arc<DanfeContext>) -> Self {
        Self {
            ctx,
            duplicatas_qtd_max: 15,
            duplicatas_max_linhas: 8,
        }
    }

    /// Extra height of the invoice line when instalments are printed too.
    fn size_extra_texto_fatura(&self) -> f64 {
        // The invoice text is never empty in the original.
        if !self.ctx.nfe.cobr.dup.is_empty() {
            5.0
        } else {
            0.0
        }
    }
}

impl BandHandler for FaturaDuplicatas {
    fn on_init(&mut self, band: &mut BandState, args: &InitArgs) {
        let nfe = &self.ctx.nfe;
        band.visible = !nfe.cobr.dup.is_empty();
        if !band.visible {
            return;
        }
        let portrait = args.orientation == Orientation::Portrait;
        self.duplicatas_qtd_max = if portrait { 15 } else { 20 };
        self.duplicatas_max_linhas = if portrait { 8 } else { 6 };

        if !self.ctx.options.exibe_campo_duplicata {
            band.height = self.size_extra_texto_fatura() + 4.0;
            return;
        }
        let qtd_pagamentos = if !nfe.cobr.dup.is_empty() {
            nfe.cobr.dup.len()
        } else if !nfe.pag.is_empty() {
            nfe.pag.len()
        } else {
            0
        };
        let mut linhas_dup = qtd_pagamentos / self.duplicatas_qtd_max;
        if qtd_pagamentos % self.duplicatas_qtd_max > 0 {
            linhas_dup += 1;
        }
        linhas_dup = linhas_dup.min(self.duplicatas_max_linhas);
        band.height = (if linhas_dup > 0 {
            4.0 + linhas_dup as f64 * 8.0
        } else {
            0.0
        }) + self.size_extra_texto_fatura();
    }

    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let width = band.width;
        let ctx = Arc::clone(&self.ctx);
        let nfe = &ctx.nfe;
        let pdf = &mut args.pdf;

        let mut x = 0.0;
        let mut y = 1.0;
        let old_x = x;
        let texto_fatura = ctx.texto_fatura();

        let mut h = 8.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, width, h, "FATURA / DUPLICATA", 'T', 'L', 0.0);
        y += 3.0;
        let mut dup_cont = 0usize;
        if !texto_fatura.is_empty() && ctx.options.exibe_campo_fatura {
            let my_h = 4.0;
            pdf.set_font_size_style(8.0, "");
            pdf.text_box(x, y, width, my_h, &texto_fatura, 'T', 'L', 1.0);
            y += my_h + 1.0;
        }

        if ctx.options.exibe_campo_duplicata {
            let qtd = self.duplicatas_qtd_max;
            let max_linhas = self.duplicatas_max_linhas;
            let w = (width - 6.0) / qtd as f64;

            let mut old_y = y;
            let mut linha = 1usize;
            for dup in &nfe.cobr.dup {
                y = old_y;
                h = 8.0;
                dup_cont += 1;

                if dup_cont % qtd == 1 {
                    pdf.set_dash_width(0.0);
                    pdf.line(x, y, x + 6.0, y);
                    pdf.line(x, y, x, y + h);
                    pdf.line(x + 6.0, y, x + 6.0, y + h);
                    pdf.line(x + 6.0 + w * qtd as f64, y, x + 6.0 + w * qtd as f64, y + h);

                    if linha % max_linhas != 1 {
                        pdf.set_dash(DASH_BLACK, DASH_WHITE);
                    }
                    pdf.line(x + 6.0, y, x + 6.0 + w * qtd as f64, y);

                    pdf.set_font_size_style(6.0, "");
                    pdf.text_box(x, y, 6.0, h, "Num.", 'T', 'L', 0.0);
                    y += 2.5;
                    pdf.text_box(x, y, 6.0, h, "Venc.", 'T', 'L', 0.0);
                    y += 2.5;
                    pdf.text_box(x, y, 6.0, h, "Valor", 'T', 'L', 0.0);
                    x += 6.0;
                    y = old_y;
                }

                if dup_cont % qtd > 0 {
                    pdf.set_dash(DASH_BLACK, DASH_WHITE);
                    pdf.line(x + w, y, x + w, y + h);
                }

                pdf.set_font_size_style(7.0, "");
                if dup.n_dup.trim().parse::<i64>().unwrap_or(0) > 0 {
                    pdf.text_box_flags(x, y, w, h, &dup.n_dup, 'T', 'R', false, false, false);
                } else {
                    pdf.text_box(
                        x,
                        y,
                        w,
                        h,
                        &format::zero_pad(dup_cont as u64, 3),
                        'T',
                        'L',
                        1.0,
                    );
                }

                y += 2.5;
                let venc = dup.d_venc.map(|d| d.date_br()).unwrap_or_default();
                pdf.text_box(x, y, w, h, &venc, 'T', 'R', 0.0);

                y += 2.5;
                pdf.text_box(x, y, w, h, &dup.v_dup.money(), 'T', 'R', 0.0);
                x += w;

                if dup_cont >= qtd {
                    old_y += h;
                    x = old_x;
                    dup_cont = 0;
                    linha += 1;
                }
                if linha > max_linhas {
                    break;
                }
            }
            y = old_y;

            if dup_cont == 0 {
                y -= h;
            }

            if !nfe.cobr.dup.is_empty() {
                pdf.set_dash_width(0.0);
                pdf.line(old_x, y + h, old_x + 6.0 + w * qtd as f64, y + h);
            }
        }
    }
}

/// "PAGAMENTO" boxes.
pub(super) struct Pagamentos {
    pub(super) ctx: Arc<DanfeContext>,
}

impl BandHandler for Pagamentos {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        let nfe = &self.ctx.nfe;
        band.visible = true;
        if !nfe.cobr.dup.is_empty() || nfe.pag.is_empty() {
            band.visible = false;
        }
        if nfe.pag.len() == 1 && nfe.pag[0].t_pag == "90" {
            band.visible = false;
        }
        band.height = 12.0;
    }

    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let width = band.width;
        let nfe = &self.ctx.nfe;
        let pdf = &mut args.pdf;
        let portrait = pdf.orientation() == Orientation::Portrait;

        let mut x = 0.0;
        let mut y = 1.0;
        let mut linha = 1;
        let old_x = x;

        let max_dup_cont = if portrait { 3 } else { 8 };
        let mut h = 8.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, width, h, "PAGAMENTO", 'T', 'L', 0.0);
        y += 3.0;
        let mut dup_cont = 0;
        let w = if portrait {
            round_to(width / 3.968, 0) - 1.0
        } else {
            28.0
        };
        let increm = 0.7;

        for pag in &nfe.pag {
            h = 8.0;
            if pag.t_pag == "90" {
                continue;
            }
            pdf.text_box_frame(x, y, w, h);

            pdf.set_font_size_style(6.0, "");
            pdf.text_box(x, y + 0.5, w, h, "Forma", 'T', 'L', 0.0);
            pdf.set_font_size_style(7.0, "B");
            let texto = conversions::forma_pagamento(&pag.t_pag, &pag.x_pag);
            pdf.text_box(x, y + 0.5, w, h, &texto, 'T', 'R', 0.0);

            pdf.set_font_size_style(6.0, "");
            pdf.text_box(x, y + 4.0, w, h, "Valor", 'T', 'L', 0.0);
            pdf.set_font_size_style(7.0, "B");
            let texto = format!("R$ {}", pag.v_pag.money());
            pdf.text_box(x, y + 4.0, w, h, &texto, 'T', 'R', 0.0);
            x += w + increm;
            dup_cont += 1;

            if dup_cont > max_dup_cont {
                y += h + increm;
                x = old_x;
                dup_cont = 0;
                linha += 1;
            }
            if linha == 5 {
                break;
            }
        }
    }
}
