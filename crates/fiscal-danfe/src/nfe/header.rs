//! Receipt stub (canhoto) and the DANFE header (issuer, DANFE title, access key, protocol).

use std::sync::Arc;

use fiscal_pdf::report::{BandHandler, BandState, DrawArgs, InitArgs, ReportPdf, image_size};
use fiscal_pdf::{Color, Orientation, round_to};

use crate::context::DanfeContext;
use crate::format;
use crate::model::{TpEmis, TpNf};
use crate::options::PosCanhoto;

/// Font of the whole DANFE.
pub(super) const DEFAULT_FONT_FAMILY: &str = "Times";

/// Where the logo goes relative to the issuer data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // `Left` exists in the original but no option selects it.
pub(super) enum LogoAlign {
    Left,
    Center,
    Right,
    Full,
}

/// Receipt stub ("RECEBEMOS DE...").
pub(super) struct Canhoto {
    pub(super) ctx: Arc<DanfeContext>,
    pub(super) align: PosCanhoto,
}

impl Canhoto {
    fn endereco(ctx: &DanfeContext) -> String {
        let d = &ctx.nfe.dest;
        let a = &d.ender_dest;
        format!(
            "{} - {}, {} - {}{} {}-{}",
            d.x_nome,
            a.x_lgr,
            a.nro,
            if a.x_cpl.is_empty() {
                String::new()
            } else {
                format!("{} - ", a.x_cpl)
            },
            a.x_bairro,
            a.x_mun,
            a.uf
        )
    }

    #[allow(clippy::many_single_char_names)]
    fn draw_canhoto(&self, pdf: &mut ReportPdf<'_>, v_x: f64, v_y: f64, v_w: f64, v_h: f64) {
        let ctx = &self.ctx;
        let nfe = &ctx.nfe;
        let font = |pdf: &mut ReportPdf<'_>| pdf.set_font(DEFAULT_FONT_FAMILY, "", 7.0);

        let (mut x, mut y) = (v_x, v_y);
        let old_x = x;
        if self.align == PosCanhoto::Rodape {
            y += 1.0;
            pdf.dashed_line(x, y, x + v_w, y, 1.0);
            y += 1.0;
        }

        let (emitente, destinatario) = if nfe.ide.tp_nf == TpNf::Entrada {
            (Self::endereco(ctx), format!("{} ", nfe.emit.x_nome))
        } else {
            (
                format!("{} ", nfe.emit.x_nome),
                format!("{} ", Self::endereco(ctx)),
            )
        };

        let w = round_to(v_w * 0.81, 0);
        let h = v_h;
        // The box.
        font(pdf);
        pdf.text_box_force(x, y, w, h, "", 'C', 'L', 1.0, false);

        let num_nf = if ctx.options.formatar_numero_documento {
            format::numero_documento_fiscal(&nfe.ide.n_nf.to_string())
        } else {
            nfe.ide.n_nf.to_string()
        };
        let serie = format::zero_pad(u64::from(nfe.ide.serie), 3);
        let texto = format!(
            "RECEBEMOS DE {emitente} OS PRODUTOS E/OU SERVIÇOS CONSTANTES DA NOTA FISCAL ELETRÔNICA INDICADA {}. \
             EMISSÃO: {} VALOR TOTAL: R$ {} DESTINATÁRIO: {destinatario}",
            if pdf.orientation() == Orientation::Portrait {
                "ABAIXO"
            } else {
                "AO LADO"
            },
            nfe.ide.d_emi.map(|d| d.date_br()).unwrap_or_default(),
            nfe.total.icms_tot.v_nf.money(),
        );
        pdf.text_box_force(x, y, w - 1.0, h, &texto, 'C', 'L', 0.0, false);

        let x1 = x + w;
        let w1 = v_w - w;
        pdf.set_font_size_style(14.0, "B");
        pdf.text_box(x1, y, w1, 18.0, "NF-e", 'T', 'C', 1.0);
        let texto = format!("Nº. {num_nf}\nSérie {serie}");
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(x1, y + 2.0, w1, 18.0, &texto, 'C', 'C', 0.0);

        // DATA DE RECEBIMENTO
        y += h;
        let w2 = round_to(v_w * 0.17, 0);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w2, 8.0, "DATA DE RECEBIMENTO", 'T', 'L', 1.0);
        // IDENTIFICAÇÃO E ASSINATURA DO RECEBEDOR
        x += w2;
        let w3 = w - w2;
        pdf.text_box(
            x,
            y,
            w3,
            8.0,
            "IDENTIFICAÇÃO E ASSINATURA DO RECEBEDOR",
            'T',
            'L',
            1.0,
        );
        x = old_x;

