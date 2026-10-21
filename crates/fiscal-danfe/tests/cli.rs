//! Tests of the `fiscal-danfe` command line (the executable that keeps the LGPL crate out of
//! the caller's binary).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const NFE: &str = include_str!("fixtures/nfe_55.xml");
const NFCE: &str = include_str!("fixtures/nfce_65.xml");
const CCE: &str = include_str!("fixtures/evento_cce.xml");

fn exe() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fiscal-danfe"))
}

/// Runs the executable with `xml` on the standard input.
fn run(args: &[&str], xml: &[u8]) -> Output {
    let mut child = exe()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    // The child may exit before reading everything (rejected oversize input).
    let _ = child.stdin.take().expect("stdin").write_all(xml);
    child.wait_with_output().expect("output")
}

/// The PDF as text, one char per byte.
fn text(pdf: &[u8]) -> String {
    pdf.iter().map(|b| char::from(*b)).collect()
}

fn temp_file(name: &str, content: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fiscal-danfe-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write");
    path
}

/// Removes the `protNFe` block.
fn without_protocol(xml: &str) -> String {
    let start = xml.find("<protNFe").expect("protNFe");
    let end = xml.find("</protNFe>").expect("end") + "</protNFe>".len();
    format!("{}{}", &xml[..start], &xml[end..])
}

#[test]
fn nfe_from_stdin_to_stdout() {
    let out = run(&[], NFE.as_bytes());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.starts_with(b"%PDF-"));
    assert!(text(&out.stdout).trim_end().ends_with("%%EOF"));
    assert!(out.stderr.is_empty());
}

#[test]
fn nfe_from_file_to_file() {
    let input = temp_file("nfe.xml", NFE.as_bytes());
    let output = input.with_file_name("nfe.pdf");
    let out = exe()
        .arg(&input)
        .arg("--saida")
        .arg(&output)
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    assert!(std::fs::read(&output).expect("pdf").starts_with(b"%PDF-"));
}

#[test]
fn nfce_uses_the_receipt_width() {
    let out = run(&["--largura-bobina", "58"], NFCE.as_bytes());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pdf = text(&out.stdout);
    // 58 mm = 164.41 pt.
    assert!(
        pdf.contains("/MediaBox [0 0 164.4"),
        "{}",
        &pdf[..400.min(pdf.len())]
    );
}

#[test]
fn system_text_goes_to_the_footer() {
    let out = run(
        &["--sem-compressao", "--sistema", "NEXO-RS-TESTE"],
        NFE.as_bytes(),
    );
    assert!(out.status.success());
    assert!(text(&out.stdout).contains("NEXO-RS-TESTE"));
}

#[test]
fn protocol_is_completed_only_when_the_xml_has_none() {
    let bare = without_protocol(NFE);
    let args = [
        "--sem-compressao",
        "--protocolo-numero",
        "999888777666555",
        "--protocolo-data",
        "2026-07-20T10:40:00",
    ];

    let out = run(&["--sem-compressao"], bare.as_bytes());
    assert!(out.status.success());
    assert!(!text(&out.stdout).contains("999888777666555"));

    let out = run(&args, bare.as_bytes());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pdf = text(&out.stdout);
    assert!(pdf.contains("999888777666555"));
    assert!(pdf.contains("20/07/2026"));

    // With a protocol in the XML, the one from the command line is ignored.
    let out = run(&args, NFE.as_bytes());
    assert!(out.status.success());
    let pdf = text(&out.stdout);
    assert!(pdf.contains("135260000012345"));
    assert!(!pdf.contains("999888777666555"));
}

#[test]
fn offline_nfce_with_protocol_drops_the_contingency_notice() {
    let offline = without_protocol(NFCE).replace("<tpEmis>1</tpEmis>", "<tpEmis>9</tpEmis>");

    let out = run(&["--sem-compressao"], offline.as_bytes());
    assert!(out.status.success());
    let pdf = text(&out.stdout);
    assert!(
        pdf.contains("EMITIDA EM CONTING"),
        "contingency notice expected"
    );

    let out = run(
        &["--sem-compressao", "--protocolo-numero", "151260000099999"],
        offline.as_bytes(),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pdf = text(&out.stdout);
    assert!(pdf.contains("151260000099999"));
    assert!(!pdf.contains("EMITIDA EM CONTING"));
}

#[test]
fn invalid_xml_exits_with_4_and_does_not_echo_the_input() {
    let out = run(&[], b"<nfe>segredo-no-xml");
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("fiscal-danfe: "));
    assert!(!stderr.contains("segredo-no-xml"));
}

#[test]
fn usage_errors_exit_with_2() {
    for args in [
        &["--nao-existe"][..],
        &["--margem"],
        &["--margem", "abc"],
        &["--largura-bobina", "10"],
        &["a.xml", "b.xml"],
        &["--protocolo-status"],
    ] {
        let out = run(args, b"");
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn bad_protocol_values_exit_with_2() {
    let bare = without_protocol(NFE);
    for args in [
        &["--protocolo-numero", "1", "--protocolo-status", "x"][..],
        &["--protocolo-numero", "1", "--protocolo-data", "ontem"],
    ] {
        let out = run(args, bare.as_bytes());
        assert_eq!(out.status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn missing_files_exit_with_3() {
    let out = exe().arg("/nao/existe/nota.xml").output().expect("run");
    assert_eq!(out.status.code(), Some(3));
    let out = run(&["--logo", "/nao/existe/logo.png"], NFE.as_bytes());
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn oversize_input_is_rejected() {
    let big = vec![b' '; 10 * 1024 * 1024 + 1];
    let out = run(&[], &big);
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stderr).contains("10 MiB"));
}

#[test]
fn help_prints_the_usage() {
    let out = exe().arg("--help").output().expect("run");
    assert!(out.status.success());
    let help = String::from_utf8_lossy(&out.stdout);
    assert!(help.starts_with("Uso: fiscal-danfe"));
    assert!(help.contains("--protocolo-numero"));
    assert!(help.contains("--sem-compressao"));
}

#[test]
fn event_danfe_from_the_event_file() {
    let evento = temp_file("cce.xml", CCE.as_bytes());
    let evento = evento.to_str().expect("path");
    let out = run(&["--sem-compressao", "--evento", evento], NFE.as_bytes());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pdf = text(&out.stdout);
    assert!(pdf.starts_with("%PDF-"));
    assert!(pdf.contains("Carta de Correcao"));
    assert!(pdf.contains("135260000098765"));
}

#[test]
fn event_errors_use_their_own_codes() {
    let evento = temp_file("cce-ok.xml", CCE.as_bytes());
    let evento = evento.to_str().expect("path");
    let out = run(
        &["--evento", evento, "--protocolo-numero", "1"],
        NFE.as_bytes(),
    );
    assert_eq!(out.status.code(), Some(2));

    let out = run(&["--evento", "/nao/existe/evento.xml"], NFE.as_bytes());
    assert_eq!(out.status.code(), Some(3));

    let invalid = temp_file(
        "evento-ruim.xml",
        b"<procEventoNFe><evento/></procEventoNFe>",
    );
    let out = run(
        &["--evento", invalid.to_str().expect("path")],
        NFE.as_bytes(),
    );
    assert_eq!(out.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("fiscal-danfe: evento: "));
}
