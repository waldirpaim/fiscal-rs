//! The NF-e as the DANFE needs it.
//!
//! Field names follow the XML tags of the layout (`xNome` -> `x_nome`). Missing numeric
//! fields are zero and missing text fields are empty, which is what the layout code relies
//! on; amounts are exact [`Decimal`]s.

use crate::decimal::Decimal;

/// Date and, when the source has it, time of day (the UTC offset is dropped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    /// Year.
    pub year: i32,
    /// Month, 1 to 12.
    pub month: u32,
    /// Day, 1 to 31.
    pub day: u32,
    /// Hour, 0 to 23.
    pub hour: u32,
    /// Minute.
    pub minute: u32,
    /// Second.
    pub second: u32,
}

impl DateTime {
    /// Parses `YYYY-MM-DD` optionally followed by `THH:MM:SS` and a UTC offset.
    pub fn parse(text: &str) -> Option<Self> {
        let t = text.trim();
        let (date, time) = match t.split_once('T') {
            Some((d, rest)) => (d, Some(rest)),
            None => (t, None),
        };
        let mut parts = date.split('-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        let (mut hour, mut minute, mut second) = (0, 0, 0);
        if let Some(time) = time {
            let clock = time.split(['+', '-', 'Z']).next().unwrap_or("");
            let mut p = clock.split(':');
            hour = p.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            minute = p.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            second = p
                .next()
                .and_then(|v| v.split('.').next().and_then(|s| s.parse().ok()))
                .unwrap_or(0);
        }
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    /// `true` when the time of day is exactly midnight.
    pub fn time_is_zero(&self) -> bool {
        self.hour == 0 && self.minute == 0 && self.second == 0
    }

    /// `dd/mm/yyyy`
    pub fn date_br(&self) -> String {
        format!("{:02}/{:02}/{:04}", self.day, self.month, self.year)
    }

    /// `dd/mm/yyyy hh:nn:ss`
    pub fn datetime_br(&self) -> String {
        format!("{} {}", self.date_br(), self.time_string())
    }

    /// `hh:nn:ss`
    pub fn time_string(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

/// `tpNF`: operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TpNf {
    /// 0 — entrada.
    Entrada,
    /// 1 — saída.
    #[default]
    Saida,
}

impl TpNf {
    /// The XML code (`"0"` or `"1"`).
    pub fn code(self) -> &'static str {
        match self {
            Self::Entrada => "0",
            Self::Saida => "1",
        }
    }
}

/// `tpImp`: DANFE format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TpImp {
    /// 0 — no DANFE.
    SemGeracao,
    /// 1 — portrait.
    #[default]
    Retrato,
    /// 2 — landscape.
    Paisagem,
    /// 3 — simplified.
    Simplificado,
    /// 4 — NFC-e.
    NfCe,
    /// 5 — electronic message.
    MsgEletronica,
    /// 6 — simplified DANFE type 2.
    SimplificadoTipo2,
}

/// `tpEmis`: issuing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TpEmis {
    /// 1 — normal.
    #[default]
    Normal,
    /// 2 — FS-IA contingency.
    Contingencia,
    /// 3 — SCAN.
    Scan,
    /// 4 — EPEC/DPEC.
    Dpec,
    /// 5 — FS-DA contingency.
    Fsda,
    /// 6 — SVC-AN.
    SvcAn,
    /// 7 — SVC-RS.
    SvcRs,
    /// 8 — SVC-SP.
    SvcSp,
    /// 9 — off-line (NFC-e).
    OffLine,
}

/// `tpAmb`: environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TpAmb {
    /// 1 — production.
    #[default]
    Producao,
    /// 2 — homologation.
    Homologacao,
}

/// `CRT`: tax regime code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Crt {
    /// 1 — Simples Nacional.
    SimplesNacional,
    /// 2 — Simples Nacional, excess of gross revenue.
    SimplesExcessoReceita,
    /// 3 — normal regime.
    #[default]
    RegimeNormal,
    /// 4 — MEI.
    Mei,
}

/// `indPag`: payment indicator (layout 3.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndPag {
    /// 0 — cash.
    Vista,
    /// 1 — instalments.
    Prazo,
    /// 2 — other.
    Outros,
}

