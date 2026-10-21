//! Reading an NF-e XML (`nfeProc` or a bare `NFe`) into the [`Nfe`] model.

use crate::decimal::Decimal;
use crate::error::DanfeError;
use crate::model::*;
use crate::xml::{self, Node};

fn dec(node: &Node, name: &str) -> Decimal {
    Decimal::parse(node.text_of(name)).unwrap_or(Decimal::ZERO)
}

fn num<T: std::str::FromStr + Default>(node: &Node, name: &str) -> T {
    node.text_of(name).parse().unwrap_or_default()
}

fn text(node: &Node, name: &str) -> String {
    node.text_of(name).to_owned()
}

fn date(node: &Node, name: &str) -> Option<DateTime> {
    DateTime::parse(node.text_of(name))
}

/// CNPJ, falling back to CPF.
fn cnpj_cpf(node: &Node) -> String {
    let cnpj = node.text_of("CNPJ");
    if cnpj.is_empty() {
        text(node, "CPF")
    } else {
        cnpj.to_owned()
    }
}

fn address(node: Option<&Node>, tag: &str) -> Address {
    let Some(a) = node.and_then(|n| n.child(tag)) else {
        return Address::default();
    };
    Address {
        x_lgr: text(a, "xLgr"),
        nro: text(a, "nro"),
        x_cpl: text(a, "xCpl"),
        x_bairro: text(a, "xBairro"),
        c_mun: num(a, "cMun"),
        x_mun: text(a, "xMun"),
        uf: text(a, "UF"),
        cep: num(a, "CEP"),
        c_pais: num(a, "cPais"),
        x_pais: text(a, "xPais"),
        fone: text(a, "fone"),
    }
}

fn local(node: Option<&Node>) -> Local {
    let Some(l) = node else {
        return Local::default();
    };
    Local {
        cnpj_cpf: cnpj_cpf(l),
        x_nome: text(l, "xNome"),
        x_lgr: text(l, "xLgr"),
        nro: text(l, "nro"),
        x_cpl: text(l, "xCpl"),
        x_bairro: text(l, "xBairro"),
        c_mun: num(l, "cMun"),
        x_mun: text(l, "xMun"),
        uf: text(l, "UF"),
        cep: num(l, "CEP"),
        c_pais: num(l, "cPais"),
        x_pais: text(l, "xPais"),
        fone: text(l, "fone"),
        email: text(l, "email"),
        ie: text(l, "IE"),
    }
}

fn read_ide(ide: &Node) -> Ide {
    // Layout 2.00 has `dEmi`, `dSaiEnt` and `hSaiEnt`; later ones have date-times.
    let d_emi = date(ide, "dhEmi").or_else(|| date(ide, "dEmi"));
    let d_sai_ent = date(ide, "dhSaiEnt").or_else(|| {
        let mut d = date(ide, "dSaiEnt")?;
        if let Some(t) = DateTime::parse(&format!("2000-01-01T{}", ide.text_of("hSaiEnt"))) {
            (d.hour, d.minute, d.second) = (t.hour, t.minute, t.second);
        }
        Some(d)
    });
    Ide {
        c_uf: num(ide, "cUF"),
        nat_op: text(ide, "natOp"),
        ind_pag: match ide.text_of("indPag") {
            "0" => Some(IndPag::Vista),
            "1" => Some(IndPag::Prazo),
            "2" => Some(IndPag::Outros),
            _ => None,
        },
        modelo: num(ide, "mod"),
        serie: num(ide, "serie"),
        n_nf: num(ide, "nNF"),
        d_emi,
        d_sai_ent,
        tp_nf: if ide.text_of("tpNF") == "0" {
            TpNf::Entrada
        } else {
            TpNf::Saida
        },
        tp_imp: match ide.text_of("tpImp") {
            "0" => TpImp::SemGeracao,
            "2" => TpImp::Paisagem,
            "3" => TpImp::Simplificado,
            "4" => TpImp::NfCe,
            "5" => TpImp::MsgEletronica,
            "6" => TpImp::SimplificadoTipo2,
            _ => TpImp::Retrato,
        },
        tp_emis: match ide.text_of("tpEmis") {
            "2" => TpEmis::Contingencia,
            "3" => TpEmis::Scan,
            "4" => TpEmis::Dpec,
            "5" => TpEmis::Fsda,
            "6" => TpEmis::SvcAn,
            "7" => TpEmis::SvcRs,
            "8" => TpEmis::SvcSp,
            "9" => TpEmis::OffLine,
            _ => TpEmis::Normal,
        },
        tp_amb: if ide.text_of("tpAmb") == "2" {
            TpAmb::Homologacao
        } else {
            TpAmb::Producao
        },
        dh_cont: date(ide, "dhCont"),
        x_just: text(ide, "xJust"),
        nf_ref: ide.children_named("NFref").map(read_nf_ref).collect(),
    }
}