        if matches!(self.align, PosCanhoto::Cabecalho | PosCanhoto::Esquerda) {
            y += 9.0;
            pdf.dashed_line(x, y, x + v_w, y, 1.0);
        }
    }
}

impl BandHandler for Canhoto {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        match self.align {
            PosCanhoto::Cabecalho | PosCanhoto::Rodape => band.height = 20.0,
            PosCanhoto::Esquerda => band.width = 20.0,
            PosCanhoto::Nenhum => {}
        }
        // Because of the double pass.
        band.visible = self.align != PosCanhoto::Nenhum;
    }

    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        match self.align {
            PosCanhoto::Cabecalho | PosCanhoto::Rodape => {
                self.draw_canhoto(&mut args.pdf, 0.0, 0.0, band.width, 10.0);
            }
            PosCanhoto::Esquerda => {
                let (x, y) = (0.0, band.height);
                args.pdf.rotate(90.0, x, y);
                self.draw_canhoto(&mut args.pdf, x, y, band.height, 10.0);
                args.pdf.rotate(0.0, x, y);
            }
            PosCanhoto::Nenhum => {}
        }
        // Not shown on the next pages.
        band.visible = false;
    }
}

/// Header: issuer, DANFE title, barcode and the first fiscal fields.
pub(super) struct DadosNfe {
    pub(super) ctx: Arc<DanfeContext>,
    pub(super) logo: Vec<u8>,
    pub(super) logo_stretched: bool,
    pub(super) logo_align: LogoAlign,
}

