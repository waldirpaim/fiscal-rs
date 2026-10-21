//! Printing options of the DANFE (the published properties of `TACBrNFeDANFEClass` and its
//! ancestors).

/// Where the receipt stub (canhoto) goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PosCanhoto {
    /// At the top of the first page.
    #[default]
    Cabecalho,
    /// At the bottom of the first page.
    Rodape,
    /// On the left side of the first page (rotated).
    Esquerda,
    /// No receipt stub (an extension: the original always prints one).
    Nenhum,
}

/// How much of the product's additional information goes next to its description.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InfAdicProduto {
    /// None.
    Nenhum,
    /// Appended to the description.
    #[default]
    Descricao,
    /// Printed separately (nothing is added to the description).
    Separadamente,
}

/// Which base the approximate-taxes percentage uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PercentualTributos {
    /// The products total.
    #[default]
    ValorProdutos,
    /// The invoice total.
    ValorNf,
    /// A fixed percentage (`tributos_percentual_personalizado`).
    Personalizado,
}

/// Whether and where the payment methods are printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InformacoesDePagamento {
    /// Never.
    #[default]
    Nunca,
    /// In the additional information.
    Adicionais,
    /// In their own box.
    Quadro,
}

/// Vehicle fields printed under the item (`TDetVeiculo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum DetVeiculo {
    TpOp,
    Chassi,
    CCor,
    XCor,
    Pot,
    Cilin,
    PesoL,
    PesoB,
    NSerie,
    TpComb,
    NMotor,
    Cmt,
    Dist,
    AnoMod,
    AnoFab,
    TpPint,
    TpVeic,
    EspVeic,
    Vin,
    CondVeic,
    CMod,
    CCorDenatran,
    Lota,
    TpRest,
}

/// Medicine fields (`TDetMedicamento`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum DetMedicamento {
    NLote,
    QLote,
    DFab,
    DVal,
    VPmc,
}

/// Firearm fields (`TDetArmamento`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum DetArmamento {
    TpArma,
    NSerie,
    NCano,
    Descr,
}

/// Fuel fields (`TDetCombustivel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum DetCombustivel {
    CProdAnp,
    Codif,
    QTemp,
    UfCons,
    Cide,
    QBcProd,
    VAliqProd,
    VCide,
}

/// Traceability fields (`TDetRastro`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum DetRastro {
    NLote,
    QLote,
    DFab,
    DVal,
    CAgreg,
}

/// Options of the DANFE.
#[derive(Debug, Clone)]
pub struct DanfeOptions {
    // ---- layout ----
    /// Left margin in mm. The original treats its default (6) as "unset" and uses 2.
    pub margem_esquerda: f64,
    /// Top margin in mm (default 8 means "unset": 2 is used).
    pub margem_superior: f64,
    /// Right margin in mm (default 5.1 means "unset": 2 is used).
    pub margem_direita: f64,
    /// Bottom margin in mm (default 8 means "unset": 2 is used).
    pub margem_inferior: f64,
    /// Logo image (PNG or JPEG bytes).
    pub logo: Option<Vec<u8>>,
    /// Put the logo above the issuer data.
    pub logo_em_cima: bool,
    /// Make the logo fill the issuer box (ignores the issuer text).
    pub expande_logo_marca: bool,
    /// Receipt stub position.
    pub pos_canhoto: PosCanhoto,
    /// Print the "canceled" watermark.
    pub cancelada: bool,
    /// Compress the PDF content (default); turn off to inspect the content streams.
    pub comprimir_pdf: bool,
    /// Text printed in the footer (split in up to three parts by `|`: left, centre, right),
    /// usually the name of the issuing system (`Sistema`).
    pub sistema: String,
    // ---- text ----
    /// The line-break marker used inside XML texts (`|`).
    pub caractere_quebra_de_linha: String,
    /// Break lines between item details (otherwise they are joined with ` - `).
    pub quebra_linha_em_detalhamentos: bool,
    /// Use the GTIN instead of the product code in the code column.
    pub imprime_codigo_ean: bool,
    /// Print the item total net of discount.
    pub imprime_total_liquido: bool,
    /// Where the item's additional information goes.
    pub exibe_infor_adic_produto: InfAdicProduto,
    /// Print the approximate taxes of each item.
    pub exibe_total_tributos_item: bool,
    /// Base of the approximate-taxes percentage.
    pub tributos_percentual: PercentualTributos,
    /// Percentage used with [`PercentualTributos::Personalizado`].
    pub tributos_percentual_personalizado: f64,
    /// Print the vehicle/medicine/firearm/fuel/traceability details of each item.
    pub imprime_detalhamento_especifico: bool,
    /// Print the purchase order and item next to the item description.
    pub imprime_xped_nitemped: bool,
    /// Print the contributor's free-use notes (`obsCont`).
    pub imprime_inf_contr: bool,
    /// Print the referenced documents in the additional information.
    pub exibe_dados_doc_referenciados: bool,
    /// Print the recipient's SUFRAMA registration.
    pub exibe_dados_inscricao_suframa: bool,
    /// Show exempted ICMS as a discount.
    pub exibe_icms_desonerado_como_desconto: bool,
    /// Print the invoice line above the instalments.
    pub exibe_campo_fatura: bool,
    /// Print the instalments.
    pub exibe_campo_duplicata: bool,
    /// Whether and where payments are printed.
    pub exibe_campo_de_pagamento: InformacoesDePagamento,
    /// Format the document number as `000.000.000`.
    pub formatar_numero_documento: bool,
    /// Print the trade name instead of the legal name (NFC-e).
    pub imprime_nome_fantasia: bool,
    // ---- NFC-e (receipt) ----
    /// Paper width in mm (the original treats 302 as "unset" and uses 80); below 70 mm fonts
    /// shrink.
    pub largura_bobina: u32,
    /// Print the QR Code at the side instead of below the access key.
    pub imprime_qrcode_lateral: bool,
    /// Print the items list.
    pub exibir_itens: bool,
    /// Print the trade name above the legal name.
    pub exibir_nome_fantasia: bool,
    /// Print the consumer's name.
    pub exibir_consumidor_nome: bool,
    /// Print the consumer's address.
    pub exibir_consumidor_endereco: bool,
    /// Print the simplified type 2 receipt (a title and totals-first layout).
    pub simplificado_tipo2: bool,
    /// Vehicle details to print.
    pub det_veiculos: Vec<DetVeiculo>,
    /// Medicine details to print.
    pub det_medicamentos: Vec<DetMedicamento>,
    /// Firearm details to print.
    pub det_armamentos: Vec<DetArmamento>,
    /// Fuel details to print.
    pub det_combustiveis: Vec<DetCombustivel>,
    /// Traceability details to print.
    pub det_rastros: Vec<DetRastro>,
}