fn read_nf_ref(r: &Node) -> NfRef {
    let mut out = NfRef {
        ref_nfe: text(r, "refNFe"),
        ref_cte: text(r, "refCTe"),
        ..NfRef::default()
    };
    if let Some(e) = r.child("refECF") {
        out.ecf_modelo = text(e, "mod");
        out.ecf_n_ecf = text(e, "nECF");
        out.ecf_n_coo = text(e, "nCOO");
    }
    if let Some(n) = r.child("refNF") {
        out.nf_c_uf = num(n, "cUF");
        out.nf_cnpj = text(n, "CNPJ");
        out.nf_modelo = num(n, "mod");
        out.nf_serie = num(n, "serie");
        out.nf_n_nf = num(n, "nNF");
    }
    if let Some(n) = r.child("refNFP") {
        out.nfp_c_uf = num(n, "cUF");
        out.nfp_cnpj_cpf = cnpj_cpf(n);
        out.nfp_ie = text(n, "IE");
        out.nfp_modelo = text(n, "mod");
        out.nfp_serie = num(n, "serie");
        out.nfp_n_nf = num(n, "nNF");
    }
    out
}

fn read_emit(emit: Option<&Node>) -> Emit {
    let Some(e) = emit else {
        return Emit::default();
    };
    Emit {
        cnpj_cpf: cnpj_cpf(e),
        x_nome: text(e, "xNome"),
        x_fant: text(e, "xFant"),
        ender_emit: address(Some(e), "enderEmit"),
        ie: text(e, "IE"),
        iest: text(e, "IEST"),
        im: text(e, "IM"),
        crt: match e.text_of("CRT") {
            "1" => Crt::SimplesNacional,
            "2" => Crt::SimplesExcessoReceita,
            "4" => Crt::Mei,
            _ => Crt::RegimeNormal,
        },
    }
}

fn read_dest(dest: Option<&Node>) -> Dest {
    let Some(d) = dest else {
        return Dest::default();
    };
    Dest {
        cnpj_cpf: cnpj_cpf(d),
        id_estrangeiro: text(d, "idEstrangeiro"),
        x_nome: text(d, "xNome"),
        ender_dest: address(Some(d), "enderDest"),
        ie: text(d, "IE"),
        isuf: text(d, "ISUF"),
        email: text(d, "email"),
    }
}

fn read_veic_prod(v: Option<&Node>) -> VeicProd {
    let Some(v) = v else {
        return VeicProd::default();
    };
    VeicProd {
        tp_op: num(v, "tpOp"),
        chassi: text(v, "chassi"),
        c_cor: text(v, "cCor"),
        x_cor: text(v, "xCor"),
        pot: text(v, "pot"),
        cilin: text(v, "cilin"),
        peso_l: text(v, "pesoL"),
        peso_b: text(v, "pesoB"),
        n_serie: text(v, "nSerie"),
        tp_comb: text(v, "tpComb"),
        n_motor: text(v, "nMotor"),
        cmt: text(v, "CMT"),
        dist: text(v, "dist"),
        ano_mod: num(v, "anoMod"),
        ano_fab: num(v, "anoFab"),
        tp_pint: text(v, "tpPint"),
        tp_veic: num(v, "tpVeic"),
        esp_veic: num(v, "espVeic"),
        vin: text(v, "VIN"),
        cond_veic: num(v, "condVeic"),
        c_mod: text(v, "cMod"),
        c_cor_denatran: text(v, "cCorDENATRAN"),
        lota: num(v, "lota"),
        tp_rest: num(v, "tpRest"),
    }
}

