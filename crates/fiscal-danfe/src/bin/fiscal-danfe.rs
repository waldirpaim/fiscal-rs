//! Linha de comando do DANFE: lê o XML autorizado e escreve o PDF.
//!
//! ```text
//! fiscal-danfe [opções] [nota.xml] > danfe.pdf
//! ```
//!
//! Sem arquivo (ou com `-`) o XML vem da entrada padrão; o PDF sai na saída padrão, salvo
//! `--saida`. Existe para quem não quer ligar o `fiscal-danfe` (LGPL) ao próprio
//! executável: o programa chamador só troca bytes com este processo.
//!
//! Códigos de saída: 0 sucesso; 2 uso incorreto; 3 falha ao ler a entrada ou o logo; 4 XML
//! inválido; 5 falha ao gerar ou gravar o PDF. Os avisos vão para a saída de erro e nunca
//! repetem o conteúdo do XML.

use std::fs::File;
use std::io::{self, Read, Write};
use std::process::ExitCode;

use fiscal_danfe::model::{DateTime, TpEmis};
use fiscal_danfe::options::PosCanhoto;
use fiscal_danfe::{DanfeError, DanfeOptions, Nfe, render_nfe};

/// Maior XML aceito (o mesmo limite de 10 MiB do servidor).
const MAX_XML_BYTES: u64 = 10 * 1024 * 1024;
/// Maior logo aceito.
const MAX_LOGO_BYTES: u64 = 5 * 1024 * 1024;

const USO: &str = "\
Uso: fiscal-danfe [opções] [nota.xml]

Lê o XML autorizado (nfeProc ou NFe) do arquivo, ou da entrada padrão quando não há
arquivo ou ele é `-`, e escreve o PDF do DANFE (modelo 55) ou do DANFCE (modelo 65) na
saída padrão.

Opções:
  --saida ARQUIVO          grava o PDF em ARQUIVO em vez da saída padrão
  --sistema TEXTO          texto do rodapé (até três partes separadas por `|`)
  --margem MM              as quatro margens, em mm (padrão: 3 na NF-e e 1 na NFC-e)
  --largura-bobina MM      largura da bobina da NFC-e (padrão: 80)
  --sem-canhoto            não imprime o canhoto da NF-e
  --logo ARQUIVO           logo em PNG ou JPEG
  --nome-fantasia          imprime o nome fantasia no lugar da razão social (NFC-e; padrão
                           na NFC-e)
  --razao-social           imprime a razão social na NFC-e
  --cancelada              marca d'água de nota cancelada
  --sem-compressao         não comprime o conteúdo do PDF (para inspeção)
  --protocolo-numero N     completa o protocolo de autorização quando o XML não o traz e,
                           com ele, trata a nota como de emissão normal (sem o aviso de
                           contingência)
  --protocolo-data DATA    data e hora do recebimento (AAAA-MM-DDThh:mm:ss)
  --protocolo-status N     código da situação (100 por padrão)
  --protocolo-digest TEXTO digest value do protocolo
  --protocolo-motivo TEXTO descrição da situação
  -h, --help               esta ajuda
";

/// Protocolo de autorização vindo da linha de comando.
#[derive(Default)]
struct Protocolo {
    numero: Option<String>,
    data: Option<String>,
    status: Option<String>,
    digest: Option<String>,
    motivo: Option<String>,
}

/// Argumentos já lidos.
#[derive(Default)]
struct Args {
    entrada: Option<String>,
    saida: Option<String>,
    sistema: Option<String>,
    margem: Option<f64>,
    largura_bobina: Option<u32>,
    sem_canhoto: bool,
    logo: Option<String>,
    nome_fantasia: Option<bool>,
    cancelada: bool,
    sem_compressao: bool,
    protocolo: Protocolo,
}

/// Falha com o código de saída e a mensagem.
struct Falha {
    codigo: u8,
    mensagem: String,
}

impl Falha {
    fn new(codigo: u8, mensagem: impl Into<String>) -> Self {
        Self {
            codigo,
            mensagem: mensagem.into(),
        }
    }

