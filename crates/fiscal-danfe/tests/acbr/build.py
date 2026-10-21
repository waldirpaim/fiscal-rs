#!/usr/bin/env python3
"""Builds the ACBr harness (`acbrdanfe`) with Free Pascal, from the ACBr sources.

    build.py --acbr ACBR/Fontes --fpc FPC_ROOT --lcl LAZARUS_ROOT --out OUT_DIR

`--fpc` is a directory with the Free Pascal debs unpacked (`usr/lib/x86_64-linux-gnu/fpc/3.2.2`),
`--lcl` the one with `lcl-units` unpacked (`usr/lib/lazarus/4.4`): see README.md. The ACBr
sources are read, never copied, except for one unit that gets a one-line patch (see PATCH).
The program ends up in `OUT_DIR/bin/acbrdanfe`, with `OUT_DIR/lib` to put in LD_LIBRARY_PATH.
"""
import argparse
import glob
import os
import subprocess
import sys

TOP = ['ACBrComum', 'ACBrDiversos', 'ACBrDFe', 'PCNComum', 'ACBrOpenSSL', 'ACBrLibXML2', 'ACBrTCP',
       'Terceiros', 'ACBrSerial', 'ACBrIntegrador']
SKIP = ('fortes', 'lazreport', 'fast', 'rlreport', 'escpos', 'fortesreport')
UNIT = 'ACBrDFe/ACBrNFe/DANFE/NFe/FPDF/ACBrNFeDANFeFPDF.pas'
# Under Free Pascal the DANFE of events leaves its FormatSettings uninitialised (the branch that
# does it is Delphi-only), and the issue date comes out as "20". One line fixes the harness.
PATCH_BEFORE = b"  FFormatSettings.DecimalSeparator := ','"
PATCH_LINE = b"  FFormatSettings := DefaultFormatSettings;"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--acbr', required=True)
    ap.add_argument('--fpc', required=True)
    ap.add_argument('--lcl', required=True)
    ap.add_argument('--out', required=True)
    args = ap.parse_args()
    here = os.path.dirname(os.path.abspath(__file__))
    acbr, fpc, lcl, out = (os.path.abspath(p) for p in (args.acbr, args.fpc, args.lcl, args.out))
    rtl = glob.glob(f'{fpc}/usr/lib/*/fpc/*/ppcx64')[0]
    units = os.path.join(os.path.dirname(rtl), 'units', 'x86_64-linux')
    for sub in ('bin', 'lib', 'units', 'patch', 'tools'):
        os.makedirs(os.path.join(out, sub), exist_ok=True)

    # The patched unit goes first in the search path.
    patched = open(os.path.join(acbr, UNIT), 'rb').read()
    assert PATCH_BEFORE in patched, 'the unit changed: update PATCH_BEFORE'
    nl = b'\r\n' if b'\r\n' in patched else b'\n'
    patched = patched.replace(PATCH_BEFORE, PATCH_LINE + nl + PATCH_BEFORE, 1)
    open(os.path.join(out, 'patch', os.path.basename(UNIT)), 'wb').write(patched)

    cfg = [f'-Fu{out}/patch', f'-Fu{units}/*', f'-Fu{units}/rtl', f'-FE{out}/bin', f'-Fi{acbr}/ACBrComum']
    for top in TOP:
        base = os.path.join(acbr, top)
        for root, dirs, files in os.walk(base):
            dirs[:] = [d for d in dirs if d.lower() not in SKIP and not d.lower().startswith('fortesreport')]
            if any(f.endswith('.pas') for f in files):
                cfg.append(f'-Fu{root}')
    cfg += [f'-Fu{lcl}/lcl/units/x86_64-linux', f'-Fu{lcl}/lcl/units/x86_64-linux/nogui',
            f'-Fu{lcl}/components/lazutils/lib/x86_64-linux', f'-Fu{lcl}/components/freetype/lib/x86_64-linux',
            '-dLCL', '-dLCLnogui', '-Fl/usr/lib/x86_64-linux-gnu']
    crt = glob.glob('/usr/lib/gcc/x86_64-linux-gnu/*/crtbegin.o')
    if crt:
        cfg.append(f'-Fl{os.path.dirname(crt[0])}')
    open(os.path.join(out, 'fpc.cfg'), 'w').write('\n'.join(cfg) + '\n')

    # fpc looks for `fpcres`; the Debian package names it with the triplet.
    res = glob.glob(f'{fpc}/usr/bin/*fpcres*')
    link = os.path.join(out, 'tools', 'fpcres')
    if res and not os.path.exists(link):
        os.symlink(res[0], link)
    # ACBr loads libxml2.so.2 at run time; newer systems ship another soname.
    xml = sorted(glob.glob('/usr/lib/x86_64-linux-gnu/libxml2.so.*'))
    for name in ('libxml2.so.2', 'libxml2.so'):
        path = os.path.join(out, 'lib', name)
        if xml and not os.path.exists(path):
            os.symlink(xml[0], path)

    env = dict(os.environ, PATH=os.path.join(out, 'tools') + os.pathsep + os.environ['PATH'])
    # -Fccp1252: the sources are in code page 1252, as in Delphi; the program sets the same
    # code page for the ANSI strings at start.
    cmd = [rtl, f'@{out}/fpc.cfg', '-Mdelphi', '-Sd', '-Fccp1252', '-vew', '-B', f'-FU{out}/units',
           os.path.join(here, 'acbrdanfe.pas')]
    done = subprocess.run(cmd, env=env, capture_output=True, text=True, encoding='latin-1')
    errors = [l for l in done.stdout.splitlines() if ' Error' in l or 'Fatal' in l]
    if done.returncode != 0 or errors:
        print('\n'.join(errors) or done.stdout[-2000:], file=sys.stderr)
        sys.exit(1)
    print(f'{out}/bin/acbrdanfe\nLD_LIBRARY_PATH={out}/lib')


if __name__ == '__main__':
    main()
