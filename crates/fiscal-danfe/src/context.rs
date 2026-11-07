//! The NF-e together with the printing options, and the text-building rules that sit between
//! them (`TNFeUtilsFPDF`, `TACBrNFeDANFEClass` and `TACBrDFeDANFeReport` in the original).

use crate::conversions;
use crate::decimal::Decimal;
use crate::format;
use crate::model::{Crt, DateTime, IndPag, Nfe, TpEmis, TpNf};
use crate::options::{
    DanfeOptions, DetArmamento, DetCombustivel, DetMedicamento, DetRastro, DetVeiculo,
    InfAdicProduto, InformacoesDePagamento, PercentualTributos,
};

/// What the bands read: the document and the options.
#[derive(Debug, Clone)]
pub struct DanfeContext {
    /// The NF-e.
    pub nfe: Nfe,
    /// Printing options.
    pub options: DanfeOptions,
}

fn date_or_empty(date: Option<DateTime>) -> String {
    date.map(|d| d.date_br()).unwrap_or_default()
}

fn is_zero_date(date: Option<DateTime>) -> bool {
    date.is_none()
}

impl DanfeContext {
    /// Builds the context.
    pub fn new(nfe: Nfe, options: DanfeOptions) -> Self {
        Self { nfe, options }
    }

    // ---- document state -------------------------------------------------------------

    /// The authorisation status says the invoice was canceled (101, 151, 155).
    pub fn nota_cancelada(&self) -> bool {
        matches!(self.nfe.proc_nfe.c_stat, 101 | 151 | 155)
    }

    /// The authorisation status says the use was denied (110, 301, 302, 303).
    pub fn nota_denegada(&self) -> bool {
        matches!(self.nfe.proc_nfe.c_stat, 110 | 301..=303)
    }

    /// Issued in EPEC/DPEC contingency.
    pub fn nota_epec(&self) -> bool {
        self.nfe.ide.tp_emis == TpEmis::Dpec
    }

    /// The 36-digit contingency data (`GerarChaveContingencia`).
    pub fn chave_contingencia(&self) -> String {
        let nfe = &self.nfe;
        let dest = &nfe.dest.ender_dest;
        let exterior = dest.uf == "EX";
        let mut key = String::new();
        if exterior {
            key.push_str("99");
        } else {
            let c_mun = if nfe.ide.tp_nf == TpNf::Saida {
                dest.c_mun
            } else {
                nfe.emit.ender_emit.c_mun
            };
            key.push_str(&c_mun.to_string().chars().take(2).collect::<String>());
        }
        key.push_str(match nfe.ide.tp_emis {
            TpEmis::Contingencia => "2",
            TpEmis::Fsda => "5",
            TpEmis::SvcAn => "6",
            TpEmis::SvcRs => "7",
            // Not a valid contingency mode: the original marks it with a zero.
            _ => "0",
        });
        if exterior {
            key.push_str(&"0".repeat(14));
        } else {
            let doc = nfe.dest.cnpj_cpf.trim();
            key.push_str(&format!("{doc:0>14}"));
        }
        let cents = (nfe.total.icms_tot.v_nf.round(2).to_f64() * 100.0).round() as u64;
        key.push_str(&format!("{cents:014}"));
        key.push(if nfe.total.icms_tot.v_icms.is_zero() {
            '2'
        } else {
            '1'
        });
        key.push(if nfe.total.icms_tot.v_st.is_zero() {
            '2'
        } else {
            '1'
        });
        key.push_str(&format!("{:02}", nfe.ide.d_emi.map_or(0, |d| d.day)));
        key.push_str(&contingency_check_digit(&key).to_string());
        key
    }

    /// Text of the invoice line above the instalments (`GetTextoFatura`).
    pub fn texto_fatura(&self) -> String {
        let fat = &self.nfe.cobr.fat;
        let ind_pag = match self.nfe.ide.ind_pag {
            Some(IndPag::Vista) => "Pagamento à Vista - ",
            Some(IndPag::Prazo) => "Pagamento à Prazo - ",
            _ => "",
        };
        let separador = "   -   ";
        let numero = if fat.n_fat.is_empty() {
            String::new()
        } else {
            format!("Fatura: {}{separador}", fat.n_fat)
        };
        format!(
            "{ind_pag}{numero}Valor Original: R$ {}{separador}Desconto: R$ {}{separador}Valor Líquido: R$ {}",
            fat.v_orig.money(),
            fat.v_desc.money(),
            fat.v_liq.money()
        )
    }

