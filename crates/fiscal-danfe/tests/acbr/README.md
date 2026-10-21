# Differential test against the real ACBr

`fiscal-danfe` is a translation of the ACBr DANFE layout. This folder checks it against the
thing it translates: the ACBr itself, compiled with Free Pascal, prints the same XML with the
same options, and the PDFs are compared (page count, page size and, above all, the **content
streams**, byte by byte).

Result with ACBr from the `x-duall` tree (FPDF DANFE of NF-e, NFC-e and events), the two
synthetic fixtures, their variants and four real XML (150 cases):

| | |
|---|---|
| Identical content streams | **143** |
| Known and decided differences | 7 (below) |
| Unexplained differences | **0** |

The cases (`matrix.py`) cover: portrait and landscape, the three stub positions, 8 to 150 items
(1 to 7 pages), every issuing mode (`tpEmis` 2 to 9, with and without protocol), every
watermark (canceled, denied, homologation, no protocol), recipients with CPF, abroad and none,
carriers, instalments, payments, ISSQN, vehicles, medicines, fuel, firearms, tax percentages,
logos, margins, footers, the receipt widths and layouts of the NFC-e, and the event DANFE (CC-e,
cancellation, other event types, huge texts, notes without recipient or exit date).

## Known differences

| Case | Why |
|---|---|
| `nfce/sem-qrcode` and the real offline NFC-e without `infNFeSupl` | The ACBr divides by zero drawing a QR Code of nothing (`EInvalidOp` under Free Pascal; with the exception masked it draws meaningless rectangles). Here no QR Code is drawn. |
| `nfce/logo`, `nfce/logo-58` | The ACBr moves the content down 5 mm only in the final pass, so the receipt is 5 mm short and its bottom is clipped. Here the height includes the 5 mm. |
| `evento/sem-retevento` | Without the `retEvento` the ACBr prints the registration date as `30/12/1899 00:00:00` (a zero `TDateTime`). Here it stays blank. |
| `evento/nfce` | The NFC-e class of the ACBr (FPDF) does not print events. Here the same A4 event layout comes out. |

The test found one real bug, now fixed: `RoundTo` of the original rounds half to even
(`RoundTo(30.5, 0)` is `30`), and this crate rounded half away from zero, which moved a column
by 1 mm in a few layouts (portrait with the stub on the left, expanded logo).

## What the harness does not reproduce

The ACBr of this tree ignores some component properties in the FPDF classes (`PosCanhoto` and
`Cancelada` are not handed to the report; the `Exibir*` options of the NFC-e exist only on the
inner report; `CaractereQuebraDeLinha` is not configurable). The harness (`acbrdanfe.pas`)
drives the inner report for those, so they are compared too.

Free Pascal on Linux also differs from the Delphi of the MCPServer in a few places that are not
layout, and the harness neutralises them: pt-BR format settings, the ANSI code page 1252 for the
source literals, masked floating-point exceptions (`MASKFP=1`, only for XML that crashes the ACBr),
and one line in the event unit (`FFormatSettings` is only initialised in the Delphi branch; the
issue date of the invoice comes out as `20`). That one is patched in a copy made by `build.py`;
the ACBr sources are never changed or committed. The comparison also accepts numbers that differ
by one hundredth (the same value at the rounding edge of two float formatters).

## Running it

Needs the ACBr sources (`Fontes`), Free Pascal 3.2.2 and the LCL units, all unpacked without
installing anything. On Ubuntu:

```sh
mkdir -p fpc/debs && cd fpc/debs
apt-get download fp-compiler-3.2.2 fp-units-rtl-3.2.2 fp-units-base-3.2.2 fp-units-fcl-3.2.2 \
  fp-units-misc-3.2.2 fp-units-math-3.2.2 fp-units-net-3.2.2 fp-units-db-3.2.2 fp-utils-3.2.2 \
  fp-units-gfx-3.2.2 fp-units-multimedia-3.2.2 lcl-units-4.4 lcl-nogui-4.4
for d in *.deb; do dpkg -x "$d" ..; done; cd ../..

python3 crates/fiscal-danfe/tests/acbr/build.py --acbr /path/to/acbr/Fontes \
  --fpc fpc --lcl fpc/usr/lib/lazarus/4.4 --out acbr-harness
cargo build --release -p fiscal-danfe --example acbr_diff

export LD_LIBRARY_PATH=$PWD/acbr-harness/lib
python3 crates/fiscal-danfe/tests/acbr/matrix.py acbr-harness/bin/acbrdanfe \
  target/release/examples/acbr_diff crates/fiscal-danfe/tests/fixtures [real.xml ...] [-k filtro]
```

`matrix.py` exits with 0 when everything is identical or a known difference, and needs Pillow
only for the logo cases. Do not commit real XML or the PDFs the ACBr makes.