fn read_comb(c: Option<&Node>) -> Comb {
    let Some(c) = c else {
        return Comb::default();
    };
    let cide = c.child("CIDE");
    let enc = c.child("encerrante");
    Comb {
        c_prod_anp: num(c, "cProdANP"),
        codif: text(c, "CODIF"),
        q_temp: dec(c, "qTemp"),
        uf_cons: text(c, "UFCons"),
        cide_q_bc_prod: cide.map_or(Decimal::ZERO, |n| dec(n, "qBCProd")),
        cide_v_aliq_prod: cide.map_or(Decimal::ZERO, |n| dec(n, "vAliqProd")),
        cide_v_cide: cide.map_or(Decimal::ZERO, |n| dec(n, "vCIDE")),
        encerrante_n_bico: enc.map_or(0, |n| num(n, "nBico")),
        encerrante_n_bomba: enc.map_or(0, |n| num(n, "nBomba")),
        encerrante_n_tanque: enc.map_or(0, |n| num(n, "nTanque")),
        encerrante_v_enc_ini: enc.map_or(Decimal::ZERO, |n| dec(n, "vEncIni")),
        encerrante_v_enc_fin: enc.map_or(Decimal::ZERO, |n| dec(n, "vEncFin")),
    }
}

fn read_prod(p: &Node) -> Prod {
    Prod {
        c_prod: text(p, "cProd"),
        c_ean: text(p, "cEAN"),
        x_prod: text(p, "xProd"),
        ncm: text(p, "NCM"),
        cfop: text(p, "CFOP"),
        u_com: text(p, "uCom"),
        q_com: dec(p, "qCom"),
        v_un_com: dec(p, "vUnCom"),
        v_prod: dec(p, "vProd"),
        u_trib: text(p, "uTrib"),
        q_trib: dec(p, "qTrib"),
        v_un_trib: dec(p, "vUnTrib"),
        v_frete: dec(p, "vFrete"),
        v_seg: dec(p, "vSeg"),
        v_desc: dec(p, "vDesc"),
        v_outro: dec(p, "vOutro"),
        x_ped: text(p, "xPed"),
        n_item_ped: text(p, "nItemPed"),
        veic_prod: read_veic_prod(p.child("veicProd")),
        med: p
            .children_named("med")
            .map(|m| Med {
                n_lote: text(m, "nLote"),
                q_lote: dec(m, "qLote"),
                d_fab: date(m, "dFab"),
                d_val: date(m, "dVal"),
                v_pmc: dec(m, "vPMC"),
                c_prod_anvisa: text(m, "cProdANVISA"),
            })
            .collect(),
        arma: p
            .children_named("arma")
            .map(|a| Arma {
                tp_arma: num(a, "tpArma"),
                n_serie: text(a, "nSerie"),
                n_cano: text(a, "nCano"),
                descr: text(a, "descr"),
            })
            .collect(),
        comb: read_comb(p.child("comb")),
        rastro: p
            .children_named("rastro")
            .map(|r| Rastro {
                n_lote: text(r, "nLote"),
                q_lote: dec(r, "qLote"),
                d_fab: date(r, "dFab"),
                d_val: date(r, "dVal"),
                c_agreg: text(r, "cAgreg"),
            })
            .collect(),
    }
}

fn read_imposto(imp: Option<&Node>) -> Imposto {
    let Some(imp) = imp else {
        return Imposto::default();
    };
    // Whichever ICMS group the item uses (ICMS00, ICMS60, ICMSSN101...).
    let icms = imp
        .child("ICMS")
        .and_then(|i| i.children.first())
        .map(|g| Icms {
            orig: text(g, "orig"),
            cst: text(g, "CST"),
            csosn: text(g, "CSOSN"),
            v_bc: dec(g, "vBC"),
            p_icms: dec(g, "pICMS"),
            v_icms: dec(g, "vICMS"),
            v_icms_st: dec(g, "vICMSST"),
            v_icms_deson: dec(g, "vICMSDeson"),
        })
        .unwrap_or_default();
    let ipi = imp
        .find("IPI/IPITrib")
        .map(|i| Ipi {
            v_ipi: dec(i, "vIPI"),
            p_ipi: dec(i, "pIPI"),
        })
        .unwrap_or_default();
    Imposto {
        v_tot_trib: dec(imp, "vTotTrib"),
        icms,
        ipi,
    }
}

