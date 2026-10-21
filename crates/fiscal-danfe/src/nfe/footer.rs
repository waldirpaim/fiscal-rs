//! Additional information, footer message and watermarks.

use std::sync::Arc;

use fiscal_pdf::report::{BandHandler, BandState, DrawArgs, InitArgs, ReportPdf};
use fiscal_pdf::{Color, Fpdf, Orientation, PageFormat, Unit, round_to};

use crate::context::DanfeContext;
use crate::model::{TpAmb, TpEmis};

/// "DADOS ADICIONAIS" (page footer).
pub(super) struct DadosAdicionais {
    pub(super) ctx: Arc<DanfeContext>,
}

impl DadosAdicionais {
    /// Vertical space the additional information needs.
    fn espaco_vertical(&self, width: f64, texto_adic: &str) -> f64 {
        let mut pdf = Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4);
        pdf.set_font("Times", "", 7.0);
        let mut numlinhasdados = 0.0;
        if !texto_adic.is_empty() {
            for linha in texto_adic.split('\n') {
                let linha = linha.trim_start_matches(|c: char| c <= ' ');
                numlinhasdados += pdf.get_num_lines(linha, width, 0.0) as f64;
            }
        }
        let mut result = round_to((numlinhasdados + 3.0) * pdf.font_size(), 0);
        if result < 10.0 {
            result = 10.0;
        }
        result + 4.0
    }
}

impl BandHandler for DadosAdicionais {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.visible = true;
        band.height = self.espaco_vertical(band.width, &self.ctx.texto_adicional());
    }

    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let width = band.width;
        let pdf = &mut args.pdf;
        let x = 0.0;
        let mut y = 1.0;
        let h = band.height;

        // DADOS ADICIONAIS
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, width, 8.0, "DADOS ADICIONAIS", 'T', 'L', 0.0);
        // INFORMAÇÕES COMPLEMENTARES
        y += 3.0;
        let w = round_to(width * 0.66, 0);
        pdf.set_font_size_style(6.0, "B");
        pdf.text_box(
            x,
            y,
            w,
            h - 4.0,
            "INFORMAÇÕES COMPLEMENTARES",
            'T',
            'L',
            1.0,
        );
        y += 1.0;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box_force(
            x,
            y + 2.0,
            w - 2.0,
            h - 3.0,
            &self.ctx.texto_adicional(),
            'T',
            'L',
            0.0,
            false,
        );
        // RESERVADO AO FISCO
        let x = x + w;
        y -= 1.0;
        let w = width - w;
        pdf.set_font_size_style(6.0, "B");
        pdf.text_box(x, y, w, h - 4.0, "RESERVADO AO FISCO", 'T', 'L', 1.0);
    }
}

/// Footer message (page footer).
pub(super) struct Rodape {
    pub(super) mensagem: String,
}

impl BandHandler for Rodape {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.height = 5.0;
    }

    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        if self.mensagem.is_empty() {
            return;
        }
        let parts: Vec<&str> = self.mensagem.split('|').collect();
        let (x, y, w, h) = (0.0, 0.0, band.width, band.height);
        let pdf = &mut args.pdf;
        pdf.set_font_size_style(6.0, "I");
        if let Some(text) = parts.first() {
            pdf.text_box(x, y, w, h, text, 'T', 'L', 0.0);
        }
        if let Some(text) = parts.get(1) {
            pdf.text_box(x, y, w, h, text, 'T', 'C', 0.0);
        }
        if let Some(text) = parts.get(2) {
            pdf.text_box(x, y, w, h, text, 'T', 'R', 0.0);
        }
    }
}

/// Watermark: canceled, denied, contingency, homologation, missing protocol.
pub(super) struct MarcaDagua {
    pub(super) ctx: Arc<DanfeContext>,
    pub(super) cancelada: bool,
}

/// Draws one watermark text and returns `y`.
#[allow(clippy::too_many_arguments)]
fn watermark(
    pdf: &mut ReportPdf<'_>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    texto: &str,
    font_size: f64,
    font_style: &str,
) -> f64 {
    let family = pdf.current_font_family();
    pdf.set_text_color(Color::Rgb(230, 230, 230));
    pdf.set_font(&family, font_style, font_size);
    pdf.text_box_flags(x, y, w, h, texto, 'C', 'C', false, false, true);
    pdf.set_text_color(Color::Rgb(0, 0, 0));
    y
}

impl BandHandler for MarcaDagua {
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let ctx = &self.ctx;
        let nfe = &ctx.nfe;
        let (width, height) = (band.width, band.height);
        let m = 5.0;
        let x = args.left_margin + m;
        let y = height - 130.0;
        let pdf = &mut args.pdf;

        // Canceled.
        if self.cancelada || ctx.nota_cancelada() {
            watermark(
                pdf,
                x,
                y,
                width - 2.0 * m,
                25.0,
                "NOTA CANCELADA",
                50.0,
                "B",
            );
        }
        if ctx.nota_epec() {
            watermark(
                pdf,
                x,
                height - 130.0,
                width - 2.0 * m,
                25.0,
                "DANFE impresso em contingência -\nDPEC regularmente recebido pela Receita\nFederal do Brasil",
                48.0,
                "B",
            );
        }
        if ctx.nota_denegada() {
            let y = watermark(
                pdf,
                x,
                height - 130.0,
                width - 2.0 * m,
                25.0,
                "NFe USO DENEGADO",
                48.0,
                "B",
            );
            watermark(
                pdf,
                x,
                y + 35.0,
                width - 2.0 * m,
                5.0,
                &format!("SEM VALOR FISCAL\n{}", nfe.proc_nfe.x_motivo),
                48.0,
                "B",
            );
        }

        if nfe.ide.tp_amb == TpAmb::Homologacao {
            let y = watermark(
                pdf,
                x,
                round_to(height / 2.0, 0),
                width - 2.0 * m,
                5.0,
                "SEM VALOR FISCAL",
                48.0,
                "B",
            );
            watermark(
                pdf,
                x,
                y + 14.0,
                width - 2.0 * m,
                5.0,
                "AMBIENTE DE HOMOLOGAÇÃO",
                30.0,
                "B",
            );
        } else {
            let y = round_to(height / 2.0, 0);
            let h = 25.0;
            let w = width - 2.0 * m;
            pdf.set_text_color(Color::Rgb(200, 200, 200));

            if matches!(nfe.ide.tp_emis, TpEmis::Contingencia | TpEmis::Fsda) && !ctx.nota_epec() {
                watermark(pdf, x, y, w, h, "DANFE Emitido em Contingência", 48.0, "B");
                watermark(
                    pdf,
                    x,
                    y + 14.0,
                    w,
                    h,
                    "devido à problemas técnicos",
                    30.0,
                    "B",
                );
            } else if nfe.proc_nfe.n_prot.is_empty() {
                if !ctx.nota_epec() {
                    watermark(pdf, x, y, w, h, "SEM VALOR FISCAL", 48.0, "B");
                    watermark(
                        pdf,
                        x,
                        y + 25.0,
                        w,
                        h,
                        "FALTA PROTOCOLO DE AUTORIZAÇÃO DA SEFAZ",
                        30.0,
                        "B",
                    );
                } else {
                    watermark(
                        pdf,
                        x,
                        y + 35.0,
                        w,
                        h,
                        "FALTA PROTOCOLO DE AUTORIZAÇÃO DA SEFAZ",
                        30.0,
                        "B",
                    );
                }
            }
            pdf.set_text_color(Color::Rgb(0, 0, 0));
        }
    }
}
