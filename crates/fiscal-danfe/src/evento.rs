//! DANFE of an NF-e event (correction letter, cancellation...): the model of the event, its
//! reader and the layout (`TNFeDANFeEventoFPDF` of the original).
//!
//! The event is printed together with the NF-e it refers to: the issuer, recipient and
//! identification come from the invoice, the rest from the `procEventoNFe`.

use std::sync::Arc;

use fiscal_pdf::report::{
    Band, BandHandler, BandState, BandType, DrawArgs, Engine, InitArgs, Report,
};
use fiscal_pdf::{Color, Orientation, PageFormat, Unit, round_to};

use crate::error::DanfeError;
use crate::format;
use crate::model::{DateTime, Nfe, TpAmb};
use crate::options::DanfeOptions;
use crate::xml::{self, Node};

/// Event code of the correction letter (CC-e).
pub const TP_EVENTO_CCE: &str = "110110";
/// Event code of the cancellation.
pub const TP_EVENTO_CANCELAMENTO: &str = "110111";

const FONT_FAMILY: &str = "Times";

/// `detEvento`: the part of the event that depends on its type.
#[derive(Debug, Clone, Default)]
pub struct DetEvento {
    /// Description of the event (`descEvento`).
    pub desc_evento: String,
    /// Correction text of the CC-e (`xCorrecao`).
    pub x_correcao: String,
    /// Conditions of use of the CC-e (`xCondUso`).
    pub x_cond_uso: String,
    /// Justification of the cancellation (`xJust`).
    pub x_just: String,
    /// Protocol of the invoice the event refers to (`nProt`).
    pub n_prot: String,
}

/// `evento/infEvento`: what was requested.
#[derive(Debug, Clone, Default)]
pub struct InfEvento {
    /// IBGE code of the organ that receives the event (`cOrgao`).
    pub c_orgao: u32,
    /// Environment.
    pub tp_amb: TpAmb,
    /// CNPJ or CPF of the author.
    pub cnpj_cpf: String,
    /// Access key of the invoice.
    pub ch_nfe: String,
    /// Date and time of the event.
    pub dh_evento: Option<DateTime>,
    /// Event code (`tpEvento`).
    pub tp_evento: String,
    /// Sequence number of the event.
    pub n_seq_evento: u32,
    /// Version of the event (`verEvento`).
    pub versao_evento: String,
    /// Details.
    pub det: DetEvento,
}

/// `retEvento/infEvento`: the answer of the tax authority.
#[derive(Debug, Clone, Default)]
pub struct RetInfEvento {
    /// Status code.
    pub c_stat: u32,
    /// Status description.
    pub x_motivo: String,
    /// Protocol that registered the event.
    pub n_prot: String,
    /// Date and time of the registration.
    pub dh_reg_evento: Option<DateTime>,
}

/// An event with the answer of the tax authority, if any.
#[derive(Debug, Clone, Default)]
pub struct Evento {
    /// The request.
    pub inf: InfEvento,
    /// The answer.
    pub ret: RetInfEvento,
}

fn text(node: &Node, name: &str) -> String {
    node.text_of(name).to_owned()
}

fn read_inf_evento(inf: &Node) -> InfEvento {
    let det = inf.child("detEvento");
    let det_text = |name: &str| det.map(|d| text(d, name)).unwrap_or_default();
    let cnpj = inf.text_of("CNPJ");
    InfEvento {
        c_orgao: inf.text_of("cOrgao").parse().unwrap_or_default(),
        tp_amb: if inf.text_of("tpAmb") == "2" {
            TpAmb::Homologacao
        } else {
            TpAmb::Producao
        },
        cnpj_cpf: if cnpj.is_empty() {
            text(inf, "CPF")
        } else {
            cnpj.to_owned()
        },
        ch_nfe: text(inf, "chNFe"),
        dh_evento: DateTime::parse(inf.text_of("dhEvento")),
        tp_evento: text(inf, "tpEvento"),
        n_seq_evento: inf.text_of("nSeqEvento").parse().unwrap_or_default(),
        versao_evento: text(inf, "verEvento"),
        det: DetEvento {
            desc_evento: det_text("descEvento"),
            x_correcao: det_text("xCorrecao"),
            x_cond_uso: det_text("xCondUso"),
            x_just: det_text("xJust"),
            n_prot: det_text("nProt"),
        },
    }
}

