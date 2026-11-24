# Proposal: vendor raw containers — Canon CR3 (#1474)

## Why

Every Canon body since 2018 (EOS R and RF, M50 and later, 90D, 250D, 850D,
the 1D X Mark III, recent PowerShots) writes CR3, not CR2. None opens today.
CR3 is an ISO base media file whose sensor data is always CRX-coded: a
lossless coding ("RAW") or a wavelet coding with quantization ("C-RAW").
There is no uncompressed CR3, so a container-only reader would open no file.

## Owner decisions (2026-11-23)

- **Provenance.** The issue's rule (public write-ups only, no dcraw, LibRaw
  or rawspeed code) changes for the CRX coding. The AI authoring model has
  training exposure to LibRaw's CRX decoder, so its own decoder would not be
  independent (as for ORF, #1535). The coding is ported openly from LibRaw
  0.22.1 (`crx.cpp`, LGPL-2.1 or CDDL-1.0; the port takes the LGPL-2.1,
  compatible with the project's GPL-3.0-or-later), with LibRaw's copyright
  notices and an entry in `assets/PROVENANCE.md`. It is a port, not a
  clean-room implementation.
- **Scope.** Lossless and C-RAW, the main image only: the first frame of
  the largest raw track. A raw burst's other frames and a Dual Pixel
  file's second image are not read. Unobserved variants are refused by name.
- **Colour in this change.** CR3 files embed no colour matrix. The fitted
  table (#1465) gains the CR3 bodies rawtoaces-data publishes sensitivities
  for (EOS R, RP, R5, R5 Mark II, R6, R6 Mark II, R10, 250D/200D II, and
  rawtoaces' same-sensor aliases R50, R8, Rebel SL3, Kiss X10), and the
  as-shot white balance is read from the CTMD track's maker note.
- **Patents.** A Google Patents status search on the CRX coding is
  recorded (status only, no legal conclusion), as for #1535 and #1541.

## What changes

- `sniff_raw` names an ISO media file of major brand `crx ` as CR3, and
  `read_raw` reads it.
- New `dng/cr3.rs` (container), and the CRX port in `dng/crx.rs`,
  `dng/crx_lines.rs` and `dng/crx_wavelet.rs`.
- `scripts/gen-camera-matrices.py` adds the CR3 bodies; the generated
  table grows by seven bodies and their aliases, and no existing row changes.
- The Open dialog lists `*.CR3` under Camera Raw and All Formats.

## Capabilities

- Modified: `codec/vendor-raw-containers`, `codec/camera-matrices`,
  `document/raw-open`.

## Provenance

The CRX coding is ported from LibRaw 0.22.1 (tag `0.22.1`, commit
b860248a89d9082b8e0a1e202e516f46af9adb29). The track choice, the `CMP1`
fields and the `CTMD` record walk follow its `cr3_parser.cpp`. The box walk
follows ISO/IEC 14496-12, and the maker-note tags follow the existing CR2
reader. The port was checked against LibRaw 0.22.1's unpacked sensor data on
real files (raw.pixls.us, kept outside the repository).