    /// The additional-information text, with explicit line breaks (`GetTextoAdicional`).
    pub fn texto_adicional(&self) -> String {
        self.manter_informacoes_dados_adicionais()
            .replace(&self.options.caractere_quebra_de_linha, "\n")
    }

    // ---- item texts -----------------------------------------------------------------

    /// Separator between the details of an item (`SeparadorDetalhamentos`).
    pub fn separador_detalhamentos(&self) -> &'static str {
        if self.options.quebra_linha_em_detalhamentos {
            "\n"
        } else {
            " - "
        }
    }

    /// `ManterCodigo`: the product code, or the GTIN when configured and valid.
    pub fn manter_codigo(&self, c_ean: &str, c_prod: &str) -> String {
        let ean = c_ean.trim();
        if self.options.imprime_codigo_ean && ean != "SEM GTIN" && !ean.is_empty() {
            ean.to_owned()
        } else {
            c_prod.trim().to_owned()
        }
    }

    /// `ManterXProd`: description plus the item details.
    pub fn manter_xprod(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        format!(
            "{}{}{}",
            det.prod.x_prod,
            self.manter_inf_ad_prod(item),
            self.manter_xped_nitemped(item)
        )
    }

    fn manter_xped_nitemped(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        let (x_ped, n_item_ped) = (&det.prod.x_ped, &det.prod.n_item_ped);
        if self.options.imprime_xped_nitemped && (!x_ped.is_empty() || !n_item_ped.is_empty()) {
            let text = format!(";N.Pedido = {x_ped}  N.Item = {n_item_ped}");
            text.replace(';', self.separador_detalhamentos())
        } else {
            String::new()
        }
    }

    /// `ManterinfAdProd` of the NF-e class: additional info, approximate taxes and the
    /// specific details.
    pub fn manter_inf_ad_prod(&self, item: usize) -> String {
        let mut out = self.base_inf_ad_prod(item);
        out.push_str(&self.manter_val_aprox(item));
        if self.options.imprime_detalhamento_especifico {
            out.push_str(&self.manter_veiculos(item));
            out.push_str(&self.manter_medicamentos(item));
            out.push_str(&self.manter_rastro(item));
            out.push_str(&self.manter_arma(item));
            out.push_str(&self.manter_combustivel(item));
        }
        out
    }

    fn base_inf_ad_prod(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        if self.options.exibe_infor_adic_produto != InfAdicProduto::Descricao {
            return String::new();
        }
        let separador = self.separador_detalhamentos();
        let text = det
            .inf_ad_prod
            .trim()
            .replace(&self.options.caractere_quebra_de_linha, separador);
        if text.is_empty() {
            String::new()
        } else {
            format!("{separador}{text}")
        }
    }

    fn manter_val_aprox(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        let tot_trib = det.imposto.v_tot_trib;
        if !(self.options.exibe_total_tributos_item && !tot_trib.is_zero()) {
            return String::new();
        }
        let mut out = format!(
            "{}Val Aprox Tributos: {}",
            self.separador_detalhamentos(),
            tot_trib.money()
        );
        let prod = &det.prod;
        let base = if self.options.tributos_percentual == PercentualTributos::ValorNf {
            let total = prod.v_prod
                + prod.v_frete
                + prod.v_outro
                + prod.v_seg
                + det.imposto.ipi.v_ipi
                + det.imposto.icms.v_icms_st;
            (!total.is_zero()).then_some(total)
        } else {
            (!prod.v_prod.is_zero()).then_some(prod.v_prod)
        };
        if let Some(base) = base {
            let percent = tot_trib.to_f64() * 100.0 / base.to_f64();
            out.push_str(&format!(" ({}%)", decimal_from_f64(percent).money()));
        }
        out
    }

    fn manter_veiculos(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        let v = &det.prod.veic_prod;
        if v.chassi.is_empty() {
            return String::new();
        }
        let sep = self.separador_detalhamentos();
        let has = |d: DetVeiculo| self.options.det_veiculos.contains(&d);
        let mut out = sep.to_owned();
        let mut line = |label: &str, value: &str| {
            out.push_str(label);
            out.push_str(value);
            out.push_str(sep);
        };
        if has(DetVeiculo::TpOp) {
            line(
                "TIPO DA OPERAÇÃO: ",
                conversions::veiculo_tipo_operacao(v.tp_op),
            );
        }
        if has(DetVeiculo::Chassi) {
            line("CHASSI: ", &v.chassi);
        }
        if has(DetVeiculo::CCor) {
            line("CÓDIGO DA COR: ", &v.c_cor);
        }
        if has(DetVeiculo::XCor) {
            line("NOME DA COR: ", &v.x_cor);
        }
        if has(DetVeiculo::Pot) {
            line("POTÊNCIA DO MOTOR: ", &v.pot);
        }
        if has(DetVeiculo::Cilin) {
            line("CILINDRADAS: ", &v.cilin);
        }
        if has(DetVeiculo::PesoL) {
            line("PESO LÍQUIDO: ", &v.peso_l);
        }
        if has(DetVeiculo::PesoB) {
            line("PESO BRUTO: ", &v.peso_b);
        }
        if has(DetVeiculo::NSerie) {
            line("NÚMERO DE SÉRIE: ", &v.n_serie);
        }
        if has(DetVeiculo::TpComb) {
            line(
                "COMBUSTÍVEL: ",
                &conversions::veiculo_combustivel(&v.tp_comb),
            );
        }
        if has(DetVeiculo::NMotor) {
            line("NÚMERO DO MOTOR: ", &v.n_motor);
        }
        if has(DetVeiculo::Cmt) {
            line("CAP. MÁX. TRAÇÃO: ", &v.cmt);
        }
        if has(DetVeiculo::Dist) {
            line("DISTÂNCIA ENTRE EIXOS: ", &v.dist);
        }
        if has(DetVeiculo::AnoMod) {
            line("ANO DO MODELO: ", &v.ano_mod.to_string());
        }
        if has(DetVeiculo::AnoFab) {
            line("ANO DE FABRICAÇÃO: ", &v.ano_fab.to_string());
        }
        if has(DetVeiculo::TpPint) {
            line("TIPO DE PINTURA: ", &v.tp_pint);
        }
        if has(DetVeiculo::TpVeic) {
            line("TIPO DE VEÍCULO: ", &conversions::veiculo_tipo(v.tp_veic));
        }
        if has(DetVeiculo::EspVeic) {
            line(
                "ESPÉCIE DO VEÍCULO: ",
                &conversions::veiculo_especie(v.esp_veic),
            );
        }
        if has(DetVeiculo::Vin) {
            line("VIN (CHASSI): ", conversions::veiculo_vin(&v.vin));
        }
        if has(DetVeiculo::CondVeic) {
            line(
                "CONDIÇÃO DO VEÍCULO: ",
                conversions::veiculo_condicao(v.cond_veic),
            );
        }
        if has(DetVeiculo::CMod) {
            line("CÓDIGO MARCA MODELO: ", &v.c_mod);
        }
        if has(DetVeiculo::CCorDenatran) {
            line(
                "CÓDIGO COR DENATRAN: ",
                &conversions::veiculo_cor_denatran(&v.c_cor_denatran),
            );
        }
        if has(DetVeiculo::Lota) {
            line("CAPACIDADE MÁXIMA DE LOTAÇÃO: ", &v.lota.to_string());
        }
        if has(DetVeiculo::TpRest) {
            line("RESTRIÇÃO: ", &conversions::veiculo_restricao(v.tp_rest));
        }
        out
    }

    fn manter_medicamentos(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        if det.prod.med.is_empty() {
            return String::new();
        }
        let sep = self.separador_detalhamentos();
        let has = |d: DetMedicamento| self.options.det_medicamentos.contains(&d);
        let mut out = sep.to_owned();
        for med in &det.prod.med {
            if self.nfe.versao >= 4.0 {
                out.push_str(&format!("Registro ANVISA {}{sep}", med.c_prod_anvisa));
            } else {
                if has(DetMedicamento::NLote) && !med.n_lote.is_empty() {
                    out.push_str(&format!("LOTE: {}{sep}", med.n_lote));
                }
                if has(DetMedicamento::QLote) && !med.q_lote.is_zero() {
                    out.push_str(&format!("QTD: {}{sep}", med.q_lote.format_br(3, true)));
                }
                if has(DetMedicamento::DFab) && !is_zero_date(med.d_fab) {
                    out.push_str(&format!("FAB: {}{sep}", date_or_empty(med.d_fab)));
                }
                if has(DetMedicamento::DVal) && !is_zero_date(med.d_val) {
                    out.push_str(&format!("VAL: {}{sep}", date_or_empty(med.d_val)));
                }
            }
            if has(DetMedicamento::VPmc) && med.v_pmc.is_positive() {
                out.push_str(&format!("PMC: R${}{sep}", med.v_pmc.money()));
            }
        }
        out
    }

    fn manter_arma(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        if det.prod.arma.is_empty() {
            return String::new();
        }
        let sep = self.separador_detalhamentos();
        let has = |d: DetArmamento| self.options.det_armamentos.contains(&d);
        let mut out = sep.to_owned();
        for arma in &det.prod.arma {
            if has(DetArmamento::TpArma) {
                out.push_str(&format!(
                    "TIPO DE ARMA: {}{sep}",
                    conversions::arma_tipo(arma.tp_arma)
                ));
            }
            if has(DetArmamento::NSerie) {
                out.push_str(&format!("No. SÉRIE ARMA: {}{sep}", arma.n_serie));
            }
            if has(DetArmamento::NCano) {
                out.push_str(&format!("No. SÉRIE CANO: {}{sep}", arma.n_cano));
            }
            if has(DetArmamento::Descr) {
                out.push_str(&format!("DESCRIÇÃO ARMA: {}{sep}", arma.descr));
            }
        }
        out
    }

    fn manter_combustivel(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        let c = &det.prod.comb;
        if c.c_prod_anp == 0 {
            return String::new();
        }
        let sep = self.separador_detalhamentos();
        let has = |d: DetCombustivel| self.options.det_combustiveis.contains(&d);
        let mut out = sep.to_owned();
        if has(DetCombustivel::CProdAnp) {
            out.push_str(&format!("CÓD. PRODUTO ANP: {}{sep}", c.c_prod_anp));
        }
        if has(DetCombustivel::Codif) && !c.codif.is_empty() {
            out.push_str(&format!("AUTORIZAÇÃO/CODIF: {}{sep}", c.codif));
        }
        if has(DetCombustivel::QTemp) && c.q_temp.is_positive() {
            out.push_str(&format!(
                "QTD. FATURADA TEMP. AMBIENTE: {}{sep}",
                c.q_temp.format_br(4, true)
            ));
        }
        if has(DetCombustivel::UfCons) {
            out.push_str(&format!("UF DE CONSUMO: {}{sep}", c.uf_cons));
        }
        if c.cide_q_bc_prod.is_positive() {
            if has(DetCombustivel::QBcProd) {
                out.push_str(&format!(
                    "BASE DE CÁLCULO CIDE: {}{sep}",
                    c.cide_q_bc_prod.format_br(4, true)
                ));
            }
            if has(DetCombustivel::VAliqProd) {
                out.push_str(&format!(
                    "ALÍQUOTA CIDE: {}{sep}",
                    c.cide_v_aliq_prod.format_br(4, true)
                ));
            }
            if has(DetCombustivel::VCide) {
                out.push_str(&format!("VALOR CIDE: {}{sep}", c.cide_v_cide.money()));
            }
        }
        if c.encerrante_n_bico > 0 {
            out.push_str(&format!("ENCERRANTE{sep}"));
            out.push_str(&format!("BICO: {}{sep}", c.encerrante_n_bico));
            if c.encerrante_n_bomba > 0 {
                out.push_str(&format!("BOMBA: {}{sep}", c.encerrante_n_bomba));
            }
            out.push_str(&format!("TANQUE: {}{sep}", c.encerrante_n_tanque));
            out.push_str(&format!(
                "NO INÍCIO: {}{sep}",
                c.encerrante_v_enc_ini.format_br(3, true)
            ));
            out.push_str(&format!(
                "NO FINAL: {}{sep}",
                c.encerrante_v_enc_fin.format_br(3, true)
            ));
        }
        out
    }

    fn manter_rastro(&self, item: usize) -> String {
        let Some(det) = self.nfe.det.get(item) else {
            return String::new();
        };
        if self.options.det_rastros.is_empty() || det.prod.rastro.is_empty() {
            return String::new();
        }
        let sep = self.separador_detalhamentos();
        let has = |d: DetRastro| self.options.det_rastros.contains(&d);
        let mut out = sep.to_owned();
        for r in &det.prod.rastro {
            if has(DetRastro::NLote) && !r.n_lote.is_empty() {
                out.push_str(&format!("LOTE: {}{sep}", r.n_lote));
            }
            if has(DetRastro::QLote) && !r.q_lote.is_zero() {
                out.push_str(&format!("QTD: {}{sep}", r.q_lote.format_br(3, true)));
            }
            if has(DetRastro::DFab) && !is_zero_date(r.d_fab) {
                out.push_str(&format!("FAB: {}{sep}", date_or_empty(r.d_fab)));
            }
            if has(DetRastro::DVal) && !is_zero_date(r.d_val) {
                out.push_str(&format!("VAL: {}{sep}", date_or_empty(r.d_val)));
            }
            if has(DetRastro::CAgreg) && !r.c_agreg.is_empty() {
                out.push_str(&format!("C.AGREGAÇÃO: {}{sep}", r.c_agreg));
            }
        }
        out
    }

    /// `ManterVprod`: the item value, optionally net of discount.
    pub fn manter_vprod(&self, v_prod: Decimal, v_desc: Decimal) -> String {
        let value = if self.options.imprime_total_liquido {
            v_prod - v_desc
        } else {
            v_prod
        };
        value.money()
    }

    /// `ManterCst`: the CSOSN for Simples Nacional/MEI items, the CST otherwise.
    pub fn manter_cst(&self, crt: Crt, csosn: &str, cst: &str) -> String {
        if matches!(crt, Crt::SimplesNacional | Crt::Mei)
            && !matches!(cst, "02" | "15" | "53" | "61")
        {
            csosn.to_owned()
        } else {
            cst.to_owned()
        }
    }

    // ---- additional information -----------------------------------------------------

    /// `ManterInformacoesDadosAdicionais`.
    pub fn manter_informacoes_dados_adicionais(&self) -> String {
        let text = [
            self.manter_protocolo(),
            self.manter_suframa(),
            self.manter_pagamentos(),
            self.manter_doc_referenciados(),
            self.manter_inf_ad_fisco(),
            self.manter_obs_fisco(),
            self.manter_proc_referenciado(),
            self.manter_inf_contr(),
            self.manter_inf_compl(),
            self.manter_contingencia(),
        ]
        .concat();
        text.replace(&self.options.caractere_quebra_de_linha, "\n")
    }

    fn quebra(&self) -> &str {
        &self.options.caractere_quebra_de_linha
    }

    fn manter_protocolo(&self) -> String {
        let nfe = &self.nfe;
        if matches!(nfe.ide.tp_emis, TpEmis::Contingencia | TpEmis::Fsda)
            && nfe.proc_nfe.c_stat == 100
        {
            format!(
                "PROTOCOLO DE AUTORIZAÇÃO DE USO: {} {}{}",
                nfe.proc_nfe.n_prot,
                nfe.proc_nfe
                    .dh_recbto
                    .map(|d| d.datetime_br())
                    .unwrap_or_default(),
                self.quebra()
            )
        } else {
            String::new()
        }
    }

    fn manter_suframa(&self) -> String {
        if !self.nfe.dest.isuf.is_empty() && self.options.exibe_dados_inscricao_suframa {
            format!("INSCRIÇÃO SUFRAMA: {}{}", self.nfe.dest.isuf, self.quebra())
        } else {
            String::new()
        }
    }

    fn manter_pagamentos(&self) -> String {
        if self.options.exibe_campo_de_pagamento != InformacoesDePagamento::Adicionais
            || self.nfe.pag.is_empty()
        {
            return String::new();
        }
        let mut out = String::new();
        for pag in &self.nfe.pag {
            let temp = format!(
                "{} R${}",
                conversions::forma_pagamento(&pag.t_pag, &pag.x_pag),
                pag.v_pag.money()
            );
            out = lado_a_lado(&out, "Pagamento(s): (", &temp);
        }
        if !out.is_empty() {
            out.push(')');
            out.push_str(self.separador_detalhamentos());
        }
        out
    }

    fn manter_doc_referenciados(&self) -> String {
        if !self.options.exibe_dados_doc_referenciados || self.nfe.ide.nf_ref.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        let add = |execute: bool, out: &mut String, start: &str, text: &str| {
            if execute {
                *out = lado_a_lado(out, start, &format!("({text})"));
            }
        };
        for r in &self.nfe.ide.nf_ref {
            let model_label = match r.ref_nfe.get(20..22).and_then(|m| m.parse::<u32>().ok()) {
                Some(59) => "CFe-SAT Ref.:",
                Some(65) => "NFCe Ref.:",
                _ => "NFe Ref.:",
            };
            add(
                !r.ref_nfe.is_empty(),
                &mut out,
                model_label,
                &format::chave_acesso(&r.ref_nfe),
            );
            add(
                !r.ref_cte.is_empty(),
                &mut out,
                "CTe Ref.:",
                &format::chave_acesso(&r.ref_cte),
            );
            add(
                !r.ecf_modelo.is_empty(),
                &mut out,
                "ECF Ref.:",
                &format!(
                    "modelo: {} ECF: {} COO: {}",
                    r.ecf_modelo, r.ecf_n_ecf, r.ecf_n_coo
                ),
            );
            add(
                !r.nf_cnpj.is_empty(),
                &mut out,
                "NF Ref.:",
                &format!(
                    "série: {} número: {} emit: {} modelo: {}",
                    r.nf_serie,
                    r.nf_n_nf,
                    format::cnpj_ou_cpf(&r.nf_cnpj),
                    r.nf_modelo
                ),
            );
            add(
                r.nfp_n_nf > 0,
                &mut out,
                "NFP Ref.:",
                &format!(
                    "série: {} número: {} modelo: {} emit: {} IE: {} UF: {}",
                    r.nfp_serie,
                    r.nfp_n_nf,
                    r.nfp_modelo,
                    format::cnpj_ou_cpf(&r.nfp_cnpj_cpf),
                    r.nfp_ie,
                    format::codigo_uf_para_uf(r.nfp_c_uf)
                ),
            );
        }
        if !out.is_empty() {
            out.push_str(self.separador_detalhamentos());
        }
        out
    }

    fn manter_inf_ad_fisco(&self) -> String {
        let info = &self.nfe.inf_adic;
        let fisco = info
            .inf_ad_fisco
            .strip_suffix('\n')
            .unwrap_or(&info.inf_ad_fisco);
        let quebra = self.quebra();
        let sequence = !info.inf_cpl.is_empty()
            || !info.obs_fisco.is_empty()
            || !info.proc_ref.is_empty()
            || !info.obs_cont.is_empty();
        if fisco.is_empty() {
            String::new()
        } else if sequence {
            format!(
                "{fisco}{}",
                if fisco.ends_with(quebra) { "" } else { quebra }
            )
        } else {
            fisco.to_owned()
        }
    }

    fn manter_obs_fisco(&self) -> String {
        self.obs_list(&self.nfe.inf_adic.obs_fisco)
    }

    fn manter_inf_contr(&self) -> String {
        if self.options.imprime_inf_contr {
            self.obs_list(&self.nfe.inf_adic.obs_cont)
        } else {
            String::new()
        }
    }

    fn obs_list(&self, list: &[crate::model::Obs]) -> String {
        if list.is_empty() {
            return String::new();
        }
        let quebra = self.quebra();
        let mut out = String::new();
        for (i, o) in list.iter().enumerate() {
            out.push_str(&format!("{}: {}", o.x_campo, o.x_texto));
            if i + 1 != list.len() {
                out.push_str(quebra);
            }
        }
        out.push_str(quebra);
        out.push(' ');
        out
    }

    fn manter_proc_referenciado(&self) -> String {
        let procs = &self.nfe.inf_adic.proc_ref;
        let Some(last) = procs.last() else {
            return String::new();
        };
        // Only the last process is printed.
        let quebra = self.quebra();
        format!(
            "PROCESSO OU ATO CONCESSÓRIO Nº: {} - ORIGEM: {}{quebra} ",
            last.n_proc,
            conversions::ind_proc(&last.ind_proc)
        )
    }

    fn manter_inf_compl(&self) -> String {
        self.nfe.inf_adic.inf_cpl.clone()
    }

    fn manter_contingencia(&self) -> String {
        let ide = &self.nfe.ide;
        let quebra = self.quebra();
        match ide.tp_emis {
            TpEmis::OffLine
            | TpEmis::Contingencia
            | TpEmis::Fsda
            | TpEmis::Scan
            | TpEmis::SvcAn
            | TpEmis::SvcRs
            | TpEmis::SvcSp => {
                format!(
                    "DANFE EM CONTINGÊNCIA, IMPRESSO EM DECORRÊNCIA DE PROBLEMAS TÉCNICOS{quebra}"
                )
            }
            TpEmis::Dpec => format!(
                "DANFE IMPRESSO EM CONTINGÊNCIA - DPEC REGULARMENTE RECEBIDA PELA RECEITA FEDERAL DO BRASIL{quebra}\
                 DATA/HORA INÍCIO: {}{quebra}MOTIVO CONTINGÊNCIA: {}{quebra}",
                ide.dh_cont.map_or(" ".to_owned(), |d| d.datetime_br()),
                if ide.x_just.is_empty() {
                    " "
                } else {
                    ide.x_just.as_str()
                }
            ),
            TpEmis::Normal => String::new(),
        }
    }
}

