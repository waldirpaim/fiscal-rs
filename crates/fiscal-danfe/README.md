# fiscal-danfe

Renders the **DANFE** of an NF-e (model 55) and the **DANFE NFC-e** receipt (model 65) as a
PDF, straight from the authorised XML — no database, no files, no external service.

```rust
use fiscal_danfe::{DanfeOptions, render_danfe};

let xml = std::fs::read_to_string("35260711222333000181550010000012341123456780-nfe.xml")?;
let pdf: Vec<u8> = render_danfe(&xml, &DanfeOptions::default())?;
std::fs::write("danfe.pdf", pdf)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The input is an `nfeProc` (NF-e with the authorisation protocol) or a bare `NFe`; the model in
the access key (`mod`) picks the layout. Use [`render_danfe_bytes`] for files in Windows-1252.

## Command line

The `fiscal-danfe` executable does the same from a shell, or from a program that prefers not
to link this crate (see the license below): it reads the XML from a file or from the standard
input and writes the PDF to the standard output or to `--saida`.

```text
fiscal-danfe [options] [nota.xml] > danfe.pdf
fiscal-danfe --evento procEventoNFe.xml [options] nota.xml > evento.pdf
fiscal-danfe --help
```

It applies defaults per model (3 mm margins for the NF-e, 1 mm and the trade name for the
NFC-e). `--protocolo-numero` (with `--protocolo-data`, `--protocolo-status`,
`--protocolo-digest` and `--protocolo-motivo`) fills the authorisation protocol of an XML that
lacks one — an offline NFC-e saved before transmission — and then treats the note as a normal
emission, so the DANFE is not stamped "pending authorisation". Exit codes: 0 success, 2 usage,
3 input (file, logo, 10 MiB limit), 4 invalid XML, 5 rendering or output; messages go to the
standard error and never repeat the XML.

## License

**LGPL-2.1-or-later** — this crate is a Rust translation of the DANFE layout of the
[ACBr](https://www.projetoacbr.com.br/) project (`ACBrNFeDANFeFPDF`, `ACBrNFCeDANFeFPDF` and
their helper units), so it carries ACBr's license. See `NOTICE.md` and `LICENSE`. The PDF
engine it builds on, [`fiscal-pdf`](../fiscal-pdf), stays MIT.

## What is covered

* **NF-e DANFE** (portrait and landscape, A4): receipt stub (top, bottom or left), header with
  the Code 128 access key, protocol or contingency data, recipient, pick-up/delivery locations,
  invoice and instalments, payment boxes, tax totals, transport, items table that flows across
  pages (header and additional data repeat, "Folha n/N" is exact), ISSQN totals, additional
  information, footer and the watermarks (canceled, denied, EPEC, homologation, missing
  protocol, contingency).
* **NFC-e DANFE**: the 80/58 mm receipt on a single endless page (as tall as its content), the
  QR Code (centred or at the side), consumer, items, totals, payments, access key, protocol,
  contingency/homologation banners, logo and the *simplified type 2* variant with IBS/CBS/IS.
* Item details: description + additional information, approximate taxes, vehicles, medicines,
  firearms, fuel, traceability; additional information (referenced documents, notes, payments,
  contingency text, SUFRAMA).

* **Event DANFE** (`render_evento`, `render_evento_xml`, `Evento::from_xml`): the correction letter
  (CC-e) and the cancellation, and the header, invoice, event, issuer and recipient blocks of any
  other event type, from a `procEventoNFe` together with the XML of its invoice.

Not ported: the DANFE of voided number ranges (the FPDF layouts of ACBr do not print it).

## Differences from ACBr

* Amounts stay exact decimals read from the XML (no binary floating point); only percentages
  go through `f64`, as in the original.
* Errors are returned, not raised; the layout never loops forever on a band that cannot fit.
* On the NFC-e, the 5 mm taken by a logo is reserved in the first pass too, so the receipt is
  tall enough (the original clips the bottom of receipts with a logo).
* An NFC-e without `infNFeSupl` (offline, saved before the QR Code was built) gets no QR Code;
  the original divides by zero.
* The event DANFE also prints for an NFC-e, and leaves the registration date blank when the
  XML has no `retEvento` (the original prints `30/12/1899`).
* `PosCanhoto::Nenhum` hides the receipt stub, which the original cannot do.
* Fixes inherited from [`fiscal-pdf`](../fiscal-pdf) (see its README).

## Testing

`tests/fixtures` holds synthetic documents (invented companies, valid access keys). Do not add
real customer XML there.

`tests/acbr` compares the output with the real ACBr, compiled with Free Pascal, case by case and
byte by byte (150 cases, 143 identical, 7 known differences, none unexplained): see its README.