fn read_ret_evento(inf: &Node) -> RetInfEvento {
    RetInfEvento {
        c_stat: inf.text_of("cStat").parse().unwrap_or_default(),
        x_motivo: text(inf, "xMotivo"),
        n_prot: text(inf, "nProt"),
        dh_reg_evento: DateTime::parse(inf.text_of("dhRegEvento")),
    }
}

impl Evento {
    /// Reads a `procEventoNFe` (event with the answer) or a bare `evento` / `envEvento`.
    pub fn from_xml(xml: &str) -> Result<Self, DanfeError> {
        let root = xml::parse(xml.trim_start_matches('\u{feff}'))?;
        let (evento, ret) = match root.name.as_str() {
            "procEventoNFe" => (root.child("evento"), root.child("retEvento")),
            "evento" => (Some(&root), None),
            "envEvento" => (root.child("evento"), None),
            other => {
                return Err(DanfeError::InvalidNfe(format!(
                    "unexpected root element <{other}>, expected <procEventoNFe> or <evento>"
                )));
            }
        };
        let inf = evento
            .and_then(|e| e.child("infEvento"))
            .ok_or_else(|| DanfeError::InvalidNfe("missing <infEvento>".to_owned()))?;
        Ok(Self {
            inf: read_inf_evento(inf),
            ret: ret
                .and_then(|r| r.child("infEvento"))
                .map(read_ret_evento)
                .unwrap_or_default(),
        })
    }

    /// Reads an event from raw bytes: UTF-8 or, failing that, Windows-1252.
    pub fn from_xml_bytes(bytes: &[u8]) -> Result<Self, DanfeError> {
        match std::str::from_utf8(bytes) {
            Ok(s) => Self::from_xml(s),
            Err(_) => {
                let decoded: String = bytes
                    .iter()
                    .map(|b| crate::reader::cp1252_to_char(*b))
                    .collect();
                Self::from_xml(&decoded)
            }
        }
    }

    /// The event is a correction letter.
    fn is_cce(&self) -> bool {
        self.inf.tp_evento == TP_EVENTO_CCE
    }

    /// The event is a cancellation.
    fn is_cancelamento(&self) -> bool {
        self.inf.tp_evento == TP_EVENTO_CANCELAMENTO
    }
}

/// What the bands read.
struct Context {
    nfe: Nfe,
    evento: Evento,
    mensagem_rodape: String,
    quebra_de_linha: String,
}

/// One block of the event DANFE.
#[derive(Clone, Copy)]
enum Block {
    Cabecalho,
    DadosNota,
    DadosEvento,
    Emitente,
    Destinatario,
    CartaCorrecao,
    CorrecaoOuJustificativa,
    Mensagem,
    Rodape,
}

struct EventoBand {
    ctx: Arc<Context>,
    block: Block,
    height: f64,
}

impl EventoBand {
    fn band(ctx: &Arc<Context>, band_type: BandType, height: f64, block: Block) -> Band {
        Band::new(
            band_type,
            Self {
                ctx: Arc::clone(ctx),
                block,
                height,
            },
        )
    }
}

fn font_header(pdf: &mut fiscal_pdf::report::ReportPdf<'_>) {
    pdf.set_font_size_style(6.0, "");
}

fn font_content(pdf: &mut fiscal_pdf::report::ReportPdf<'_>) {
    pdf.set_font_size_style(10.0, "");
}

fn font_content_bold(pdf: &mut fiscal_pdf::report::ReportPdf<'_>) {
    pdf.set_font_size_style(10.0, "B");
}

type Pdf<'a, 'b> = &'a mut fiscal_pdf::report::ReportPdf<'b>;