/// Address block (`enderEmit`, `enderDest`).
#[derive(Debug, Clone, Default)]
pub struct Address {
    /// Street.
    pub x_lgr: String,
    /// Number.
    pub nro: String,
    /// Complement.
    pub x_cpl: String,
    /// District.
    pub x_bairro: String,
    /// IBGE city code.
    pub c_mun: u32,
    /// City.
    pub x_mun: String,
    /// State (UF).
    pub uf: String,
    /// Postal code (digits as a number).
    pub cep: u32,
    /// BACEN country code.
    pub c_pais: u32,
    /// Country.
    pub x_pais: String,
    /// Phone.
    pub fone: String,
}

/// Issuer.
#[derive(Debug, Clone, Default)]
pub struct Emit {
    /// CNPJ or CPF (digits).
    pub cnpj_cpf: String,
    /// Legal name.
    pub x_nome: String,
    /// Trade name.
    pub x_fant: String,
    /// Address.
    pub ender_emit: Address,
    /// State registration.
    pub ie: String,
    /// Substitute taxpayer state registration.
    pub iest: String,
    /// Municipal registration.
    pub im: String,
    /// Tax regime.
    pub crt: Crt,
}

/// Recipient.
#[derive(Debug, Clone, Default)]
pub struct Dest {
    /// CNPJ or CPF (digits).
    pub cnpj_cpf: String,
    /// Foreign ID.
    pub id_estrangeiro: String,
    /// Name.
    pub x_nome: String,
    /// Address.
    pub ender_dest: Address,
    /// State registration.
    pub ie: String,
    /// SUFRAMA registration.
    pub isuf: String,
    /// E-mail.
    pub email: String,
}

/// Pick-up or delivery location (`retirada`, `entrega`).
#[derive(Debug, Clone, Default)]
pub struct Local {
    /// CNPJ or CPF; empty when the block is absent.
    pub cnpj_cpf: String,
    /// Name.
    pub x_nome: String,
    /// Street.
    pub x_lgr: String,
    /// Number.
    pub nro: String,
    /// Complement.
    pub x_cpl: String,
    /// District.
    pub x_bairro: String,
    /// IBGE city code.
    pub c_mun: u32,
    /// City.
    pub x_mun: String,
    /// State (UF).
    pub uf: String,
    /// Postal code.
    pub cep: u32,
    /// BACEN country code.
    pub c_pais: u32,
    /// Country.
    pub x_pais: String,
    /// Phone.
    pub fone: String,
    /// E-mail.
    pub email: String,
    /// State registration.
    pub ie: String,
}

/// Referenced document (`NFref`); only the members of the kind present are filled.
#[derive(Debug, Clone, Default)]
pub struct NfRef {
    /// `refNFe` access key.
    pub ref_nfe: String,
    /// `refCTe` access key.
    pub ref_cte: String,
    /// `refECF/mod`.
    pub ecf_modelo: String,
    /// `refECF/nECF`.
    pub ecf_n_ecf: String,
    /// `refECF/nCOO`.
    pub ecf_n_coo: String,
    /// `refNF/cUF`.
    pub nf_c_uf: u32,
    /// `refNF/CNPJ`.
    pub nf_cnpj: String,
    /// `refNF/mod`.
    pub nf_modelo: u32,
    /// `refNF/serie`.
    pub nf_serie: u32,
    /// `refNF/nNF`.
    pub nf_n_nf: u64,
    /// `refNFP/cUF`.
    pub nfp_c_uf: u32,
    /// `refNFP/CNPJ` or `CPF`.
    pub nfp_cnpj_cpf: String,
    /// `refNFP/IE`.
    pub nfp_ie: String,
    /// `refNFP/mod`.
    pub nfp_modelo: String,
    /// `refNFP/serie`.
    pub nfp_serie: u32,
    /// `refNFP/nNF`.
    pub nfp_n_nf: u64,
}

