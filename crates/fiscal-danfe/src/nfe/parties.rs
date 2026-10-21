//! Recipient and pick-up/delivery location bands.

use std::sync::Arc;

use fiscal_pdf::report::{BandHandler, BandState, DrawArgs, InitArgs};
use fiscal_pdf::{Orientation, round_to};

use crate::context::DanfeContext;
use crate::format;
use crate::model::Local;

/// "DESTINATÁRIO / REMETENTE".
pub(super) struct Destinatario {
    pub(super) ctx: Arc<DanfeContext>,
}

impl BandHandler for Destinatario {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.height = 25.0;
    }

    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let width = band.width;
        let nfe = &self.ctx.nfe;
        let pdf = &mut args.pdf;
        let header = |pdf: &mut fiscal_pdf::report::ReportPdf<'_>| pdf.set_font_size_style(6.0, "");
        let content =
            |pdf: &mut fiscal_pdf::report::ReportPdf<'_>| pdf.set_font_size_style(10.0, "");
        let bold = |pdf: &mut fiscal_pdf::report::ReportPdf<'_>| pdf.set_font_size_style(10.0, "B");
        let portrait = pdf.orientation() == Orientation::Portrait;
        let dest = &nfe.dest;
        let ender = &dest.ender_dest;

        let mut x = 0.0;
        let mut y = 1.0;
        let old_x = x;

        let mut w = width;
        let h = 7.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, w, h, "DESTINATÁRIO / REMETENTE", 'T', 'L', 0.0);
        // NOME / RAZÃO SOCIAL
        w = round_to(width * 0.61, 0);
        let mut w1 = w;
        y += 3.0;
        header(pdf);
        pdf.text_box(x, y, w, h, "NOME / RAZÃO SOCIAL", 'T', 'L', 1.0);
        content(pdf);
        pdf.text_box(
            x,
            y,
            w,
            h,
            &dest.x_nome,
            'B',
            'L',
            if portrait { 0.0 } else { 1.0 },
        );
        // CNPJ / CPF
        x += w;
        w = round_to(width * 0.23, 0);
        let mut w2 = w;
        header(pdf);
        pdf.text_box(x, y, w, h, "CNPJ / CPF", 'T', 'L', 1.0);
        bold(pdf);
        pdf.text_box(
            x,
            y,
            w,
            h,
            &format::cnpj_ou_cpf(&dest.cnpj_cpf),
            'B',
            'L',
            0.0,
        );
        // DATA DA EMISSÃO
        x += w;
        w = width - (w1 + w2);
        let wx = w;
        header(pdf);
        pdf.text_box(x, y, w, h, "DATA DA EMISSÃO", 'T', 'L', 1.0);
        content(pdf);
        let emissao = nfe.ide.d_emi.map(|d| d.date_br()).unwrap_or_default();
        pdf.text_box(
            x,
            y,
            w,
            h,
            &emissao,
            'B',
            'C',
            if portrait { 0.0 } else { 1.0 },
        );

        // ENDEREÇO
        w = round_to(width * 0.47, 0);
        w1 = w;
        y += h;
        x = old_x;
        header(pdf);
        pdf.text_box(x, y, w, h, "ENDEREÇO", 'T', 'L', 1.0);
        let mut texto = format!("{}, {}", ender.x_lgr, ender.nro);
        if !ender.x_cpl.is_empty() {
            texto.push_str(&format!(" - {}", ender.x_cpl));
        }
        content(pdf);
        pdf.text_box_force(x, y, w, h, &texto, 'B', 'L', 0.0, true);
        // BAIRRO / DISTRITO
        x += w;
        w = round_to(width * 0.21, 0);
        w2 = w;
        header(pdf);
        pdf.text_box(x, y, w, h, "BAIRRO / DISTRITO", 'T', 'L', 1.0);
        content(pdf);
        pdf.text_box(x, y, w, h, &ender.x_bairro, 'B', 'L', 0.0);
        // CEP
        x += w;
        w = width - w1 - w2 - wx;
        header(pdf);
        pdf.text_box(x, y, w, h, "CEP", 'T', 'L', 1.0);
        let cep = if ender.cep > 0 {
            format::cep_number(ender.cep)
        } else {
            String::new()
        };
        content(pdf);
        pdf.text_box(x, y, w, h, &cep, 'B', 'L', 0.0);
        // DATA DA SAÍDA
        x += w;
        w = wx;
        header(pdf);
        pdf.text_box(x, y, w, h, "DATA DA SAÍDA/ENTRADA", 'T', 'L', 1.0);
        let saida = nfe.ide.d_sai_ent.map(|d| d.date_br()).unwrap_or_default();
        bold(pdf);
        pdf.text_box(x, y, w, h, &saida, 'B', 'C', 0.0);
        // MUNICÍPIO
        w = w1;
        y += h;
        x = old_x;
        header(pdf);
        pdf.text_box(x, y, w, h, "MUNICÍPIO", 'T', 'L', 1.0);
        let mut texto = ender.x_mun.clone();
        if texto.trim().to_uppercase() == "EXTERIOR" && !ender.x_pais.is_empty() {
            texto.push_str(&format!(" - {}", ender.x_pais));
        }
        content(pdf);
        pdf.text_box(x, y, w, h, &texto, 'B', 'L', 0.0);
        // UF
        x += w;
        w = 8.0;
        header(pdf);
        pdf.text_box(x, y, w, h, "UF", 'T', 'L', 1.0);
        content(pdf);
        pdf.text_box(x, y, w, h, &ender.uf, 'B', 'C', 0.0);
        // FONE / FAX
        x += w;
        w = round_to((width - w1 - wx - 8.0) / 2.0, 0);
        let w3 = w;
        header(pdf);
        pdf.text_box(x, y, w, h, "FONE / FAX", 'T', 'L', 1.0);
        content(pdf);
        pdf.text_box(x, y, w, h, &ender.fone, 'B', 'C', 0.0);
        // INSCRIÇÃO ESTADUAL
        x += w;
        w = width - w1 - wx - 8.0 - w3;
        header(pdf);
        pdf.text_box(x, y, w, h, "INSCRIÇÃO ESTADUAL", 'T', 'L', 1.0);
        content(pdf);
        pdf.text_box(x, y, w, h, &dest.ie, 'B', 'C', 0.0);
        // HORA DA SAÍDA
        x += w;
        w = wx;
        header(pdf);
        pdf.text_box(x, y, w, h, "HORA DA SAÍDA/ENTRADA", 'T', 'L', 1.0);
        let hora = nfe
            .ide
            .d_sai_ent
            .filter(|d| !d.time_is_zero())
            .map(|d| d.time_string())
            .unwrap_or_default();
        bold(pdf);
        pdf.text_box(x, y, w, h, &hora, 'B', 'C', 0.0);
    }
}

