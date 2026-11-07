//! Renders a DANFE with options given by their ACBr names, for the differential test against
//! the real ACBr (see `tests/acbr/README.md`):
//! `acbr_diff <nota.xml> <saida.pdf> [Nome=Valor ...]`.
//!
//! The names and values are the ones the Pascal harness gives the ACBr component (`Sistema`,
//! `PosCanhoto=prRodape`, `ExibeCampoDePagamento=eipQuadro`...), so the same list drives both.
//! An option this crate does not have is reported on the standard error and ignored.

use fiscal_danfe::DanfeOptions;
use fiscal_danfe::options::{
    InfAdicProduto, InformacoesDePagamento, PercentualTributos, PosCanhoto,
};

fn flag(value: &str) -> bool {
    value.eq_ignore_ascii_case("true")
}

fn number(value: &str) -> f64 {
    value.replace(',', ".").parse().expect("number")
}

/// Applies `name=value`; `false` if the option is unknown.
fn apply(o: &mut DanfeOptions, event: &mut Option<String>, name: &str, value: &str) -> bool {
    match name.to_ascii_lowercase().as_str() {
        "evento" => *event = Some(value.to_owned()),
        "sistema" => o.sistema = value.to_owned(),
        "margemesquerda" => o.margem_esquerda = number(value),
        "margemsuperior" => o.margem_superior = number(value),
        "margemdireita" => o.margem_direita = number(value),
        "margeminferior" => o.margem_inferior = number(value),
        "logo" => o.logo = Some(std::fs::read(value).expect("logo")),
        "logoemcima" => o.logo_em_cima = flag(value),
        "expandelogomarca" => o.expande_logo_marca = flag(value),
        "poscanhoto" => {
            o.pos_canhoto = match value {
                "prCabecalho" => PosCanhoto::Cabecalho,
                "prRodape" => PosCanhoto::Rodape,
                "prEsquerda" => PosCanhoto::Esquerda,
                other => panic!("PosCanhoto {other}"),
            }
        }
        "cancelada" => o.cancelada = flag(value),
        "caracterequebradelinha" => o.caractere_quebra_de_linha = value.to_owned(),
        "quebralinhaemdetalhamentos" => o.quebra_linha_em_detalhamentos = flag(value),
        "imprimecodigoean" => o.imprime_codigo_ean = flag(value),
        "imprimetotalliquido" => o.imprime_total_liquido = flag(value),
        "exibeinforadicproduto" => {
            o.exibe_infor_adic_produto = match value {
                "infNenhum" => InfAdicProduto::Nenhum,
                "infDescricao" => InfAdicProduto::Descricao,
                "infSeparadamente" => InfAdicProduto::Separadamente,
                other => panic!("ExibeInforAdicProduto {other}"),
            }
        }
        "exibetotaltributositem" => o.exibe_total_tributos_item = flag(value),
        "tributospercentual" => {
            o.tributos_percentual = match value {
                "ptValorProdutos" => PercentualTributos::ValorProdutos,
                "ptValorNF" => PercentualTributos::ValorNf,
                "ptPersonalizado" => PercentualTributos::Personalizado,
                other => panic!("TributosPercentual {other}"),
            }
        }
        "tributospercentualpersonalizado" => o.tributos_percentual_personalizado = number(value),
        "imprimedetalhamentoespecifico" => o.imprime_detalhamento_especifico = flag(value),
        "imprimexpednitemped" => o.imprime_xped_nitemped = flag(value),
        "imprimeinfcontr" => o.imprime_inf_contr = flag(value),
        "exibedadosdocreferenciados" => o.exibe_dados_doc_referenciados = flag(value),
        "exibedadosinscricaosuframa" => o.exibe_dados_inscricao_suframa = flag(value),
        "exibeicmsdesoneradocomodesconto" => o.exibe_icms_desonerado_como_desconto = flag(value),
        "exibecampofatura" => o.exibe_campo_fatura = flag(value),
        "exibecampoduplicata" => o.exibe_campo_duplicata = flag(value),
        "exibecampodepagamento" => {
            o.exibe_campo_de_pagamento = match value {
                "eipNunca" => InformacoesDePagamento::Nunca,
                "eipAdicionais" => InformacoesDePagamento::Adicionais,
                "eipQuadro" => InformacoesDePagamento::Quadro,
                other => panic!("ExibeCampoDePagamento {other}"),
            }
        }
        "formatarnumerodocumento" => o.formatar_numero_documento = flag(value),
        "imprimenomefantasia" => o.imprime_nome_fantasia = flag(value),
        "largurabobina" => o.largura_bobina = value.parse().expect("width"),
        "imprimeqrcodelateral" => o.imprime_qrcode_lateral = flag(value),
        "exibiritens" => o.exibir_itens = flag(value),
        "exibirnomefantasia" => o.exibir_nome_fantasia = flag(value),
        "exibirconsumidornome" => o.exibir_consumidor_nome = flag(value),
        "exibirconsumidorendereco" => o.exibir_consumidor_endereco = flag(value),
        _ => return false,
    }
    true
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: acbr_diff <nota.xml> <saida.pdf> [Nome=Valor ...]";
    let input = args.next().expect(usage);
    let output = args.next().expect(usage);
    let mut options = DanfeOptions::default();
    let mut ignored = Vec::new();
    let mut event = None;
    for pair in args {
        let (name, value) = pair.split_once('=').expect("Nome=Valor");
        if !apply(&mut options, &mut event, name, value) {
            ignored.push(name.to_owned());
        }
    }
    if !ignored.is_empty() {
        eprintln!("ignorado: {}", ignored.join(","));
    }
    let xml = std::fs::read(&input).expect("read the XML");
    let pdf = match event {
        Some(path) => {
            let event = std::fs::read(path).expect("read the event");
            fiscal_danfe::render_evento_xml(&xml, &event, &options).expect("render the event")
        }
        None => fiscal_danfe::render_danfe_bytes(&xml, &options).expect("render"),
    };
    std::fs::write(&output, pdf).expect("write the PDF");
}