/// Identification block.
#[derive(Debug, Clone, Default)]
pub struct Ide {
    /// IBGE state code.
    pub c_uf: u32,
    /// Nature of the operation.
    pub nat_op: String,
    /// Payment indicator (layout 3.10).
    pub ind_pag: Option<IndPag>,
    /// Model (55 or 65).
    pub modelo: u32,
    /// Series.
    pub serie: u32,
    /// Number.
    pub n_nf: u64,
    /// Issue date (`dhEmi` or `dEmi`).
    pub d_emi: Option<DateTime>,
    /// Exit/entry date and time (`dhSaiEnt`, or `dSaiEnt` + `hSaiEnt`).
    pub d_sai_ent: Option<DateTime>,
    /// Operation type.
    pub tp_nf: TpNf,
    /// DANFE format.
    pub tp_imp: TpImp,
    /// Issuing mode.
    pub tp_emis: TpEmis,
    /// Environment.
    pub tp_amb: TpAmb,
    /// Contingency start.
    pub dh_cont: Option<DateTime>,
    /// Contingency reason.
    pub x_just: String,
    /// Referenced documents.
    pub nf_ref: Vec<NfRef>,
}

/// Vehicle details (`veicProd`).
#[derive(Debug, Clone, Default)]
pub struct VeicProd {
    /// Operation type (0 to 3).
    pub tp_op: u32,
    /// Chassis.
    pub chassi: String,
    /// Colour code.
    pub c_cor: String,
    /// Colour name.
    pub x_cor: String,
    /// Engine power.
    pub pot: String,
    /// Displacement.
    pub cilin: String,
    /// Net weight.
    pub peso_l: String,
    /// Gross weight.
    pub peso_b: String,
    /// Serial number.
    pub n_serie: String,
    /// Fuel type code.
    pub tp_comb: String,
    /// Engine number.
    pub n_motor: String,
    /// Maximum traction capacity.
    pub cmt: String,
    /// Wheelbase.
    pub dist: String,
    /// Model year.
    pub ano_mod: u32,
    /// Manufacturing year.
    pub ano_fab: u32,
    /// Paint type.
    pub tp_pint: String,
    /// Vehicle type.
    pub tp_veic: u32,
    /// Vehicle species.
    pub esp_veic: u32,
    /// VIN condition (`R` or `N`).
    pub vin: String,
    /// Vehicle condition.
    pub cond_veic: u32,
    /// Brand/model code.
    pub c_mod: String,
    /// DENATRAN colour code.
    pub c_cor_denatran: String,
    /// Maximum seating capacity.
    pub lota: u32,
    /// Restriction.
    pub tp_rest: u32,
}

/// Medicine details (`med`).
#[derive(Debug, Clone, Default)]
pub struct Med {
    /// Batch.
    pub n_lote: String,
    /// Batch quantity.
    pub q_lote: Decimal,
    /// Manufacturing date.
    pub d_fab: Option<DateTime>,
    /// Expiry date.
    pub d_val: Option<DateTime>,
    /// Maximum consumer price.
    pub v_pmc: Decimal,
    /// ANVISA registration.
    pub c_prod_anvisa: String,
}

/// Firearm details (`arma`).
#[derive(Debug, Clone, Default)]
pub struct Arma {
    /// Type (0 permitted, 1 restricted).
    pub tp_arma: u32,
    /// Weapon serial number.
    pub n_serie: String,
    /// Barrel serial number.
    pub n_cano: String,
    /// Description.
    pub descr: String,
}

/// Fuel details (`comb`).
#[derive(Debug, Clone, Default)]
pub struct Comb {
    /// ANP product code.
    pub c_prod_anp: u64,
    /// CODIF authorisation.
    pub codif: String,
    /// Quantity at ambient temperature.
    pub q_temp: Decimal,
    /// Consumption state.
    pub uf_cons: String,
    /// CIDE tax base quantity.
    pub cide_q_bc_prod: Decimal,
    /// CIDE rate.
    pub cide_v_aliq_prod: Decimal,
    /// CIDE amount.
    pub cide_v_cide: Decimal,
    /// Pump nozzle number.
    pub encerrante_n_bico: u32,
    /// Pump number.
    pub encerrante_n_bomba: u32,
    /// Tank number.
    pub encerrante_n_tanque: u32,
    /// Counter at start.
    pub encerrante_v_enc_ini: Decimal,
    /// Counter at end.
    pub encerrante_v_enc_fin: Decimal,
}

/// Traceability (`rastro`).
#[derive(Debug, Clone, Default)]
pub struct Rastro {
    /// Batch.
    pub n_lote: String,
    /// Batch quantity.
    pub q_lote: Decimal,
    /// Manufacturing date.
    pub d_fab: Option<DateTime>,
    /// Expiry date.
    pub d_val: Option<DateTime>,
    /// Aggregation code.
    pub c_agreg: String,
}