    fn uso(mensagem: impl Into<String>) -> Self {
        Self::new(2, mensagem)
    }
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(falha) => {
            eprintln!("fiscal-danfe: {}", falha.mensagem);
            ExitCode::from(falha.codigo)
        }
    }
}

fn run(args: impl Iterator<Item = String>) -> Result<(), Falha> {
    let Some(args) = parse(args)? else {
        print!("{USO}");
        return Ok(());
    };

    let xml = read_input(args.entrada.as_deref())?;
    let mut nfe = Nfe::from_xml_bytes(&xml).map_err(|error| match error {
        DanfeError::Xml(_) | DanfeError::InvalidNfe(_) => Falha::new(4, error.to_string()),
        DanfeError::Pdf(_) => Falha::new(5, error.to_string()),
    })?;
    drop(xml);

    completar_protocolo(&mut nfe, &args.protocolo)?;
    let options = options(&args, nfe.ide.modelo)?;
    let pdf = render_nfe(nfe, &options).map_err(|error| Falha::new(5, error.to_string()))?;

    match args.saida {
        Some(path) => std::fs::write(&path, &pdf)
            .map_err(|error| Falha::new(5, format!("não foi possível gravar {path}: {error}")))?,
        None => {
            let mut out = io::stdout().lock();
            out.write_all(&pdf)
                .and_then(|()| out.flush())
                .map_err(|error| {
                    Falha::new(5, format!("não foi possível escrever o PDF: {error}"))
                })?;
        }
    }
    Ok(())
}

/// Lê os argumentos; `None` quando pediram a ajuda.
fn parse(mut args: impl Iterator<Item = String>) -> Result<Option<Args>, Falha> {
    let mut out = Args::default();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| Falha::uso(format!("{name} pede um valor")))
        };
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--saida" => out.saida = Some(value("--saida")?),
            "--sistema" => out.sistema = Some(value("--sistema")?),
            "--margem" => {
                let text = value("--margem")?;
                let mm = text
                    .replace(',', ".")
                    .parse::<f64>()
                    .ok()
                    .filter(|mm| (0.0..=50.0).contains(mm))
                    .ok_or_else(|| Falha::uso("--margem pede um número de 0 a 50"))?;
                out.margem = Some(mm);
            }
            "--largura-bobina" => {
                let mm = value("--largura-bobina")?
                    .parse::<u32>()
                    .ok()
                    .filter(|mm| (40..=300).contains(mm))
                    .ok_or_else(|| Falha::uso("--largura-bobina pede um número de 40 a 300"))?;
                out.largura_bobina = Some(mm);
            }
            "--sem-canhoto" => out.sem_canhoto = true,
            "--logo" => out.logo = Some(value("--logo")?),
            "--nome-fantasia" => out.nome_fantasia = Some(true),
            "--razao-social" => out.nome_fantasia = Some(false),
            "--cancelada" => out.cancelada = true,
            "--sem-compressao" => out.sem_compressao = true,
            "--protocolo-numero" => out.protocolo.numero = Some(value("--protocolo-numero")?),
            "--protocolo-data" => out.protocolo.data = Some(value("--protocolo-data")?),
            "--protocolo-status" => out.protocolo.status = Some(value("--protocolo-status")?),
            "--protocolo-digest" => out.protocolo.digest = Some(value("--protocolo-digest")?),
            "--protocolo-motivo" => out.protocolo.motivo = Some(value("--protocolo-motivo")?),
            "-" => {
                if out.entrada.replace(arg).is_some() {
                    return Err(Falha::uso("só um arquivo de entrada"));
                }
            }
            flag if flag.starts_with("--") => {
                return Err(Falha::uso(format!("opção desconhecida: {flag}")));
            }
            _ => {
                if out.entrada.replace(arg).is_some() {
                    return Err(Falha::uso("só um arquivo de entrada"));
                }
            }
        }
    }
    Ok(Some(out))
}