fn read_total(total: Option<&Node>) -> Total {
    let Some(t) = total else {
        return Total::default();
    };
    let icms_tot = t.child("ICMSTot").map(|n| IcmsTot {
        v_bc: dec(n, "vBC"),
        v_icms: dec(n, "vICMS"),
        v_icms_deson: dec(n, "vICMSDeson"),
        v_fcp_uf_dest: dec(n, "vFCPUFDest"),
        v_icms_uf_dest: dec(n, "vICMSUFDest"),
        v_icms_uf_remet: dec(n, "vICMSUFRemet"),
        v_bc_st: dec(n, "vBCST"),
        v_st: dec(n, "vST"),
        v_prod: dec(n, "vProd"),
        v_frete: dec(n, "vFrete"),
        v_seg: dec(n, "vSeg"),
        v_desc: dec(n, "vDesc"),
        v_ii: dec(n, "vII"),
        v_ipi: dec(n, "vIPI"),
        v_pis: dec(n, "vPIS"),
        v_cofins: dec(n, "vCOFINS"),
        v_outro: dec(n, "vOutro"),
        v_nf: dec(n, "vNF"),
        v_tot_trib: dec(n, "vTotTrib"),
    });
    let issqn_tot = t.child("ISSQNtot").map(|n| IssqnTot {
        v_serv: dec(n, "vServ"),
        v_bc: dec(n, "vBC"),
        v_iss: dec(n, "vISS"),
    });
    let amount = |path: &str| {
        t.find(path)
            .and_then(|n| Decimal::parse(n.text.trim()))
            .unwrap_or(Decimal::ZERO)
    };
    Total {
        icms_tot: icms_tot.unwrap_or_default(),
        issqn_tot: issqn_tot.unwrap_or_default(),
        ibs_cbs_tot: IbsCbsTot {
            v_ibs: amount("IBSCBSTot/gIBS/vIBS"),
            v_cbs: amount("IBSCBSTot/gCBS/vCBS"),
            v_is: amount("ISTot/vIS"),
        },
    }
}

fn veiculo(n: &Node) -> Veiculo {
    Veiculo {
        placa: text(n, "placa"),
        uf: text(n, "UF"),
        rntc: text(n, "RNTC"),
    }
}

fn read_transp(transp: Option<&Node>) -> Transp {
    let Some(t) = transp else {
        return Transp {
            mod_frete: 9,
            ..Transp::default()
        };
    };
    Transp {
        mod_frete: t.text_of("modFrete").parse().unwrap_or(0),
        transporta: t
            .child("transporta")
            .map(|n| Transporta {
                cnpj_cpf: cnpj_cpf(n),
                x_nome: text(n, "xNome"),
                ie: text(n, "IE"),
                x_ender: text(n, "xEnder"),
                x_mun: text(n, "xMun"),
                uf: text(n, "UF"),
            })
            .unwrap_or_default(),
        veic_transp: t.child("veicTransp").map(veiculo).unwrap_or_default(),
        reboque: t.children_named("reboque").map(veiculo).collect(),
        vol: t
            .children_named("vol")
            .map(|v| Vol {
                q_vol: num(v, "qVol"),
                esp: text(v, "esp"),
                marca: text(v, "marca"),
                n_vol: text(v, "nVol"),
                peso_l: dec(v, "pesoL"),
                peso_b: dec(v, "pesoB"),
            })
            .collect(),
    }
}

fn read_cobr(cobr: Option<&Node>) -> Cobr {
    let Some(c) = cobr else {
        return Cobr::default();
    };
    Cobr {
        fat: c
            .child("fat")
            .map(|f| Fat {
                n_fat: text(f, "nFat"),
                v_orig: dec(f, "vOrig"),
                v_desc: dec(f, "vDesc"),
                v_liq: dec(f, "vLiq"),
            })
            .unwrap_or_default(),
        dup: c
            .children_named("dup")
            .map(|d| Dup {
                n_dup: text(d, "nDup"),
                d_venc: date(d, "dVenc"),
                v_dup: dec(d, "vDup"),
            })
            .collect(),
    }
}

fn det_pag(n: &Node) -> DetPag {
    DetPag {
        ind_pag: text(n, "indPag"),
        t_pag: text(n, "tPag"),
        x_pag: text(n, "xPag"),
        v_pag: dec(n, "vPag"),
    }
}

/// Layout 4.00 lists `detPag` inside `pag`; layout 3.10 puts `tPag`/`vPag` in `pag` itself.
fn read_pag(nfe_inf: &Node) -> Vec<DetPag> {
    let mut out = Vec::new();
    for pag in nfe_inf.children_named("pag") {
        let details: Vec<DetPag> = pag.children_named("detPag").map(det_pag).collect();
        if details.is_empty() {
            if pag.child("tPag").is_some() {
                out.push(det_pag(pag));
            }
        } else {
            out.extend(details);
        }
    }
    out
}

fn read_obs(n: &Node, tag: &str) -> Vec<Obs> {
    n.children_named(tag)
        .map(|o| Obs {
            x_campo: o.attr("xCampo").unwrap_or("").to_owned(),
            x_texto: text(o, "xTexto"),
        })
        .collect()
}