/// Product or service (`prod`).
#[derive(Debug, Clone, Default)]
pub struct Prod {
    /// Code.
    pub c_prod: String,
    /// GTIN (`cEAN`).
    pub c_ean: String,
    /// Description.
    pub x_prod: String,
    /// NCM.
    pub ncm: String,
    /// CFOP.
    pub cfop: String,
    /// Commercial unit.
    pub u_com: String,
    /// Commercial quantity.
    pub q_com: Decimal,
    /// Commercial unit value.
    pub v_un_com: Decimal,
    /// Total value.
    pub v_prod: Decimal,
    /// Taxable unit.
    pub u_trib: String,
    /// Taxable quantity.
    pub q_trib: Decimal,
    /// Taxable unit value.
    pub v_un_trib: Decimal,
    /// Freight.
    pub v_frete: Decimal,
    /// Insurance.
    pub v_seg: Decimal,
    /// Discount.
    pub v_desc: Decimal,
    /// Other expenses.
    pub v_outro: Decimal,
    /// Purchase order.
    pub x_ped: String,
    /// Purchase order item.
    pub n_item_ped: String,
    /// Vehicle details.
    pub veic_prod: VeicProd,
    /// Medicines.
    pub med: Vec<Med>,
    /// Firearms.
    pub arma: Vec<Arma>,
    /// Fuel.
    pub comb: Comb,
    /// Traceability entries.
    pub rastro: Vec<Rastro>,
}

/// ICMS values of an item (whichever `ICMSxx` group it came from).
#[derive(Debug, Clone, Default)]
pub struct Icms {
    /// Origin of the goods.
    pub orig: String,
    /// CST (empty for Simples Nacional groups).
    pub cst: String,
    /// CSOSN (empty for regular groups).
    pub csosn: String,
    /// Tax base.
    pub v_bc: Decimal,
    /// Rate.
    pub p_icms: Decimal,
    /// Amount.
    pub v_icms: Decimal,
    /// ICMS-ST amount.
    pub v_icms_st: Decimal,
    /// Exempted ICMS.
    pub v_icms_deson: Decimal,
}

/// IPI values of an item.
#[derive(Debug, Clone, Default)]
pub struct Ipi {
    /// Amount.
    pub v_ipi: Decimal,
    /// Rate.
    pub p_ipi: Decimal,
}

/// Taxes of an item.
#[derive(Debug, Clone, Default)]
pub struct Imposto {
    /// Approximate total of taxes.
    pub v_tot_trib: Decimal,
    /// ICMS.
    pub icms: Icms,
    /// IPI.
    pub ipi: Ipi,
}

/// Item (`det`).
#[derive(Debug, Clone, Default)]
pub struct Det {
    /// Item number.
    pub n_item: u32,
    /// Product.
    pub prod: Prod,
    /// Taxes.
    pub imposto: Imposto,
    /// Additional product information.
    pub inf_ad_prod: String,
}

/// ICMS totals (`ICMSTot`).
#[derive(Debug, Clone, Default)]
pub struct IcmsTot {
    /// ICMS base.
    pub v_bc: Decimal,
    /// ICMS.
    pub v_icms: Decimal,
    /// Exempted ICMS.
    pub v_icms_deson: Decimal,
    /// FCP of the destination state.
    pub v_fcp_uf_dest: Decimal,
    /// ICMS of the destination state.
    pub v_icms_uf_dest: Decimal,
    /// ICMS of the origin state.
    pub v_icms_uf_remet: Decimal,
    /// ICMS-ST base.
    pub v_bc_st: Decimal,
    /// ICMS-ST.
    pub v_st: Decimal,
    /// Products.
    pub v_prod: Decimal,
    /// Freight.
    pub v_frete: Decimal,
    /// Insurance.
    pub v_seg: Decimal,
    /// Discount.
    pub v_desc: Decimal,
    /// Import tax.
    pub v_ii: Decimal,
    /// IPI.
    pub v_ipi: Decimal,
    /// PIS.
    pub v_pis: Decimal,
    /// COFINS.
    pub v_cofins: Decimal,
    /// Other expenses.
    pub v_outro: Decimal,
    /// Total of the invoice.
    pub v_nf: Decimal,
    /// Approximate total of taxes.
    pub v_tot_trib: Decimal,
}

