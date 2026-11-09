# Pattern fill thumbnail and mask from a loaded PSD

## Why

A Photoshop PSD that carries a Pattern fill layer shows a flat grey thumbnail in
the Layers panel instead of the tiled pattern (#1202). Photoshop writes every
pattern with **24 colour slots plus two alpha slots** in its
`VirtualMemoryArrayList`, writing only the planes the pattern uses. The decoder
(`crates/pictura-codec/src/patterns.rs`) instead requires the declared
`num_channels` to equal the image mode's colour-plane count (three for RGB), so
a real Photoshop pattern is skipped and the renderer falls back to the grey
placeholder. The same failed lookup makes the layer's composited content grey.

The issue also reported the layer mask thumbnail missing. That half is already
fixed on this branch by the canvas-wide/default-mask work (#1199/#1247); the
regression has no test, and the grey thumbnail still reproduces with a
Photoshop-authored sample.

## What Changes

- The pattern decoder reads `num_channels` as the **colour-slot count** and
  accepts Photoshop's 24 slots regardless of the mode's plane count. The first
  written colour slots are the pattern colour (one for Grayscale, three for RGB);
  the last written trailing slot is the alpha.
- Slots between the mode's colour count and the declared slot count are skipped
  without decoding, so a 24-slot pattern still decodes at most five planes.
- A colour-slot count above Photoshop's 24 is skipped (bounded, no allocation).
- The existing Grayscale/RGB scope is unchanged; Indexed and CMYK patterns still
  render the placeholder.
- A generated fixture mirroring Photoshop's 24-slot layout, with a Pattern fill
  layer and a painted layer mask, is added with a codec oracle test and a Qt
  Layers-panel test that assert a non-grey pattern thumbnail and a non-null mask
  thumbnail. The user-supplied Photoshop file is the reference, not a committed
  asset (the project generators author fixtures and reject Adobe-tool strings).

## Capabilities

- `imaging/adjustment-layer-rendering` — the Patt-resource decoder requirement
  gains Photoshop's 24-colour-slot layout.
- `ui/layers-panel` — a loaded Pattern fill row SHALL show the decoded pattern
  preview and its mask card.

## Impact

- `crates/pictura-codec/src/patterns.rs` — slot parsing, bounded decoding, and
  unit tests.
- `crates/pictura-codec/tests/fixtures/` — the new fixture and its README row.
- `crates/pictura-codec/tests/oracle/fills.rs` — fixture decode/round-trip.
- `crates/pictura-app/cpp/tests/tst_layers_snapshot.cpp` — the row thumbnail and
  mask assertion.
- No renderer change: `pattern_preview_rgba` and the compositor already consume
  `decode_patterns`; they start resolving once the pattern decodes.