/// Which location a [`Local`] band prints.
#[derive(Clone, Copy)]
pub(super) enum LocalKind {
    Retirada,
    Entrega,
}

/// "INFORMAÇÕES DO LOCAL DE RETIRADA/ENTREGA".
pub(super) struct LocalBand {
    pub(super) ctx: Arc<DanfeContext>,
    pub(super) kind: LocalKind,
}

impl LocalBand {
    fn local(&self) -> &Local {
        match self.kind {
            LocalKind::Retirada => &self.ctx.nfe.retirada,
            LocalKind::Entrega => &self.ctx.nfe.entrega,
        }
    }
}

impl BandHandler for LocalBand {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.height = 25.0;
        band.visible = !self.local().cnpj_cpf.is_empty();
    }

    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let width = band.width;
        let l = self.local();
        if l.cnpj_cpf.is_empty() {
            return;
        }
        let titulo = match self.kind {
            LocalKind::Retirada => "INFORMAÇÕES DO LOCAL DE RETIRADA",
            LocalKind::Entrega => "INFORMAÇÕES DO LOCAL DE ENTREGA",
        };
        let pdf = &mut args.pdf;
        let portrait = pdf.orientation() == Orientation::Portrait;

        let mut x = 0.0;
        let mut y = 1.0;
        let old_x = x;

        let mut w = width;
        let h = 7.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, w, h, titulo, 'T', 'L', 0.0);
        // NOME / RAZÃO SOCIAL
        w = round_to(width * 0.61, 0);
        let mut w1 = w;
        y += 3.0;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "NOME / RAZÃO SOCIAL", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(
            x,
            y,
            w,
            h,
            &l.x_nome,
            'B',
            'L',
            if portrait { 0.0 } else { 1.0 },
        );
        // CNPJ / CPF
        x += w;
        w = round_to(width * 0.23, 0);
        let mut w2 = w;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "CNPJ / CPF", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &format::cnpj_ou_cpf(&l.cnpj_cpf), 'B', 'L', 0.0);
        // INSCRIÇÃO ESTADUAL
        x += w;
        w = width - (w1 + w2);
        let wx = w;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "INSCRIÇÃO ESTADUAL", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(
            x,
            y,
            w,
            h,
            &l.ie,
            'B',
            'C',
            if portrait { 0.0 } else { 1.0 },
        );
        // ENDEREÇO
        w = round_to(width * 0.355, 0) + wx;
        w1 = w;
        y += h;
        x = old_x;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "ENDEREÇO", 'T', 'L', 1.0);
        let texto = format!(
            "{}{}{}",
            l.x_lgr,
            if l.nro.is_empty() {
                String::new()
            } else {
                format!(", {}", l.nro)
            },
            if l.x_cpl.is_empty() {
                String::new()
            } else {
                format!(" - {}", l.x_cpl)
            }
        );
        pdf.set_font_size_style(10.0, "");
        pdf.text_box_force(x, y, w, h, &texto, 'B', 'L', 0.0, true);
        // BAIRRO / DISTRITO
        x += w;
        w = round_to(width * 0.335, 0);
        w2 = w;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "BAIRRO / DISTRITO", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &l.x_bairro, 'B', 'L', 0.0);
        // CEP
        x += w;
        w = width - (w1 + w2);
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "CEP", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &format::cep_number(l.cep), 'B', 'L', 0.0);
        // MUNICÍPIO
        w = round_to(width * 0.805, 0);
        w1 = w;
        y += h;
        x = old_x;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "MUNICÍPIO", 'T', 'L', 1.0);
        let mut texto = l.x_mun.clone();
        if texto.trim().to_uppercase() == "EXTERIOR" && !l.x_pais.is_empty() {
            texto.push_str(&format!(" - {}", l.x_pais));
        }
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &texto, 'B', 'L', 0.0);
        // UF
        x += w;
        w = 8.0;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "UF", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &l.uf, 'B', 'C', 0.0);
        // FONE / FAX
        x += w;
        w = width - w - w1;
        pdf.set_font_size_style(6.0, "");
        pdf.text_box(x, y, w, h, "FONE / FAX", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(x, y, w, h, &l.fone, 'B', 'C', 0.0);
    }
}