impl Default for DanfeOptions {
    fn default() -> Self {
        use DetVeiculo as V;
        Self {
            margem_esquerda: 6.0,
            margem_superior: 8.0,
            margem_direita: 5.1,
            margem_inferior: 8.0,
            logo: None,
            logo_em_cima: false,
            expande_logo_marca: false,
            pos_canhoto: PosCanhoto::Cabecalho,
            cancelada: false,
            comprimir_pdf: true,
            sistema: String::new(),
            caractere_quebra_de_linha: "|".to_owned(),
            quebra_linha_em_detalhamentos: true,
            imprime_codigo_ean: false,
            imprime_total_liquido: false,
            exibe_infor_adic_produto: InfAdicProduto::Descricao,
            exibe_total_tributos_item: false,
            tributos_percentual: PercentualTributos::ValorProdutos,
            tributos_percentual_personalizado: 0.0,
            imprime_detalhamento_especifico: true,
            imprime_xped_nitemped: false,
            imprime_inf_contr: true,
            exibe_dados_doc_referenciados: true,
            exibe_dados_inscricao_suframa: true,
            exibe_icms_desonerado_como_desconto: false,
            exibe_campo_fatura: true,
            exibe_campo_duplicata: true,
            exibe_campo_de_pagamento: InformacoesDePagamento::Nunca,
            formatar_numero_documento: true,
            imprime_nome_fantasia: false,
            largura_bobina: 302,
            imprime_qrcode_lateral: false,
            exibir_itens: true,
            exibir_nome_fantasia: false,
            exibir_consumidor_nome: false,
            exibir_consumidor_endereco: false,
            simplificado_tipo2: false,
            det_veiculos: vec![
                V::Chassi,
                V::XCor,
                V::NSerie,
                V::TpComb,
                V::NMotor,
                V::AnoMod,
                V::AnoFab,
            ],
            det_medicamentos: vec![
                DetMedicamento::NLote,
                DetMedicamento::QLote,
                DetMedicamento::DFab,
                DetMedicamento::DVal,
                DetMedicamento::VPmc,
            ],
            det_armamentos: vec![
                DetArmamento::TpArma,
                DetArmamento::NSerie,
                DetArmamento::NCano,
                DetArmamento::Descr,
            ],
            det_combustiveis: vec![
                DetCombustivel::CProdAnp,
                DetCombustivel::Codif,
                DetCombustivel::QTemp,
                DetCombustivel::UfCons,
                DetCombustivel::Cide,
                DetCombustivel::QBcProd,
                DetCombustivel::VAliqProd,
                DetCombustivel::VCide,
            ],
            det_rastros: vec![
                DetRastro::NLote,
                DetRastro::QLote,
                DetRastro::DFab,
                DetRastro::DVal,
                DetRastro::CAgreg,
            ],
        }
    }
}

impl DanfeOptions {
    /// The margins actually used: each value equal to the class default is replaced by 2 mm
    /// (`left, top, right, bottom`).
    pub fn effective_margins(&self) -> (f64, f64, f64, f64) {
        let pick = |value: f64, default: f64| if value != default { value } else { 2.0 };
        (
            pick(self.margem_esquerda, 6.0),
            pick(self.margem_superior, 8.0),
            pick(self.margem_direita, 5.1),
            pick(self.margem_inferior, 8.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_margins_collapse_to_two_millimetres() {
        assert_eq!(
            DanfeOptions::default().effective_margins(),
            (2.0, 2.0, 2.0, 2.0)
        );
        let custom = DanfeOptions {
            margem_esquerda: 10.0,
            margem_inferior: 4.0,
            ..DanfeOptions::default()
        };
        assert_eq!(custom.effective_margins(), (10.0, 2.0, 2.0, 4.0));
    }
}