/// Services totals (`ISSQNtot`).
#[derive(Debug, Clone, Default)]
pub struct IssqnTot {
    /// Services value.
    pub v_serv: Decimal,
    /// ISSQN base.
    pub v_bc: Decimal,
    /// ISSQN.
    pub v_iss: Decimal,
}

/// Tax-reform totals (`IBSCBSTot`, `ISTot`).
#[derive(Debug, Clone, Default)]
pub struct IbsCbsTot {
    /// IBS total (`gIBS/vIBS`).
    pub v_ibs: Decimal,
    /// CBS total (`gCBS/vCBS`).
    pub v_cbs: Decimal,
    /// Selective tax total (`ISTot/vIS`).
    pub v_is: Decimal,
}

/// Totals.
#[derive(Debug, Clone, Default)]
pub struct Total {
    /// ICMS totals.
    pub icms_tot: IcmsTot,
    /// ISSQN totals.
    pub issqn_tot: IssqnTot,
    /// IBS, CBS and IS totals.
    pub ibs_cbs_tot: IbsCbsTot,
}

/// Carrier (`transporta`).
#[derive(Debug, Clone, Default)]
pub struct Transporta {
    /// CNPJ or CPF.
    pub cnpj_cpf: String,
    /// Name.
    pub x_nome: String,
    /// State registration.
    pub ie: String,
    /// Address.
    pub x_ender: String,
    /// City.
    pub x_mun: String,
    /// State.
    pub uf: String,
}

/// Vehicle or trailer (`veicTransp`, `reboque`).
#[derive(Debug, Clone, Default)]
pub struct Veiculo {
    /// Plate.
    pub placa: String,
    /// State.
    pub uf: String,
    /// ANTT code.
    pub rntc: String,
}

/// Volume (`vol`).
#[derive(Debug, Clone, Default)]
pub struct Vol {
    /// Quantity.
    pub q_vol: i64,
    /// Species.
    pub esp: String,
    /// Brand.
    pub marca: String,
    /// Numbering.
    pub n_vol: String,
    /// Net weight.
    pub peso_l: Decimal,
    /// Gross weight.
    pub peso_b: Decimal,
}

/// Transport block.
#[derive(Debug, Clone, Default)]
pub struct Transp {
    /// Freight modality code (0..4, 9).
    pub mod_frete: u32,
    /// Carrier.
    pub transporta: Transporta,
    /// Vehicle.
    pub veic_transp: Veiculo,
    /// Trailers.
    pub reboque: Vec<Veiculo>,
    /// Volumes.
    pub vol: Vec<Vol>,
}

/// Invoice (`fat`).
#[derive(Debug, Clone, Default)]
pub struct Fat {
    /// Number.
    pub n_fat: String,
    /// Original value.
    pub v_orig: Decimal,
    /// Discount.
    pub v_desc: Decimal,
    /// Net value.
    pub v_liq: Decimal,
}

/// Instalment (`dup`).
#[derive(Debug, Clone, Default)]
pub struct Dup {
    /// Number.
    pub n_dup: String,
    /// Due date.
    pub d_venc: Option<DateTime>,
    /// Value.
    pub v_dup: Decimal,
}

/// Billing block.
#[derive(Debug, Clone, Default)]
pub struct Cobr {
    /// Invoice.
    pub fat: Fat,
    /// Instalments.
    pub dup: Vec<Dup>,
}

/// Payment (`detPag`).
#[derive(Debug, Clone, Default)]
pub struct DetPag {
    /// Payment indicator.
    pub ind_pag: String,
    /// Payment means code (`tPag`).
    pub t_pag: String,
    /// Description when `tPag` is `99`.
    pub x_pag: String,
    /// Value.
    pub v_pag: Decimal,
}

/// A `field: text` note (`obsCont`, `obsFisco`).
#[derive(Debug, Clone, Default)]
pub struct Obs {
    /// Field name.
    pub x_campo: String,
    /// Text.
    pub x_texto: String,
}

/// Referenced process (`procRef`).
#[derive(Debug, Clone, Default)]
pub struct ProcRef {
    /// Process number.
    pub n_proc: String,
    /// Origin indicator (0..4, 9).
    pub ind_proc: String,
}

