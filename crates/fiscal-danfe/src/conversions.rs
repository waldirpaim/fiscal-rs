//! Code-to-description tables of the NF-e layout (payment means, freight modality, vehicle
//! attributes...).

/// `FormaPagamentoToDescricao`: description of the payment means code `tPag`; for `99` the
/// free text `xPag` is used when present.
pub fn forma_pagamento(t_pag: &str, x_pag: &str) -> String {
    if t_pag == "99" && !x_pag.is_empty() {
        return x_pag.to_owned();
    }
    let text = match t_pag {
        "01" => "Dinheiro",
        "02" => "Cheque",
        "03" => "Cartão de Crédito",
        "04" => "Cartão de Débito",
        "05" => "Cartão da Loja (Private Label)",
        "10" => "Vale Alimentação",
        "11" => "Vale Refeição",
        "12" => "Vale Presente",
        "13" => "Vale Combustível",
        "14" => "Duplicata Mercantil",
        "15" => "Boleto Bancário",
        "16" => "Deposito Bancário",
        "17" => "PIX - Dinâmico",
        "18" => "Transferência Bancária",
        "19" => "Programa Fidelidade",
        "90" => "Sem Pagamento",
        "98" => "Regime Especial NFF",
        "99" => "Outro",
        "20" => "PIX - Estático",
        "21" => "Crédito em Loja",
        "22" => "Falha de hardware do sistema emissor",
        "91" => "Pagamento Posterior",
        "23" => "PIX - Automatico",
        "24" => "TEF - Book Transfer",
        // Unknown codes fall back to the first entry, as the original lookup does.
        _ => "Dinheiro",
    };
    text.to_owned()
}

/// `modFreteToDesStr`: short description of the freight modality for a layout version.
pub fn mod_frete(code: u32, versao: f64) -> &'static str {
    if versao >= 4.0 {
        match code {
            0 => "0 - REMETENTE",
            1 => "1 - DESTINATARIO",
            2 => "2 - TERCEIROS",
            3 => "3 - PROP/REMT",
            4 => "4 - PROP/DEST",
            9 => "9 - SEM FRETE",
            _ => "",
        }
    } else if versao >= 2.0 {
        match code {
            0 => "0 - EMITENTE",
            1 => "1 - DEST/REM",
            2 => "2 - TERCEIROS",
            3 => "3 - PROP/REMT",
            4 => "4 - PROP/DEST",
            9 => "9 - SEM FRETE",
            _ => "",
        }
    } else {
        ""
    }
}

/// `VeiculosTipoOperStr`.
pub fn veiculo_tipo_operacao(tp_op: u32) -> &'static str {
    match tp_op {
        1 => "1-VENDA CONCESSIONÁRIA",
        2 => "2-FAT. DIRETO CONS. FINAL",
        3 => "3-VENDA DIRETA",
        0 => "0-OUTROS",
        _ => "",
    }
}

/// `VeiculosCondicaoStr`.
pub fn veiculo_condicao(cond: u32) -> &'static str {
    match cond {
        1 => "1-ACABADO",
        2 => "2-INACABADO",
        3 => "3-SEMI-ACABADO",
        _ => "",
    }
}

/// `VeiculosVinStr`.
pub fn veiculo_vin(vin: &str) -> &'static str {
    match vin {
        "R" => "R-REMARCADO",
        "N" => "N-NORMAL",
        _ => "NÃO DEFINIDA",
    }
}

/// `VeiculosEspecieStr`.
pub fn veiculo_especie(code: u32) -> String {
    let text = match code {
        1 => "01-PASSAGEIRO",
        2 => "02-CARGA",
        3 => "03-MISTO",
        4 => "04-CORRIDA",
        5 => "05-TRAÇÃO",
        6 => "06-ESPECIAL",
        7 => "07-COLEÇÃO",
        _ => return format!("{code}NÃO DEFINIDA"),
    };
    text.to_owned()
}

/// `VeiculosTipoStr`.
pub fn veiculo_tipo(code: u32) -> String {
    let text = match code {
        1 => "01-BICICLETA",
        2 => "02-CICLOMOTOR",
        3 => "03-MOTONETA",
        4 => "04-MOTOCICLETA",
        5 => "05-TRICICLO",
        6 => "06-AUTOMÓVEL",
        7 => "07-MICROONIBUS",
        8 => "08-ONIBUS",
        9 => "09-BONDE",
        10 => "10-REBOQUE",
        11 => "11-SEMI-REBOQUE",
        12 => "12-CHARRETE",
        13 => "13-CAMIONETA",
        14 => "14-CAMINHÃO",
        15 => "15-CARROÇA",
        16 => "16-CARRO DE MÃO",
        17 => "17-CAMINHÃO TRATOR",
        18 => "18-TRATOR DE RODAS",
        19 => "19-TRATOR DE ESTEIRAS",
        20 => "20-TRATOR MISTO",
        21 => "21-QUADRICICLO",
        22 => "22-CHASSI/PLATAFORMA",
        23 => "23-CAMINHONETE",
        24 => "24-SIDE-CAR",
        25 => "25-UTILITÁRIO",
        26 => "26-MOTOR-CASA",
        _ => return format!("{code}NÃO DEFINIDO"),
    };
    text.to_owned()
}

