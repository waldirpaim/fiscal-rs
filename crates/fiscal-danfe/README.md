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

Not ported: the DANFE of events (cancellation, correction letter) and of voided number ranges.

## Differences from ACBr

* Amounts stay exact decimals read from the XML (no binary floating point); only percentages
  go through `f64`, as in the original.
* Errors are returned, not raised; the layout never loops forever on a band that cannot fit.
* On the NFC-e, the 5 mm taken by a logo is reserved in the first pass too, so the receipt is
  tall enough (the original clips the bottom of receipts with a logo).
* `PosCanhoto::Nenhum` hides the receipt stub, which the original cannot do.
* Fixes inherited from [`fiscal-pdf`](../fiscal-pdf) (see its README).

## Testing

`tests/fixtures` holds synthetic documents (invented companies, valid access keys). Do not add
real customer XML there.