/// Additional information.
#[derive(Debug, Clone, Default)]
pub struct InfAdic {
    /// For the tax authority.
    pub inf_ad_fisco: String,
    /// For the taxpayer.
    pub inf_cpl: String,
    /// Taxpayer notes.
    pub obs_cont: Vec<Obs>,
    /// Tax authority notes.
    pub obs_fisco: Vec<Obs>,
    /// Referenced processes.
    pub proc_ref: Vec<ProcRef>,
}

/// Authorisation protocol (`protNFe/infProt`).
#[derive(Debug, Clone, Default)]
pub struct ProcNfe {
    /// Environment.
    pub tp_amb: TpAmb,
    /// Access key.
    pub ch_nfe: String,
    /// Reception date and time.
    pub dh_recbto: Option<DateTime>,
    /// Protocol number.
    pub n_prot: String,
    /// Digest value.
    pub dig_val: String,
    /// Status code.
    pub c_stat: u32,
    /// Status description.
    pub x_motivo: String,
}

/// Supplementary data of the NFC-e (`infNFeSupl`).
#[derive(Debug, Clone, Default)]
pub struct InfNfeSupl {
    /// QR Code content.
    pub qr_code: String,
    /// Consultation URL.
    pub url_chave: String,
}

/// An NF-e (or NFC-e) with its authorisation protocol, if any.
#[derive(Debug, Clone, Default)]
pub struct Nfe {
    /// `infNFe/@Id`, including the `NFe` prefix.
    pub id: String,
    /// Layout version (`infNFe/@versao`).
    pub versao: f64,
    /// Identification.
    pub ide: Ide,
    /// Issuer.
    pub emit: Emit,
    /// Recipient.
    pub dest: Dest,
    /// Pick-up location.
    pub retirada: Local,
    /// Delivery location.
    pub entrega: Local,
    /// Items.
    pub det: Vec<Det>,
    /// Totals.
    pub total: Total,
    /// Transport.
    pub transp: Transp,
    /// Billing.
    pub cobr: Cobr,
    /// Payments.
    pub pag: Vec<DetPag>,
    /// Change given back (`pag/vTroco`).
    pub pag_v_troco: Decimal,
    /// Additional information.
    pub inf_adic: InfAdic,
    /// Authorisation protocol.
    pub proc_nfe: ProcNfe,
    /// NFC-e supplementary data.
    pub inf_nfe_supl: InfNfeSupl,
}

impl Nfe {
    /// The 44-digit access key (the `Id` without the `NFe` prefix).
    pub fn chave(&self) -> String {
        remove_literal_chave(&self.id)
    }
}

/// Strips the `NFe`/`CFe` literal that prefixes an access key (`RemoverLiteralChave`).
pub fn remove_literal_chave(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    for i in 0..chars.len() {
        if chars[i].is_ascii_digit() && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
            return chars[i..].iter().collect();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dates_and_times() {
        let d = DateTime::parse("2026-07-20T10:33:28-04:00").expect("date");
        assert_eq!(
            (d.year, d.month, d.day, d.hour, d.minute, d.second),
            (2026, 7, 20, 10, 33, 28)
        );
        assert_eq!(d.date_br(), "20/07/2026");
        assert_eq!(d.datetime_br(), "20/07/2026 10:33:28");
        let d = DateTime::parse("2026-07-20").expect("date");
        assert!(d.time_is_zero());
        assert_eq!(
            DateTime::parse("2026-07-20T10:33:28Z")
                .map(|d| d.time_string())
                .as_deref(),
            Some("10:33:28")
        );
        assert_eq!(
            DateTime::parse("2026-07-20T10:33:28.123+01:00").map(|d| d.second),
            Some(28)
        );
        assert!(DateTime::parse("20/07/2026").is_none());
        assert!(DateTime::parse("2026-13-01").is_none());
    }

    #[test]
    fn removes_the_key_literal() {
        assert_eq!(
            remove_literal_chave("NFe51260793209765056503551010000769171752265105"),
            "51260793209765056503551010000769171752265105"
        );
        assert_eq!(remove_literal_chave("5126"), "5126");
        assert_eq!(remove_literal_chave("NFe"), "");
    }
}