/// O XML do arquivo ou da entrada padrão, com o limite de tamanho.
fn read_input(path: Option<&str>) -> Result<Vec<u8>, Falha> {
    let mut bytes = Vec::new();
    let read = match path {
        None | Some("-") => io::stdin()
            .lock()
            .take(MAX_XML_BYTES + 1)
            .read_to_end(&mut bytes),
        Some(path) => File::open(path)
            .map_err(|error| Falha::new(3, format!("não foi possível abrir {path}: {error}")))?
            .take(MAX_XML_BYTES + 1)
            .read_to_end(&mut bytes),
    };
    read.map_err(|error| Falha::new(3, format!("não foi possível ler o XML: {error}")))?;
    if bytes.len() as u64 > MAX_XML_BYTES {
        return Err(Falha::new(3, "o XML excede o limite de 10 MiB"));
    }
    Ok(bytes)
}

/// Completa o protocolo quando o XML não o traz (`nProt` vazio) e a linha de comando o dá.
///
/// Uma nota de contingência offline gravada antes da transmissão não tem `protNFe`; já
/// autorizada, o DANFE não deve sair "pendente de autorização" nem "emitida em
/// contingência". Por isso a emissão passa a normal junto com o protocolo.
fn completar_protocolo(nfe: &mut Nfe, protocolo: &Protocolo) -> Result<(), Falha> {
    let Some(numero) = protocolo
        .numero
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
    else {
        return Ok(());
    };
    if !nfe.proc_nfe.n_prot.trim().is_empty() {
        return Ok(());
    }
    let status = match protocolo.status.as_deref() {
        None => 100,
        Some(text) => text
            .trim()
            .parse::<u32>()
            .map_err(|_| Falha::uso("--protocolo-status pede um número"))?,
    };
    let data = match protocolo.data.as_deref() {
        None => None,
        Some(text) => Some(
            DateTime::parse(text)
                .ok_or_else(|| Falha::uso("--protocolo-data pede AAAA-MM-DDThh:mm:ss"))?,
        ),
    };
    let motivo = protocolo
        .motivo
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();

    let proc_nfe = &mut nfe.proc_nfe;
    proc_nfe.n_prot = numero.to_owned();
    proc_nfe.ch_nfe = nfe.id.strip_prefix("NFe").unwrap_or(&nfe.id).to_owned();
    proc_nfe.c_stat = status;
    proc_nfe.tp_amb = nfe.ide.tp_amb;
    proc_nfe.dh_recbto = data;
    proc_nfe.dig_val = protocolo
        .digest
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_owned();
    proc_nfe.x_motivo = if motivo.is_empty() {
        "Autorizado o uso da NF-e".to_owned()
    } else {
        motivo.to_owned()
    };
    nfe.ide.tp_emis = TpEmis::Normal;
    Ok(())
}

/// As opções do DANFE: o padrão de cada modelo e o que a linha de comando muda.
fn options(args: &Args, modelo: u32) -> Result<DanfeOptions, Falha> {
    let mut options = DanfeOptions {
        sistema: args.sistema.clone().unwrap_or_default(),
        cancelada: args.cancelada,
        comprimir_pdf: !args.sem_compressao,
        ..DanfeOptions::default()
    };
    let nfce = modelo == 65;
    let margem = args.margem.unwrap_or(if nfce { 1.0 } else { 3.0 });
    options.margem_esquerda = margem;
    options.margem_superior = margem;
    options.margem_direita = margem;
    options.margem_inferior = margem;
    if nfce {
        options.imprime_nome_fantasia = args.nome_fantasia.unwrap_or(true);
        if let Some(largura) = args.largura_bobina {
            options.largura_bobina = largura;
        }
    }
    if args.sem_canhoto {
        options.pos_canhoto = PosCanhoto::Nenhum;
    }
    if let Some(path) = &args.logo {
        let mut logo = Vec::new();
        File::open(path)
            .and_then(|file| file.take(MAX_LOGO_BYTES + 1).read_to_end(&mut logo))
            .map_err(|error| {
                Falha::new(3, format!("não foi possível ler o logo {path}: {error}"))
            })?;
        if logo.len() as u64 > MAX_LOGO_BYTES {
            return Err(Falha::new(3, "o logo excede o limite de 5 MiB"));
        }
        options.logo = Some(logo);
    }
    Ok(options)
}