fn read_inf_adic(inf: Option<&Node>) -> InfAdic {
    let Some(i) = inf else {
        return InfAdic::default();
    };
    InfAdic {
        inf_ad_fisco: text(i, "infAdFisco"),
        // The text is kept as written (only trimmed at the ends by `text`).
        inf_cpl: text(i, "infCpl"),
        obs_cont: read_obs(i, "obsCont"),
        obs_fisco: read_obs(i, "obsFisco"),
        proc_ref: i
            .children_named("procRef")
            .map(|p| ProcRef {
                n_proc: text(p, "nProc"),
                ind_proc: text(p, "indProc"),
            })
            .collect(),
    }
}

fn read_prot(prot: Option<&Node>) -> ProcNfe {
    let Some(p) = prot.and_then(|n| n.child("infProt")) else {
        return ProcNfe::default();
    };
    ProcNfe {
        tp_amb: if p.text_of("tpAmb") == "2" {
            TpAmb::Homologacao
        } else {
            TpAmb::Producao
        },
        ch_nfe: text(p, "chNFe"),
        dh_recbto: date(p, "dhRecbto"),
        n_prot: text(p, "nProt"),
        dig_val: text(p, "digVal"),
        c_stat: num(p, "cStat"),
        x_motivo: text(p, "xMotivo"),
    }
}

impl Nfe {
    /// Reads an `nfeProc` (NF-e with protocol) or a bare `NFe` document.
    pub fn from_xml(xml: &str) -> Result<Self, DanfeError> {
        let root = xml::parse(xml.trim_start_matches('\u{feff}'))?;
        let (nfe_node, prot) = match root.name.as_str() {
            "nfeProc" | "procNFe" => (root.child("NFe"), root.child("protNFe")),
            "NFe" => (Some(&root), None),
            other => {
                return Err(DanfeError::InvalidNfe(format!(
                    "unexpected root element <{other}>, expected <nfeProc> or <NFe>"
                )));
            }
        };
        let nfe_node =
            nfe_node.ok_or_else(|| DanfeError::InvalidNfe("missing <NFe>".to_owned()))?;
        let inf = nfe_node
            .child("infNFe")
            .ok_or_else(|| DanfeError::InvalidNfe("missing <infNFe>".to_owned()))?;
        let ide = inf
            .child("ide")
            .ok_or_else(|| DanfeError::InvalidNfe("missing <ide>".to_owned()))?;
        let supl = nfe_node.child("infNFeSupl");

        Ok(Self {
            id: inf.attr("Id").unwrap_or("").to_owned(),
            versao: inf
                .attr("versao")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            ide: read_ide(ide),
            emit: read_emit(inf.child("emit")),
            dest: read_dest(inf.child("dest")),
            retirada: local(inf.child("retirada")),
            entrega: local(inf.child("entrega")),
            det: inf
                .children_named("det")
                .map(|d| Det {
                    n_item: d.attr("nItem").and_then(|v| v.parse().ok()).unwrap_or(0),
                    prod: d.child("prod").map(read_prod).unwrap_or_default(),
                    imposto: read_imposto(d.child("imposto")),
                    inf_ad_prod: text(d, "infAdProd"),
                })
                .collect(),
            total: read_total(inf.child("total")),
            transp: read_transp(inf.child("transp")),
            cobr: read_cobr(inf.child("cobr")),
            pag: read_pag(inf),
            pag_v_troco: inf.child("pag").map_or(Decimal::ZERO, |p| dec(p, "vTroco")),
            inf_adic: read_inf_adic(inf.child("infAdic")),
            proc_nfe: read_prot(prot),
            inf_nfe_supl: InfNfeSupl {
                qr_code: supl.map(|s| text(s, "qrCode")).unwrap_or_default(),
                url_chave: supl.map(|s| text(s, "urlChave")).unwrap_or_default(),
            },
        })
    }

    /// Reads an NF-e from raw bytes: UTF-8 (with or without BOM) or, failing that,
    /// Windows-1252 (the encoding of many legacy files).
    pub fn from_xml_bytes(bytes: &[u8]) -> Result<Self, DanfeError> {
        match std::str::from_utf8(bytes) {
            Ok(s) => Self::from_xml(s),
            Err(_) => {
                let decoded: String = bytes.iter().map(|b| cp1252_to_char(*b)).collect();
                Self::from_xml(&decoded)
            }
        }
    }
}

/// Windows-1252 byte to character.
fn cp1252_to_char(b: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9F => HIGH[usize::from(b - 0x80)],
        _ => char::from(b),
    }
}
