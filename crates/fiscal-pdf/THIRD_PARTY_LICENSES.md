# Third-party notices

`fiscal-pdf` is a Rust translation of **FPDF-Pascal**, itself a translation of **FPDF** (PHP).
The translated code remains subject to the notices below, which must be kept in every copy or
substantial portion of this crate.

## FPDF-Pascal (Projeto ACBr)

<https://github.com/Projeto-ACBr-Oficial/FPDF-Pascal> — units `ACBr_fpdf`, `ACBr_fpdf_ext`
(and the font metric tables and barcode tables extracted from them).

```text
Copyright (C) 2023 Projeto ACBr - Daniel Simões de Almeida

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

Except as contained in this notice, the name of <Projeto ACBr> shall not be
used in advertising or otherwise to promote the sale, use or other dealings in
this Software without prior written authorization from <Projeto ACBr>.
```

## FPDF-Pascal Report (Arimateia Jr — Nuvem Fiscal)

<https://github.com/arimateia/FPDF-Pascal> (fork of the project above) — unit
`ACBr_fpdf_report`, the model of the `report` module.

```text
Copyright (c) 2023 Arimateia Jr - https://nuvemfiscal.com.br

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to do
so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

(Contributors named in that file: Victor H Gonzales — Pandaaa.)

## Works FPDF-Pascal is based on

* **FPDF**, written in PHP by Olivier Plathey — <http://www.fpdf.org/>. The core engine
  (document model, cells, `MultiCell`, `Write`, images, output) follows FPDF 1.85, and the PDF
  core-font width tables originate from the Adobe font metrics.
* **Free JPDF Pascal**, by Jean Patrick and Gilson Nunes —
  <https://github.com/jepafi/Free-JPDF-Pascal>, distributed under:

  ```text
  Permission is hereby granted, free of charge, to any person obtaining a copy
  of this software to use, copy, modify, distribute, sublicense, and/or sell
  copies of the software, and to permit persons to whom the software is furnished
  to do so.

  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED.
  ```

* The **FPDF scripts** published at <http://www.fpdf.org/en/script/index.php> that the
  extensions follow: EAN-13/EAN-8 (Olivier), Code 39 (The-eh), Interleaved 2 of 5 (Matthias
  Lau), Code 128 (Roland Gautier), Rotation (Olivier), Rounded rectangle (Christophe
  Prugnaud), Dashes (yukihiro_o), Transformations (Moritz Wagner & Andreas Würmser), Layers
  (Olivier), Protection (Klemen Vodopivec), WordWrap (Ron Korving).
