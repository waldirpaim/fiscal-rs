# fiscal-pdf

A pure-Rust PDF engine with the [FPDF](http://www.fpdf.org/) programming model, built to render
fiscal auxiliary documents (DANFE, DANFCE, ...). It is a port of
[FPDF-Pascal](https://github.com/Projeto-ACBr-Oficial/FPDF-Pascal) — the engine underneath the
ACBr `ACBrNFeDANFeFPDF` / `ACBrNFCeDANFeFPDF` components — so layout code written against that
API translates call by call.

```rust
use fiscal_pdf::{Fpdf, Orientation, PageFormat, Unit};

let mut pdf = Fpdf::new(Orientation::Portrait, Unit::Mm, PageFormat::A4);
pdf.add_page();
pdf.set_font("Arial", "B", 16.0);
pdf.cell(40.0, 10.0, "Hello World!");
let bytes: Vec<u8> = pdf.output()?;
# Ok::<(), fiscal_pdf::FpdfError>(())
```

## What is included

| Area | API |
|------|-----|
| Document | pages, formats, orientation, rotation, margins, auto page break, header/footer hooks, `{nb}` alias, metadata, zoom/layout, compression |
| Text | `cell`, `multi_cell` (justified, indent), `write`, `text`, `text_box`, `word_wrap`, `get_string_width`, underline, the 14 PDF core fonts (WinAnsi / code page 1252) |
| Graphics | `line`, `rect`, `rounded_rect`, dashes, `rotate`, transforms, colours |
| Images | PNG (including alpha as soft mask) and JPEG, embedded once per distinct content |
| Links | internal and external, as annotations |
| Barcodes | Code 128 (NF-e access key), EAN-13, EAN-8, Code 39, Interleaved 2 of 5, QR Code, generic 2D matrices |
| Reports | `report` module: band-based layout engine (page header/footer, margin bands, watermark, flowing data bands, double pass for "page x of y", endless pages) |
| Extras | optional content layers, `write_html` (`<b> <i> <u> <br> <a>`), RC4 protection (`protection` feature, obsolete scheme) |

Not ported: TrueType/Type 1 font embedding (the original has it as a TODO too) and downloading
images from URLs.

## Differences from FPDF-Pascal

The output is byte-for-byte the same stream format and number formatting (`%.2f` and
`FloatToStr` are reproduced with Delphi's rounding), except where the Pascal code
mis-translates the PHP original. Those spots follow the intended FPDF behaviour:

* `MultiCell` drops the *last* character of a text whose second-to-last character is `\n`
  (a 1-based/0-based slip); here only a trailing newline is dropped. The indent applies to the
  first line however it ends.
* `Write` starts its first chunk at index 0 instead of 1.
* `SetFontSize` looks the font up by the wrong name for `Times` (writing `/F0`).
* Symbol/ZapfDingbats share the Latin `ToUnicode` CMap.
* Document-information strings with accents are written as UTF-16BE (the PDF way) instead of
  code page bytes.
* EAN-8 check digit weights, short EAN codes, the EAN bar offset, Code 128 set conversion
  (sequential `StringReplace` instead of a simultaneous map) and `WordWrap` indentation.
* `WriteHTML` ignored the newline replacement and could loop forever on `<>` or an unterminated tag;
  `TextBox` with `scale` could loop forever when the font reached 0 pt.
* Errors do not raise: the first one is kept and returned by `output()` (see `FpdfError`).

## License

MIT, see `THIRD_PARTY_LICENSES.md` for the notices of the works this crate is derived from.