/// `VeiculosCombustivelStr`.
pub fn veiculo_combustivel(code: &str) -> String {
    let n: u32 = code.trim().parse().unwrap_or(0);
    let text = match n {
        1 => "01-ÁLCOOL",
        2 => "02-GASOLINA",
        3 => "03-DIESEL",
        4 => "04-GASOGÊNIO",
        5 => "05-GÁS METANO",
        6 => "06-ELETRICO/F INTERNA",
        7 => "07-ELETRICO/F EXTERNA",
        8 => "08-GASOLINA/GNC",
        9 => "09-ÁLCOOL/GNC",
        10 => "10-DIESEL / GNC",
        11 => "11-VIDE CAMPO OBSERVAÇÃO",
        12 => "12-ÁLCOOL/GNV",
        13 => "13-GASOLINA/GNV",
        14 => "14-DIESEL/GNV",
        15 => "15-GÁS NATURAL VEICULAR",
        16 => "16-ÁLCOOL/GASOLINA",
        17 => "17-GASOLINA/ÁLCOOL/GNV",
        18 => "18-GASOLINA/ELÉTRICO",
        19 => "19-GASOLINA/ÁLCOOL/ELÉTRICO",
        20 => "20-GÁS/NATURAL/LIQUEFEITO",
        21 => "21-DIESEL/ELÉTRICO",
        22 => "22-HÍBRIDO",
        23 => "23-HÍBRIDO PLUG-IN",
        24 => "24-ELÉTRICO",
        25 => "25-CÉLULA COMBUSTÍVEL",
        26 => "26-HÍBRIDO/GÁS NATURAL VEICULAR",
        27 => "27-ETANOL/ELÉTRICO",
        _ => return format!("{code}NÃO DEFINIDO"),
    };
    text.to_owned()
}

/// `VeiculosCorDENATRANStr`.
pub fn veiculo_cor_denatran(code: &str) -> String {
    let n: u32 = code.trim().parse().unwrap_or(0);
    let text = match n {
        1 => "01-AMARELO",
        2 => "02-AZUL",
        3 => "03-BEGE",
        4 => "04-BRANCA",
        5 => "05-CINZA",
        6 => "06-DOURADA",
        7 => "07-GRENÁ",
        8 => "08-LARANJA",
        9 => "09-MARROM",
        10 => "10-PRATA",
        11 => "11-PRETA",
        12 => "12-ROSA",
        13 => "13-ROXA",
        14 => "14-VERDE",
        15 => "15-VERMELHA",
        16 => "16-FANTASIA",
        _ => return format!("{code}NÃO DEFINIDA"),
    };
    text.to_owned()
}

/// `VeiculosRestricaoStr`.
pub fn veiculo_restricao(code: u32) -> String {
    let text = match code {
        0 => "0-NÃO HÁ",
        1 => "1-ALIENAÇÃO FIDUCIÁRIA",
        2 => "2-RESERVA DE DOMICÍLIO",
        3 => "3-RESERVA DE DOMÍNIO",
        4 => "4-PENHOR DE VEÍCULOS",
        9 => "9-OUTRAS",
        _ => return format!("{code}NÃO DEFINIDO"),
    };
    text.to_owned()
}

/// `ArmaTipoStr`.
pub fn arma_tipo(tp_arma: u32) -> &'static str {
    match tp_arma {
        0 => "0-USO PERMITIDO",
        1 => "1-USO RESTRITO",
        _ => "",
    }
}

/// `indProcToDescrStr`.
pub fn ind_proc(code: &str) -> &'static str {
    match code {
        "0" => "SEFAZ",
        "1" => "JUSTIÇA FEDERAL",
        "2" => "JUSTIÇA ESTADUAL",
        "3" => "SECEX / RFB",
        "4" => "CONFAZ",
        "9" => "OUTROS",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_descriptions() {
        assert_eq!(forma_pagamento("01", ""), "Dinheiro");
        assert_eq!(forma_pagamento("15", ""), "Boleto Bancário");
        assert_eq!(forma_pagamento("99", "Cripto"), "Cripto");
        assert_eq!(forma_pagamento("99", ""), "Outro");
        assert_eq!(forma_pagamento("17", ""), "PIX - Dinâmico");
    }

    #[test]
    fn freight_modalities_depend_on_the_version() {
        assert_eq!(mod_frete(0, 4.0), "0 - REMETENTE");
        assert_eq!(mod_frete(0, 3.1), "0 - EMITENTE");
        assert_eq!(mod_frete(1, 4.0), "1 - DESTINATARIO");
        assert_eq!(mod_frete(9, 4.0), "9 - SEM FRETE");
        assert_eq!(mod_frete(5, 4.0), "");
        assert_eq!(mod_frete(0, 1.0), "");
    }

    #[test]
    fn vehicle_tables() {
        assert_eq!(veiculo_combustivel("3"), "03-DIESEL");
        assert_eq!(veiculo_combustivel("99"), "99NÃO DEFINIDO");
        assert_eq!(veiculo_tipo(6), "06-AUTOMÓVEL");
        assert_eq!(veiculo_vin("R"), "R-REMARCADO");
        assert_eq!(veiculo_vin("X"), "NÃO DEFINIDA");
        assert_eq!(veiculo_cor_denatran("11"), "11-PRETA");
        assert_eq!(arma_tipo(1), "1-USO RESTRITO");
        assert_eq!(ind_proc("3"), "SECEX / RFB");
    }
}
