# Design

## Context

`ControlServer::methodExportWebp` reads `quality` with `toInt(90)` and
`lossless` with `toBool(false)`. Qt's conversion defaults hide bad input:
`quality: -1` reaches `export_webp`, whose negative-quality branch writes VP8L,
and `lossless: "true"` becomes false. The capability spec says `quality` is 0
through 100 and `lossless` is a boolean.

`ExportAsDialog` sets the Quality spin box to 1..100. The engine and control
surface accept 0, and Photoshop's web-quality slider starts at 0.

## Decisions

### D1 — Validate in the control method, not the Rust bridge

The Rust `export_webp` bridge keeps its internal negative-quality convention for
the Export As bridge. The control method is the public boundary, so it checks the
JSON types and range before calling the bridge:

- absent `quality` → 90;
- present `quality` → a finite JSON number whose floor equals itself and lies in
  0..=100, otherwise `invalid_param`;
- absent `lossless` → false;
- present `lossless` → a JSON boolean, otherwise `invalid_param`.

The checks run before the export, so a refused request leaves no file.

### D2 — Quality 0 is reachable from Export As

Set the Quality spin box range to 0..100. This matches `encode_webp_lossy`, the
control surface, and the Save For Web quality slider. The default stays 90.

### D3 — Docs describe the shipped surface

`docs/10-workflow-io/export-formats.md` lists WebP, its Transparency control,
Quality for lossy WebP, and the Lossless control. `docs/11-cross-cutting/agent-control.md`
adds `export_webp` to the Save permission row. `docs/dev/STATE.md` replaces the
stale lossless-only ceiling with the #807 resolution.

## Risks

- A client that previously sent a negative quality to mean lossless now gets
  `invalid_param`. That spelling was never in the capability spec; the documented
  way to ask for lossless is `lossless: true`.