/// A titled rectangle with free text (`BlocoTexto`).
fn bloco_texto(pdf: Pdf<'_, '_>, band: &BandState, titulo: &str, conteudo: &str, quebra: &str) {
    let (x, mut y) = (0.0, 1.0);
    let max_w = band.width;
    pdf.set_font_size_style(7.0, "B");
    pdf.text_box(x, y, max_w, 5.0, titulo, 'T', 'L', 0.0);
    y += 3.0;
    pdf.rect(x, y, max_w, band.height - y, "D");
    let texto = if quebra.is_empty() {
        conteudo.to_owned()
    } else {
        conteudo.replace(quebra, "\n")
    };
    font_content(pdf);
    pdf.text_box_force(
        x + 2.0,
        y + 2.0,
        max_w - 4.0,
        band.height - y,
        &texto,
        'T',
        'L',
        0.0,
        false,
    );
}

/// `CodigoUFparaUF` of the original: its table sends the national environment (90, 91) to DF.
fn orgao_uf(code: u32) -> &'static str {
    match code {
        90 | 91 => "DF",
        other => format::codigo_uf_para_uf(other),
    }
}

impl EventoBand {
    fn cabecalho(&self, pdf: Pdf<'_, '_>, band: &BandState) {
        let (x, mut y) = (0.0, 0.0);
        let w = band.width;
        pdf.text_box_frame(x, y, w, 18.0);
        pdf.set_font_size_style(12.0, "B");
        y += 2.0;
        y += pdf.text_box(
            x,
            y,
            w,
            5.0,
            &self.ctx.evento.inf.det.desc_evento,
            'T',
            'C',
            0.0,
        );
        y += 1.0;
        pdf.set_font_size_style(10.0, "");
        y += pdf.text_box(
            x,
            y,
            w,
            5.0,
            "Não possui valor fiscal, simples representação do fato indicado abaixo.",
            'T',
            'C',
            0.0,
        );
        y += 1.0;
        pdf.set_font_size_style(10.0, "");
        pdf.text_box(
            x,
            y,
            w,
            5.0,
            "CONSULTE A AUTENTICIDADE NO SITE DA SEFAZ AUTORIZADORA.",
            'T',
            'C',
            0.0,
        );
    }

    fn dados_nota(&self, pdf: Pdf<'_, '_>, band: &BandState) {
        let nfe = &self.ctx.nfe;
        let old_x = 0.0;
        let (mut x, mut y) = (0.0, 1.0);
        let max_w = band.width;
        let h = 8.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, max_w, h, "NOTA FISCAL ELETRÔNICA", 'T', 'L', 0.0);
        y += 3.0;