/// `MontaLadoALado` of the original: joins `start` and `text` to what is already in `result`.
fn lado_a_lado(result: &str, start: &str, text: &str) -> String {
    let joined = if result.is_empty() {
        start.to_owned()
    } else if !result.contains(start) {
        format!("{result}, {start}")
    } else {
        format!("{result}, ")
    };
    format!("{joined}{text}")
}

/// Converts a percentage computed in floating point back to a [`Decimal`].
fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::parse(&format!("{value:.8}")).unwrap_or(Decimal::ZERO)
}

/// Check digit of the 35-digit contingency data (modulo 11 with the weights of the
/// integration manual).
fn contingency_check_digit(key: &str) -> u32 {
    const WEIGHTS: &str = "43298765432987654329876543298765432";
    let sum: u32 = key
        .bytes()
        .take(35)
        .zip(WEIGHTS.bytes())
        .map(|(k, w)| u32::from(k.to_ascii_uppercase().wrapping_sub(b'0')) * u32::from(w - b'0'))
        .sum();
    if sum % 11 < 2 { 0 } else { 11 - sum % 11 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(xml: &str) -> DanfeContext {
        DanfeContext::new(Nfe::from_xml(xml).expect("nfe"), DanfeOptions::default())
    }

    const XML: &str = include_str!("../tests/fixtures/nfe_55.xml");

    #[test]
    fn product_description_includes_additional_info_on_a_new_line() {
        let c = context(XML);
        assert_eq!(c.manter_xprod(1), "CHAVE DE FENDA PHILLIPS 1/4");
        assert_eq!(
            c.manter_xprod(0),
            "PARAFUSO SEXTAVADO ACO INOX 8MM CAIXA COM 100 UNIDADES\nLote 2026-07 validade indeterminada"
        );
        let mut c = c;
        c.options.quebra_linha_em_detalhamentos = false;
        assert!(
            c.manter_xprod(0)
                .ends_with("UNIDADES - Lote 2026-07 validade indeterminada")
        );
        c.options.exibe_infor_adic_produto = InfAdicProduto::Nenhum;
        assert!(c.manter_xprod(0).ends_with("UNIDADES"));
    }

    #[test]
    fn approximate_taxes_per_item() {
        let mut c = context(XML);
        c.options.exibe_total_tributos_item = true;
        let text = c.manter_xprod(0);
        assert!(
            text.contains("Val Aprox Tributos: 82,90 (18,22%)"),
            "{text}"
        );
        c.options.tributos_percentual = PercentualTributos::ValorNf;
        // 455.00 + 22.75 IPI = 477.75 -> 17,35%
        assert!(c.manter_xprod(0).contains("(17,35%)"));
    }

    #[test]
    fn code_column_uses_the_product_code_unless_gtin_is_requested() {
        let mut c = context(XML);
        assert_eq!(c.manter_codigo("7891234567895", " 0002 "), "0002");
        c.options.imprime_codigo_ean = true;
        assert_eq!(c.manter_codigo("7891234567895", "0002"), "7891234567895");
        assert_eq!(c.manter_codigo("SEM GTIN", "0002"), "0002");
        assert_eq!(c.manter_codigo("", "0002"), "0002");
    }

    #[test]
    fn cst_or_csosn_by_regime() {
        let c = context(XML);
        assert_eq!(c.manter_cst(Crt::RegimeNormal, "", "00"), "00");
        assert_eq!(c.manter_cst(Crt::SimplesNacional, "102", ""), "102");
        assert_eq!(c.manter_cst(Crt::SimplesNacional, "", "02"), "02");
        assert_eq!(c.manter_cst(Crt::Mei, "400", ""), "400");
    }

    #[test]
    fn item_value_can_be_net_of_discount() {
        let mut c = context(XML);
        let d = |s| Decimal::parse(s).expect("decimal");
        assert_eq!(c.manter_vprod(d("38.70"), d("1.70")), "38,70");
        c.options.imprime_total_liquido = true;
        assert_eq!(c.manter_vprod(d("38.70"), d("1.70")), "37,00");
    }

    #[test]
    fn additional_information_combines_the_pieces() {
        let c = context(XML);
        let text = c.texto_adicional();
        assert!(text.starts_with("Vendedor: Maria Souza\n "), "{text:?}");
        assert!(text.contains("Pedido de compra 4521."));
        // `|` is the XML line break marker; the plain fixture has none.
        assert!(!text.contains('|'));
    }

    #[test]
    fn invoice_line_text() {
        let c = context(XML);
        assert_eq!(
            c.texto_fatura(),
            "Fatura: 001234   -   Valor Original: R$ 664,75   -   Desconto: R$ 0,00   -   Valor Líquido: R$ 664,75"
        );
    }

    #[test]
    fn referenced_documents_join_side_by_side() {
        let mut c = context(XML);
        c.nfe.ide.nf_ref = vec![
            crate::model::NfRef {
                ref_nfe: "35260511222333000181550010000000011000000015".to_owned(),
                ..Default::default()
            },
            crate::model::NfRef {
                ref_nfe: "35260511222333000181650010000000011000000019".to_owned(),
                ..Default::default()
            },
        ];
        let text = c.manter_doc_referenciados();
        assert!(text.starts_with("NFe Ref.:(3526 0511"), "{text}");
        assert!(text.contains(", NFCe Ref.:(3526 0511"), "{text}");
        assert!(text.ends_with('\n'));
        c.options.exibe_dados_doc_referenciados = false;
        assert_eq!(c.manter_doc_referenciados(), "");
    }

    #[test]
    fn payments_in_the_additional_information() {
        let mut c = context(XML);
        c.options.exibe_campo_de_pagamento = InformacoesDePagamento::Adicionais;
        assert!(
            c.manter_pagamentos()
                .starts_with("Pagamento(s): (Boleto Bancário R$664,75)")
        );
        c.nfe.pag.push(crate::model::DetPag {
            t_pag: "01".to_owned(),
            v_pag: Decimal::parse("10").expect("d"),
            ..Default::default()
        });
        assert!(
            c.manter_pagamentos()
                .contains("Boleto Bancário R$664,75, Dinheiro R$10,00)")
        );
    }

    #[test]
    fn contingency_text_depends_on_the_issuing_mode() {
        let mut c = context(XML);
        assert_eq!(c.manter_contingencia(), "");
        c.nfe.ide.tp_emis = TpEmis::Fsda;
        assert!(c.manter_contingencia().starts_with("DANFE EM CONTINGÊNCIA"));
        c.nfe.ide.tp_emis = TpEmis::Dpec;
        assert!(
            c.manter_contingencia()
                .contains("DPEC REGULARMENTE RECEBIDA")
        );
    }

    #[test]
    fn contingency_key_has_36_digits_with_a_valid_check_digit() {
        let mut c = context(XML);
        c.nfe.ide.tp_emis = TpEmis::Fsda;
        let key = c.chave_contingencia();
        assert_eq!(key.len(), 36, "{key}");
        assert!(key.bytes().all(|b| b.is_ascii_digit()));
        // UF of the recipient city (33 = RJ), emission 5, recipient CNPJ, vNF in cents.
        assert!(key.starts_with("3359988877700016600000000066475"), "{key}");
        assert_eq!(&key[31..33], "12", "ICMS highlighted, no ST");
        assert_eq!(&key[33..35], "20", "emission day");
        assert_eq!(contingency_check_digit(&key[..35]).to_string(), key[35..]);
    }

    #[test]
    fn document_state() {
        let mut c = context(XML);
        assert!(!c.nota_cancelada() && !c.nota_denegada() && !c.nota_epec());
        c.nfe.proc_nfe.c_stat = 101;
        assert!(c.nota_cancelada());
        c.nfe.proc_nfe.c_stat = 302;
        assert!(c.nota_denegada());
    }
}
