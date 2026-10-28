# Proposal

## Why

`Filter ▸ Digimarc ▸ Embed Watermark` and `▸ Read Watermark` are disabled stubs
in the command catalogue (`filter.digimarc.embed.watermark` /
`filter.digimarc.read.watermark`, `implemented: false`). There is no kernel and
no docs page. Adobe's Digimarc embedding called the licensed **Digimarc Barcode**
third-party service, so exact behavioral parity is impossible; issue #350 gates
the work on a scope decision (rule 8). The decision taken here is **option 2 — an
in-tree approximate watermark**: our own deterministic spread-spectrum scheme,
marked approximate everywhere, that never claims Digimarc compatibility. #351
(Read Watermark) ships in the same change so the payload round-trip is testable.

## What Changes

- Add a `digimarc` kernel to `pictura-filters`: a deterministic, in-tree
  spread-spectrum carrier over a layer's colour planes. A payload (watermark
  type, creator ID, ID) is expanded to a checksummed 66-bit codeword, repeated by
  a durability factor, and tiled across 8×8 cells; each pixel carries a
  pseudo-random ±1 chip scaled by a small amplitude. Reading correlates each cell
  against the same chip and majority-votes the codeword, validating a magic byte
  and CRC-8. It is **not** the Digimarc algorithm and does not interoperate with
  it.
- Add `Filter::EmbedWatermark { kind, creator_id, id, durability }` and a
  `Filter ▸ Digimarc ▸ Embed Watermark` catalogue row (four controls: Watermark
  Type, Creator ID, Image ID, Watermark Durability), so the existing generic
  filter dialog, live preview, undo, and Last Filter work unchanged.
- Add a `read_watermark` bridge method that inspects the active pixel layer and
  returns the decoded payload, and a bespoke `Filter ▸ Digimarc ▸ Read Watermark`
  handler that reports the result in a dialog.
- Show a copyright indicator in the document title when a watermark is detected,
  as `docs/02-ui-ux/application-frame.md` already specifies.
- Document the decision and behavior in `docs/06-filters/digimarc-watermark.md`
  (the `extract-and-pattern-maker.md` precedent) and record that the entry is an
  approximation, not parity.

## Capabilities

### New Capabilities

- `imaging/digimarc-watermark`: the in-tree approximate watermark payload, the
  spread-spectrum embed/read kernel, the Embed Watermark filter command and its
  CS6-shaped dialog, and the Read Watermark command.

### Modified Capabilities

- `ui/application-shell`: add the detected-watermark indicator requirement — a
  document whose watermark has been read shows a copyright symbol in its title.

## Impact

- New `crates/pictura-filters/src/digimarc.rs`; touched `filter/types.rs`,
  `filter/variants.rs`, `filter/apply.rs`, `catalogue/rows/*`, `catalogue/build/*`,
  `lib.rs`, and the test instances.
- `crates/pictura-app/src/cxxqt_object/filter_tools.rs`: add the
  `read_watermark` bridge method and its result row.
- `crates/pictura-app/cpp/`: a new read-result dialog, the Read Watermark handler
  in `frame_menus_filter.cpp`, and the title indicator in `document_window.cpp` /
  `frame.cpp`; `CMakeLists.txt` lists the new files.
- `crates/pictura-script/src/command_catalogue/entries.rs`: both Digimarc rows
  become implemented (with the golden table and shape counts updated).
- Docs: `docs/06-filters/digimarc-watermark.md`, `docs/dev/STATE.md`,
  `docs/dev/doc-code-parity-gaps.md`, `docs/06-filters/extract-and-pattern-maker.md`,
  and `ROADMAP.md`.
- No new dependencies. The rendered pixels change only by the watermark carrier
  (bounded by the durability amplitude) when the command is applied.
