# De-Interlace

## Why

`Filter ▸ Video ▸ De-Interlace` is a disabled stub. The command catalogue carries
`filter.video.de.interlace` with `implemented: false`, and there is no engine
kernel, no catalogue row and no parameter descriptor behind it, so the Video
submenu ships one working entry (NTSC Colors, #345) beside one that does nothing.
Issue #344 tracks the gap. `docs/06-filters/filters-overview.md` already lists the
filter for 8-, 16- and 32-bit documents and describes it as removing interlacing
from video frames by eliminating or interpolating **Even** or **Odd** fields.

## What Changes

- A row-pair kernel in `pictura-filters::video` that reconstructs one interlaced
  field from the other, with a field choice (Odd or Even) and a method
  (Interpolation or Duplication).
- The `Filter::DeInterlace` variant, its `FilterKind` row, its dispatch arm, and
  the exhaustive engine test tables.
- A catalogue row and a slot builder, so the generic filter dialog and the menu
  row both come from one descriptor, as they do for every other catalogue
  filter.
- The command catalogue flips `filter.video.de.interlace` to implemented. The Qt
  stub test moves to a leaf that still has no kernel (`Filter > Extract…`).
- An OpenSpec delta for `imaging/video-filters`, plus a MODIFIED
  `imaging/filter-app-ui` scenario that currently names De-Interlace as its
  disabled example.
- `ROADMAP.md`, `public_docs/filter-video-other-digimarc.md`,
  `docs/dev/STATE.md` and `docs/dev/doc-code-parity-gaps.md`.

## Capabilities

- `imaging/video-filters` — extended with the De-Interlace kernel and command.
- `imaging/filter-app-ui` — the "Filter menu wiring" requirement's disabled
  example changes to a leaf that is still unimplemented.

## Impact

- `crates/pictura-filters` — new variant, kernel, catalogue row, builder, and
  the exhaustive test tables.
- `crates/pictura-script` — one command row, the golden catalogue, the unwired
  list and the implemented count.
- `crates/pictura-app` — mapping and descriptor tests, one Qt test, and the
  whole-layer preview note.
- No C++ menu or dialog code: the descriptor drives both.