        let w1 = round_to(max_w * 0.10, 0);
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "MODELO", 'T', 'C', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w1, h, &nfe.ide.modelo.to_string(), 'B', 'C', 0.0);

        x += w1;
        let w2 = round_to(max_w * 0.10, 0);
        font_header(pdf);
        pdf.text_box(x, y, w2, h, "SÉRIE", 'T', 'C', 1.0);
        font_content(pdf);
        pdf.text_box(
            x,
            y,
            w2,
            h,
            &format::zero_pad(u64::from(nfe.ide.serie), 3),
            'B',
            'C',
            0.0,
        );

        x += w2;
        let w3 = round_to(max_w * 0.15, 0);
        font_header(pdf);
        pdf.text_box(x, y, w3, h, "NÚMERO", 'T', 'C', 1.0);
        font_content(pdf);
        pdf.text_box(
            x,
            y,
            w3,
            h,
            &format::zero_pad(nfe.ide.n_nf, 9),
            'B',
            'C',
            0.0,
        );

        x += w3;
        let w4 = round_to(max_w * 0.15, 0);
        font_header(pdf);
        pdf.text_box(x, y, w4, h, "DATA/HORA DE EMISSÃO", 'T', 'L', 1.0);
        font_content(pdf);
        let emissao = nfe.ide.d_emi.map(|d| d.datetime_br()).unwrap_or_default();
        pdf.text_box(x, y, w4, h, &emissao, 'B', 'C', 0.0);

        // Bar code column.
        x += w4;
        let w = max_w - w1 - w2 - w3 - w4;
        pdf.text_box_frame(x, y, w, 2.0 * h);
        pdf.set_fill_color(Color::Rgb(0, 0, 0));
        let (b_w, b_h) = (75.0, 12.0);
        pdf.code128(&nfe.chave(), x + (w - b_w) / 2.0, y + 2.0, b_h, b_w);

        x = old_x;
        y += h;
        let w = w1 + w2 + w3 + w4;
        font_header(pdf);
        pdf.text_box(x, y, w, h, "CHAVE DE ACESSO", 'T', 'L', 1.0);
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(
            x,
            y,
            w,
            h,
            &format::chave_acesso(&nfe.chave()),
            'B',
            'C',
            0.0,
        );
    }

    fn dados_evento(&self, pdf: Pdf<'_, '_>, band: &BandState) {
        let inf = &self.ctx.evento.inf;
        let ret = &self.ctx.evento.ret;
        let old_x = 0.0;
        let (mut x, mut y) = (0.0, 1.0);
        let max_w = band.width;
        let h = 7.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, max_w, h, "DADOS DO EVENTO", 'T', 'L', 0.0);
        y += 3.0;

        // ÓRGÃO | AMBIENTE | DATA/HORA DO EVENTO
        let w1 = round_to(max_w * 0.05, 0);
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "ÓRGÃO", 'T', 'C', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w1, h, orgao_uf(inf.c_orgao), 'B', 'C', 0.0);

        x += w1;
        let w2 = max_w - round_to(max_w * 0.25, 0);
        font_header(pdf);
        pdf.text_box(x, y, w2, h, "AMBIENTE", 'T', 'L', 1.0);
        let ambiente = if inf.tp_amb == TpAmb::Producao {
            "1 - PRODUÇÃO"
        } else {
            "2 - HOMOLOGAÇÃO (SEM VALOR FISCAL)"
        };
        font_content(pdf);
        pdf.text_box(x, y, w2, h, ambiente, 'B', 'L', 0.0);

        x += w2;
        let w3 = max_w - w1 - w2;
        font_header(pdf);
        pdf.text_box(x, y, w3, h, "DATA/HORA DO EVENTO", 'T', 'L', 1.0);
        font_content(pdf);
        let data = inf.dh_evento.map(|d| d.datetime_br()).unwrap_or_default();
        pdf.text_box(x, y, w3, h, &data, 'B', 'C', 0.0);

        // EVENTO | DESCRIÇÃO | SEQUENCIAL | VERSÃO
        x = old_x;
        y += h;
        let w1 = round_to(max_w * 0.10, 0);
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "EVENTO", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w1, h, &inf.tp_evento, 'B', 'C', 0.0);

        x += w1;
        let w2 = round_to(max_w * 0.60, 0);
        font_header(pdf);
        pdf.text_box(x, y, w2, h, "DESCRIÇÃO DO EVENTO", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w2, h, &inf.det.desc_evento, 'B', 'L', 0.0);

        x += w2;
        let w3 = (max_w - w1 - w2) / 2.0;
        font_header(pdf);
        pdf.text_box(x, y, w3, h, "SEQUENCIAL DO EVENTO", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w3, h, &inf.n_seq_evento.to_string(), 'B', 'C', 0.0);

        x += w3;
        font_header(pdf);
        pdf.text_box(x, y, w3, h, "VERSÃO DO EVENTO", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w3, h, &inf.versao_evento, 'B', 'C', 0.0);

        // STATUS | PROTOCOLO | DATA/HORA DO REGISTRO
        x = old_x;
        y += h;
        let w1 = round_to(max_w * 0.50, 0);
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "STATUS", 'T', 'L', 1.0);
        font_content(pdf);
        let status = format!("{} - {}", ret.c_stat, ret.x_motivo);
        pdf.text_box(x, y, w1, h, &status, 'B', 'L', 0.0);

        x += w1;
        let w2 = (max_w - w1) / 2.0;
        font_header(pdf);
        pdf.text_box(
            x,
            y,
            w2,
            h,
            "PROTOCOLO DE REGISTRO DO EVENTO",
            'T',
            'L',
            1.0,
        );
        font_content_bold(pdf);
        pdf.text_box(x, y, w2, h, &ret.n_prot, 'B', 'C', 0.0);

        x += w2;
        font_header(pdf);
        pdf.text_box(
            x,
            y,
            w2,
            h,
            "DATA/HORA DO REGISTRO DO EVENTO",
            'T',
            'L',
            1.0,
        );
        font_content(pdf);
        let registro = ret
            .dh_reg_evento
            .map(|d| d.datetime_br())
            .unwrap_or_default();
        pdf.text_box(x, y, w2, h, &registro, 'B', 'C', 0.0);
    }

    fn emitente(&self, pdf: Pdf<'_, '_>, band: &BandState) {
        if pdf.current_page() > 1 {
            return;
        }
        let nfe = &self.ctx.nfe;
        let emit = &nfe.emit;
        let ender = &emit.ender_emit;
        let old_x = 0.0;
        let (mut x, mut y) = (0.0, 1.0);
        let max_w = band.width;
        let h = 7.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, max_w, h, "EMITENTE", 'T', 'L', 0.0);

        // NOME / RAZÃO SOCIAL | CNPJ / CPF | DATA DA EMISSÃO
        let w1 = round_to(max_w * 0.61, 0);
        y += 3.0;
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "NOME / RAZÃO SOCIAL", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w1, h, &emit.x_nome, 'B', 'L', 0.0);

        x += w1;
        let w2 = round_to(max_w * 0.23, 0);
        font_header(pdf);
        pdf.text_box(x, y, w2, h, "CNPJ / CPF", 'T', 'L', 1.0);
        font_content_bold(pdf);
        pdf.text_box(
            x,
            y,
            w2,
            h,
            &format::cnpj_ou_cpf(&emit.cnpj_cpf),
            'B',
            'L',
            0.0,
        );

        x += w2;
        let wx = max_w - (w1 + w2);
        font_header(pdf);
        pdf.text_box(x, y, wx, h, "DATA DA EMISSÃO", 'T', 'L', 1.0);
        font_content(pdf);
        let emissao = nfe.ide.d_emi.map(|d| d.date_br()).unwrap_or_default();
        pdf.text_box(x, y, wx, h, &emissao, 'B', 'C', 0.0);

        // ENDEREÇO | BAIRRO / DISTRITO | CEP | DATA DA SAÍDA/ENTRADA
        let w1 = round_to(max_w * 0.47, 0);
        y += h;
        x = old_x;
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "ENDEREÇO", 'T', 'L', 1.0);
        let mut texto = format!("{}, {}", ender.x_lgr, ender.nro);
        if !ender.x_cpl.is_empty() {
            texto.push_str(" - ");
            texto.push_str(&ender.x_cpl);
        }
        font_content(pdf);
        pdf.text_box_force(x, y, w1, h, &texto, 'B', 'L', 0.0, true);

        x += w1;
        let w2 = round_to(max_w * 0.21, 0);
        font_header(pdf);
        pdf.text_box(x, y, w2, h, "BAIRRO / DISTRITO", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w2, h, &ender.x_bairro, 'B', 'L', 0.0);

        x += w2;
        let w = max_w - w1 - w2 - wx;
        font_header(pdf);
        pdf.text_box(x, y, w, h, "CEP", 'T', 'L', 1.0);
        font_content(pdf);
        let cep = if ender.cep == 0 {
            String::new()
        } else {
            format::cep_number(ender.cep)
        };
        pdf.text_box(x, y, w, h, &cep, 'B', 'L', 0.0);

        x += w;
        font_header(pdf);
        pdf.text_box(x, y, wx, h, "DATA DA SAÍDA/ENTRADA", 'T', 'L', 1.0);
        let saida = nfe.ide.d_sai_ent.map(|d| d.date_br()).unwrap_or_default();
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(x, y, wx, h, &saida, 'B', 'C', 0.0);

        // MUNICÍPIO | UF | FONE / FAX | INSCRIÇÃO ESTADUAL | HORA DA SAÍDA/ENTRADA
        y += h;
        x = old_x;
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "MUNICÍPIO", 'T', 'L', 1.0);
        let mut municipio = ender.x_mun.clone();
        if municipio.trim().to_uppercase() == "EXTERIOR" && !ender.x_pais.is_empty() {
            municipio.push_str(" - ");
            municipio.push_str(&ender.x_pais);
        }
        font_content(pdf);
        pdf.text_box(x, y, w1, h, &municipio, 'B', 'L', 0.0);

        x += w1;
        font_header(pdf);
        pdf.text_box(x, y, 8.0, h, "UF", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, 8.0, h, &ender.uf, 'B', 'C', 0.0);

        x += 8.0;
        let w3 = round_to((max_w - w1 - wx - 8.0) / 2.0, 0);
        font_header(pdf);
        pdf.text_box(x, y, w3, h, "FONE / FAX", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w3, h, &ender.fone, 'B', 'C', 0.0);

        x += w3;
        let w = max_w - w1 - wx - 8.0 - w3;
        font_header(pdf);
        pdf.text_box(x, y, w, h, "INSCRIÇÃO ESTADUAL", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w, h, &emit.ie, 'B', 'C', 0.0);

        x += w;
        font_header(pdf);
        pdf.text_box(x, y, wx, h, "HORA DA SAÍDA/ENTRADA", 'T', 'L', 1.0);
        let hora = match nfe.ide.d_sai_ent {
            Some(d) if !d.time_is_zero() => d.time_string(),
            _ => String::new(),
        };
        pdf.set_font_size_style(10.0, "B");
        pdf.text_box(x, y, wx, h, &hora, 'B', 'C', 0.0);
    }

    fn destinatario(&self, pdf: Pdf<'_, '_>, band: &BandState) {
        let nfe = &self.ctx.nfe;
        let dest = &nfe.dest;
        let ender = &dest.ender_dest;
        let old_x = 0.0;
        let (mut x, mut y) = (0.0, 1.0);
        let max_w = band.width;
        let h = 7.0;
        pdf.set_font_size_style(7.0, "B");
        pdf.text_box(x, y, max_w, h, "DESTINATÁRIO / REMETENTE", 'T', 'L', 0.0);

        // NOME / RAZÃO SOCIAL | CNPJ / CPF
        let w1 = max_w - round_to(max_w * 0.23, 0);
        y += 3.0;
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "NOME / RAZÃO SOCIAL", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w1, h, &dest.x_nome, 'B', 'L', 0.0);

        x += w1;
        let w = max_w - w1;
        font_header(pdf);
        pdf.text_box(x, y, w, h, "CNPJ / CPF", 'T', 'L', 1.0);
        font_content_bold(pdf);
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

        // ENDEREÇO | BAIRRO / DISTRITO | CEP
        let w1 = round_to(max_w * 0.50, 0);
        y += h;
        x = old_x;
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "ENDEREÇO", 'T', 'L', 1.0);
        let mut texto = format!("{}, {}", ender.x_lgr, ender.nro);
        if !ender.x_cpl.is_empty() {
            texto.push_str(" - ");
            texto.push_str(&ender.x_cpl);
        }
        font_content(pdf);
        pdf.text_box_force(x, y, w1, h, &texto, 'B', 'L', 0.0, true);

        x += w1;
        let w2 = round_to(max_w * 0.25, 0);
        font_header(pdf);
        pdf.text_box(x, y, w2, h, "BAIRRO / DISTRITO", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w2, h, &ender.x_bairro, 'B', 'L', 0.0);

        x += w2;
        let w = max_w - w1 - w2;
        let w2 = w;
        font_header(pdf);
        pdf.text_box(x, y, w, h, "CEP", 'T', 'L', 1.0);
        font_content(pdf);
        let cep = if ender.cep == 0 {
            String::new()
        } else {
            format::cep_number(ender.cep)
        };
        pdf.text_box(x, y, w, h, &cep, 'B', 'L', 0.0);

        // MUNICÍPIO | UF | FONE / FAX | INSCRIÇÃO ESTADUAL
        y += h;
        x = old_x;
        font_header(pdf);
        pdf.text_box(x, y, w1, h, "MUNICÍPIO", 'T', 'L', 1.0);
        let mut municipio = ender.x_mun.clone();
        if municipio.trim().to_uppercase() == "EXTERIOR" && !ender.x_pais.is_empty() {
            municipio.push_str(" - ");
            municipio.push_str(&ender.x_pais);
        }
        font_content(pdf);
        pdf.text_box(x, y, w1, h, &municipio, 'B', 'L', 0.0);

        x += w1;
        font_header(pdf);
        pdf.text_box(x, y, 8.0, h, "UF", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, 8.0, h, &ender.uf, 'B', 'C', 0.0);

        x += 8.0;
        let w = max_w - w1 - w2 - 8.0;
        font_header(pdf);
        pdf.text_box(x, y, w, h, "FONE / FAX", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w, h, &ender.fone, 'B', 'C', 0.0);

        x += w;
        font_header(pdf);
        pdf.text_box(x, y, w2, h, "INSCRIÇÃO ESTADUAL", 'T', 'L', 1.0);
        font_content(pdf);
        pdf.text_box(x, y, w2, h, &dest.ie, 'B', 'C', 0.0);
    }

    fn carta_correcao(&self, pdf: Pdf<'_, '_>, band: &BandState) {
        let quebra = self.ctx.quebra_de_linha.as_str();
        let mut condicoes = self
            .ctx
            .evento
            .inf
            .det
            .x_cond_uso
            .replace("com: I", "com:\nI");
        if !quebra.is_empty() {
            condicoes = condicoes.replace(quebra, &format!("{quebra}\n"));
        }
        bloco_texto(pdf, band, "CONDIÇÕES DE USO", &condicoes, quebra);
    }

    fn correcao_ou_justificativa(&self, pdf: Pdf<'_, '_>, band: &mut BandState, args: (f64, f64)) {
        let evento = &self.ctx.evento;
        let (titulo, conteudo) = if evento.is_cce() {
            (
                "CORREÇÕES A SEREM CONSIDERADAS",
                evento.inf.det.x_correcao.as_str(),
            )
        } else if evento.is_cancelamento() {
            (
                "JUSTIFICATIVA DO CANCELAMENTO",
                evento.inf.det.x_just.as_str(),
            )
        } else {
            (evento.inf.det.desc_evento.as_str(), "")
        };
        // The box takes what is left of the page.
        band.height = args.0 - args.1;
        bloco_texto(pdf, band, titulo, conteudo, &self.ctx.quebra_de_linha);
    }

    fn mensagem(&self, pdf: Pdf<'_, '_>, band: &BandState) {
        let evento = &self.ctx.evento;
        let texto = if evento.is_cce() {
            "Este documento é uma representação gráfica da CC-e e foi impresso apenas para sua \
             informação e não possui validade fiscal.\n A CC-e deve ser recebida e mantida em \
             arquivo eletrônico XML e pode ser consultada através dos Portais das SEFAZ."
        } else if evento.is_cancelamento() {
            "Este documento é uma representação gráfica do evento de NFe e foi impresso apenas \
             para sua informação e não possui validade fiscal.\n O Evento deve ser recebido e \
             mantido em arquivo eletrônico XML e pode ser consultada através dos Portais das \
             SEFAZ."
        } else {
            ""
        };
        pdf.set_font_size_style(10.0, "I");
        pdf.text_box_flags(
            0.0,
            0.0,
            band.width,
            band.height,
            texto,
            'C',
            'C',
            true,
            false,
            false,
        );
    }

    fn rodape(&self, pdf: Pdf<'_, '_>, band: &BandState) {
        let mensagem = &self.ctx.mensagem_rodape;
        if mensagem.is_empty() {
            return;
        }
        let parts: Vec<&str> = mensagem.split('|').collect();
        let (x, y, w, h) = (0.0, 0.0, band.width, band.height);
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

impl BandHandler for EventoBand {
    fn on_init(&mut self, band: &mut BandState, _args: &InitArgs) {
        band.height = self.height;
    }

    fn on_draw(&mut self, band: &mut BandState, args: &mut DrawArgs<'_>) {
        let space = (args.free_space, args.reserved_space);
        let pdf = &mut args.pdf;
        match self.block {
            Block::Cabecalho => self.cabecalho(pdf, band),
            Block::DadosNota => self.dados_nota(pdf, band),
            Block::DadosEvento => self.dados_evento(pdf, band),
            Block::Emitente => self.emitente(pdf, band),
            Block::Destinatario => self.destinatario(pdf, band),
            Block::CartaCorrecao => self.carta_correcao(pdf, band),
            Block::CorrecaoOuJustificativa => self.correcao_ou_justificativa(pdf, band, space),
            Block::Mensagem => self.mensagem(pdf, band),
            Block::Rodape => self.rodape(pdf, band),
        }
    }
}

/// Adds the page and the bands of the event DANFE to `report` (`OnStartReport`).
fn build_report(report: &mut Report, ctx: &Arc<Context>) {
    report.set_margins(4.0, 4.0, None, None);
    report.add_page(Orientation::Portrait, Unit::Mm, PageFormat::A4);

    let mut blocks = vec![
        (BandType::Data, 18.0, Block::Cabecalho),
        (BandType::Data, 20.0, Block::DadosNota),
        (BandType::Data, 25.0, Block::DadosEvento),
        (BandType::Data, 25.0, Block::Emitente),
        (BandType::Data, 25.0, Block::Destinatario),
    ];
    if ctx.evento.is_cce() {
        blocks.push((BandType::Data, 30.0, Block::CartaCorrecao));
    }
    if ctx.evento.is_cce() || ctx.evento.is_cancelamento() {
        blocks.push((BandType::Data, 30.0, Block::CorrecaoOuJustificativa));
    }
    blocks.push((BandType::PageFooter, 25.0, Block::Mensagem));
    blocks.push((BandType::PageFooter, 5.0, Block::Rodape));
    for (band_type, height, block) in blocks {
        // The report has exactly one page at this point.
        let _ = report.add_band(EventoBand::band(ctx, band_type, height, block));
    }
}

/// Renders the DANFE of `evento`, which refers to `nfe`.
pub fn render_evento(
    nfe: Nfe,
    evento: Evento,
    options: &DanfeOptions,
) -> Result<Vec<u8>, DanfeError> {
    let ctx = Arc::new(Context {
        nfe,
        evento,
        mensagem_rodape: options.sistema.clone(),
        quebra_de_linha: options.caractere_quebra_de_linha.clone(),
    });
    let mut report = Report::new();
    report.set_font(FONT_FAMILY);
    let mut initialised = false;
    report.set_on_start(move |report, _| {
        if !initialised {
            build_report(report, &ctx);
            initialised = true;
        }
    });
    let mut engine = Engine::new(&mut report);
    engine.set_compressed(options.comprimir_pdf);
    Ok(engine.render()?)
}

/// Renders the DANFE of an event given as XML (`procEventoNFe`) together with the XML of the
/// invoice (`nfeProc` or `NFe`).
pub fn render_evento_xml(
    nfe_xml: &[u8],
    evento_xml: &[u8],
    options: &DanfeOptions,
) -> Result<Vec<u8>, DanfeError> {
    render_evento(
        Nfe::from_xml_bytes(nfe_xml)?,
        Evento::from_xml_bytes(evento_xml)?,
        options,
    )
}