impl BandHandler for DadosNfe {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.height = 46.0;
    }

    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let width = band.width;
        let nfe = &self.ctx.nfe;
        let final_pass = args.final_pass;
        let total_pages = args.total_pages;
        let pdf = &mut args.pdf;

        let mut x = 0.0;
        let mut y = 0.0;
        let old_x = x;
        let mut old_y = y;
        let mut has_logo = false;

        // ---- left column: issuer ------------------------------------------------------
        let mut w = round_to(width * 0.41, 0);
        if pdf.orientation() == Orientation::Portrait {
            pdf.set_font_size_style(6.0, "I");
        } else {
            pdf.set_font_size_style(8.0, "B");
        }
        let w1 = w;
        let h = 32.0;
        old_y += h;
        pdf.text_box_frame(x, y, w, h);
        pdf.text_box(x, y, w, 5.0, "IDENTIFICAÇÃO DO EMITENTE", 'T', 'C', 0.0);

        let mut x1 = x;
        let mut y1 = y;
        let mut tw = w;
        let mut th = h;

        // Logo placement: left/right take a third of the width, centre takes the top third
        // of the height, full fills the box.
        if final_pass && !self.logo.is_empty() {
            if let Some((logo_w, logo_h)) = image_size(&self.logo) {
                has_logo = true;
                let logo_w_mm = f64::from(logo_w) / 72.0 * 25.4;
                let logo_h_mm = f64::from(logo_h) / 72.0 * 25.4;
                let (x_img, y_img, n_img_w, n_img_h);
                match self.logo_align {
                    LogoAlign::Left => {
                        let mut iw = round_to(w / 3.0, 0);
                        let mut ih = if self.logo_stretched {
                            h - 5.0
                        } else {
                            round_to(logo_h_mm * (iw / logo_w_mm), 0)
                        };
                        if ih > h - 5.0 {
                            ih = h - 5.0;
                            iw = round_to(logo_w_mm * (ih / logo_h_mm), 0);
                        }
                        (n_img_w, n_img_h) = (iw, ih);
                        x_img = x + 1.0;
                        y_img = round_to((h - n_img_h) / 2.0, 0) + y;
                        x1 = round_to(x_img + n_img_w + 1.0, 0);
                        tw = round_to(2.0 * w / 3.0, 0);
                    }
                    LogoAlign::Center => {
                        n_img_h = round_to(h / 3.0, 0);
                        n_img_w = if self.logo_stretched {
                            w
                        } else {
                            round_to(logo_w_mm * (n_img_h / logo_h_mm), 0)
                        };
                        x_img = round_to((w - n_img_w) / 2.0 + x, 0);
                        y_img = y + 3.0;
                        x1 = x;
                        y1 += n_img_h;
                        tw = w;
                        th -= n_img_h;
                    }
                    LogoAlign::Right => {
                        let mut iw = round_to(w / 3.0, 0);
                        let mut ih = if self.logo_stretched {
                            h - 5.0
                        } else {
                            round_to(logo_h_mm * (iw / logo_w_mm), 0)
                        };
                        if ih > h - 5.0 {
                            ih = h - 5.0;
                            iw = round_to(logo_w_mm * (ih / logo_h_mm), 0);
                        }
                        (n_img_w, n_img_h) = (iw, ih);
                        x_img = round_to(x + (w - (1.0 + n_img_w)), 0);
                        y_img = round_to((h - n_img_h) / 2.0, 0) + y;
                        x1 = x;
                        tw = round_to(2.0 * w / 3.0, 0);
                    }
                    LogoAlign::Full => {
                        let mut ih = round_to(h - 5.0, 0);
                        let iw = if self.logo_stretched {
                            round_to(w - 5.0, 0)
                        } else {
                            let mut iw = round_to(logo_w_mm * (ih / logo_h_mm), 0);
                            if iw > round_to(w, 0) {
                                iw = round_to(w - 5.0, 0);
                                ih = round_to(logo_h_mm * (iw / logo_w_mm), 0);
                            }
                            iw
                        };
                        (n_img_w, n_img_h) = (iw, ih);
                        x_img = round_to((w - n_img_w) / 2.0 + x, 0);
                        y_img = round_to((h - n_img_h) / 2.0, 0) + y;
                        x1 = x;
                        tw = w;
                    }
                }
                pdf.image(x_img, y_img, n_img_w, n_img_h, &self.logo, 'T', 'L', false);
            }
        }

        // Issuer data, unless the logo fills the box.
        if !has_logo || self.logo_align != LogoAlign::Full {
            let e = &nfe.emit.ender_emit;
            let emitente_conteudo1 = nfe.emit.x_nome.clone();
            let mut emitente_conteudo2 = format!("{}, {}", e.x_lgr, e.nro);
            if !e.x_cpl.is_empty() {
                emitente_conteudo2.push_str(&format!(" - {}", e.x_cpl));
            }
            emitente_conteudo2.push_str(&format!(
                "\n{} - {} - {}\nCEP: {}",
                e.x_bairro,
                e.x_mun,
                e.uf,
                if e.cep > 0 {
                    format::cep_number(e.cep)
                } else {
                    String::new()
                }
            ));
            if !e.fone.is_empty() {
                emitente_conteudo2.push_str(&format!("\nFone/Fax: {}", e.fone));
            }

            x1 += 1.0;
            tw -= 2.0;

            pdf.set_font_size_style(8.0, "");
            pdf.word_wrap(&mut emitente_conteudo2, tw, 0.0);
            let h_emit2 = pdf.get_string_height(&emitente_conteudo2, tw, 0.0, 0.0);

            let mut emit1_font_size = 12.5;
            let mut h_emit1 = 0.0;
            loop {
                if emit1_font_size <= 8.0 {
                    break;
                }
                emit1_font_size -= 0.5;
                let mut texto = emitente_conteudo1.clone();
                pdf.set_font_size_style(emit1_font_size, "B");
                pdf.word_wrap(&mut texto, tw, 0.0);
                h_emit1 = pdf.get_string_height(&texto, tw, 0.0, 0.0);
                if h_emit1 + h_emit2 < th - 2.0 {
                    break;
                }
            }

            // Issuer name.
            pdf.set_font_size_style(emit1_font_size, "B");
            y1 += (th - (h_emit1 + h_emit2)) / 2.0 + 1.0;
            y1 += pdf.text_box_flags(
                x1,
                y1,
                tw,
                h_emit1,
                &emitente_conteudo1,
                'T',
                'C',
                false,
                true,
                false,
            );
            // Address.
            pdf.set_font_size_style(8.0, "");
            pdf.text_box(x1, y1, tw, h_emit2, &emitente_conteudo2, 'T', 'C', 0.0);
        }

        // ---- centre column: DANFE ----------------------------------------------------
        x += w;
        w = round_to(width * 0.17, 0);
        let w2 = w;
        let mut h = 32.0;
        pdf.text_box_frame(x, y, w, h);

        pdf.set_font_size_style(14.0, "B");
        pdf.text_box(x, y + 1.0, w, h, "DANFE", 'T', 'C', 0.0);
        pdf.set_font_size_style(8.0, "");
        h = 20.0;
        pdf.text_box_force(
            x,
            y + 6.0,
            w,
            h,
            "Documento Auxiliar da Nota Fiscal Eletrônica",
            'T',
            'C',
            0.0,
            false,
        );

        pdf.set_font_size_style(8.0, "");
        y1 = y + 14.0;
        h = 8.0;
        pdf.text_box(x + 2.0, y1, w, h, "0 - ENTRADA", 'T', 'L', 0.0);
        y1 = y + 17.0;
        pdf.text_box(x + 2.0, y1, w, h, "1 - SAÍDA", 'T', 'L', 0.0);
        // Operation type.
        pdf.set_font_size_style(12.0, "B");
        y1 = y + 14.0;
        h = 6.0;
        pdf.text_box(x + 27.0, y1, 5.0, h, nfe.ide.tp_nf.code(), 'C', 'C', 1.0);

        // Number.
        pdf.set_font_size_style(10.0, "B");
        y1 = y + 22.0;
        let texto = format!(
            "Nº. {}",
            format::numero_documento_fiscal(&nfe.ide.n_nf.to_string())
        );
        pdf.text_box(x, y1, w, h, &texto, 'C', 'C', 0.0);
        // Series.
        y1 = y + 25.0;
        let texto = format!("Série {}", format::zero_pad(u64::from(nfe.ide.serie), 3));
        pdf.text_box(x, y1, w, h, &texto, 'C', 'C', 0.0);
        // Page.
        pdf.set_font_size_style(8.0, "I");
        y1 = y + 28.0;
        let texto = format!("Folha {}/{}", pdf.current_page(), total_pages);
        pdf.text_box(x, y1, w, h, &texto, 'C', 'C', 0.0);

        // ---- right column: barcode ----------------------------------------------------
        x += w;
        w = width - w1 - w2;
        let w3 = w;
        h = 32.0;
        pdf.text_box_frame(x, y, w, h);
        pdf.set_fill_color(Color::Rgb(0, 0, 0));
        let b_w = 75.0;
        let b_h = 12.0;
        pdf.code128(&nfe.chave(), x + (w - b_w) / 2.0, y + 2.0, b_h, b_w);
        pdf.line(x, y + 4.0 + b_h, x + w, y + 4.0 + b_h);
        pdf.line(x, y + 12.0 + b_h, x + w, y + 12.0 + b_h);
        pdf.set_font_size_style(6.0, "");
        y1 = y + 4.0 + b_h;
        h = 7.0;
        pdf.text_box(x, y1, w, h, "CHAVE DE ACESSO", 'T', 'L', 0.0);
        pdf.set_font_size_style(10.0, "B");
        y1 = y + 8.0 + b_h;
        pdf.text_box(
            x + 2.0,
            y1 - 1.0,
            w - 4.0,
            h,
            &format::chave_acesso(&nfe.id),
            'T',
            'C',
            0.0,
        );
        y1 = y + 12.0 + b_h;
        pdf.set_font_size_style(8.0, "");

        let mut chave_contingencia = String::new();
        if matches!(nfe.ide.tp_emis, TpEmis::Contingencia | TpEmis::Fsda) && !self.ctx.nota_epec() {
            chave_contingencia = self.ctx.chave_contingencia();
            pdf.set_fill_color(Color::Rgb(0, 0, 0));
            pdf.code128(
                &chave_contingencia,
                x + (w - b_w) / 2.0 + 4.0,
                y1 + 1.0,
                b_h / 2.0,
                b_w * 0.9,
            );
        } else {
            pdf.text_box(
                x + 2.0,
                y1,
                w - 2.0,
                h,
                "Consulta de autenticidade no portal nacional da NF-e",
                'T',
                'C',
                0.0,
            );
            y1 = y + 16.0 + b_h;
            pdf.text_box(
                x + 2.0,
                y1,
                w - 2.0,
                h,
                "www.nfe.fazenda.gov.br/portal ou no site da Sefaz Autorizadora",
                'T',
                'C',
                0.0,
            );
        }

        // ---- fiscal fields --------------------------------------------------------------
        pdf.set_font_size_style(6.0, "");
        w = w1 + w2;
        y = old_y;
        x = old_x;
        h = 7.0;
        pdf.text_box(x, y, w, h, "NATUREZA DA OPERAÇÃO", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(x, y, w, h, &nfe.ide.nat_op, 'B', 'L', 0.0);
        x += w;
        w = w3;

        let (descricao, valor) = match nfe.ide.tp_emis {
            TpEmis::Normal | TpEmis::SvcAn | TpEmis::Scan | TpEmis::SvcRs | TpEmis::SvcSp => {
                let descricao = if self.ctx.nota_denegada() {
                    "PROTOCOLO DE DENEGAÇÃO DE USO"
                } else {
                    "PROTOCOLO DE AUTORIZAÇÃO DE USO"
                };
                let valor = if nfe.proc_nfe.n_prot.is_empty() {
                    "NFe sem Autorização de Uso da SEFAZ".to_owned()
                } else {
                    format!(
                        "{} {}",
                        nfe.proc_nfe.n_prot,
                        nfe.proc_nfe
                            .dh_recbto
                            .map(|d| d.datetime_br())
                            .unwrap_or_default()
                    )
                };
                (descricao, valor)
            }
            TpEmis::Contingencia | TpEmis::Fsda => ("DADOS DA NF-E", chave_contingencia),
            TpEmis::Dpec => {
                if nfe.proc_nfe.n_prot.is_empty() {
                    ("NÚMERO DE REGISTRO DPEC", String::new())
                } else {
                    (
                        "PROTOCOLO DE AUTORIZAÇÃO DO EPEC",
                        format!(
                            "{} {}",
                            nfe.proc_nfe.n_prot,
                            nfe.proc_nfe
                                .dh_recbto
                                .map(|d| d.datetime_br())
                                .unwrap_or_default()
                        ),
                    )
                }
            }
            TpEmis::OffLine => ("", String::new()),
        };
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, descricao, 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(x, y, w, h, &valor, 'B', 'C', 0.0);

        // INSCRIÇÃO ESTADUAL
        w = round_to(width * 0.333, 0);
        y += h;
        x = old_x;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "INSCRIÇÃO ESTADUAL", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(x, y, w, h, &nfe.emit.ie, 'B', 'L', 0.0);
        // INSCRIÇÃO ESTADUAL DO SUBST. TRIBUT.
        x += w;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(
            x,
            y,
            w,
            h,
            "INSCRIÇÃO ESTADUAL DO SUBST. TRIBUT.",
            'T',
            'L',
            1.0,
        );
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(x, y, w, h, &nfe.emit.iest, 'B', 'L', 0.0);
        // CNPJ
        x += w;
        w = width - 2.0 * w;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "CNPJ", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(
            x,
            y,
            w,
            h,
            &format::cnpj_ou_cpf(&nfe.emit.cnpj_cpf),
            'B',
            'L',
            0.0,
        );
    }
}
