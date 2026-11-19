# Design: native-depth-tone-tools

## Context

A 16-bpc document read from PSD/PSB keeps an 8-bit working model
(`Document::depth == Eight`) plus a retained native store
(`Document::source.depth() == Some(Sixteen)` and each layer's
`SourceChannels`). The compositor and the writer read the retained store when
it is authoritative (`Document::authoritative_plane`). The existing fill path
already resolves this with `fill_native::native_plan` and writes the store at
its own depth, then re-narrows the 8-bit mirror.

The tone engine (`crate::tone::ToneBrush`) works on an 8-bit `RgbaImage` loaded
by `LazyPixels`, and `Stroke::begin_tone` refuses every depth other than
`BitDepth::Eight`. That refusal misses loaded 16-bpc documents (their working
depth is 8) and also misses loaded 32-bpc documents (their working depth is 8
but their source depth is 32).

## Goals / Non-Goals

**Goals:**

- A 16-bpc Dodge, Burn, or Sponge stroke writes the retained native colour
  planes at 16 bits, so the canvas and a later save see the edit.
- Grayscale uses the store's single colour plane.
- Selection coverage is applied in native units.
- A cancelled stroke restores the native samples.
- 32-bpc documents are refused by the engine even when the depth is a retained
  source store.

**Non-Goals:**

- A general native-depth painting model for the other retouch tools (#551).
- GPU tone dabs; the CPU path remains the reference.
- Changing the 8-bit tone arithmetic or the existing tone curves.

## Decisions

**Reuse the fill depth plan.** `Stroke::begin_tone` calls
`fill_native::native_plan(doc, path, gray)`. `Store(BitDepth::Sixteen)` selects
the native brush; `Copy` keeps the existing 8-bit `ToneBrush`; `Refused` is
rejected. A retained `BitDepth::ThirtyTwo` source store and a 32-bit working
document are refused before the plan. A constructed 16-bit document with no
native store falls back to the 8-bit path, which is the copy the compositor
reads.

**A sibling brush, not a generic sample kernel.** `tone_native::NativeToneBrush`
mirrors `ToneBrush`'s dab loop, coverage cap, Airbrush rule, and options, but
reads and writes `u16` planes from `Layer::source_channels` and writes the
narrowed result into the per-dab `RgbaImage` so the existing `write_back`,
dirty tracking, and history capture keep working. It reuses the pure `tone` and
`sponge` functions so the model is identical. Native dabs run on one thread;
the 8-bit path keeps its banded raster. `ponytail:` a 16-bpc dab is serial
until the native planes have a parallel row split.

**Selection coverage at native depth.** The 8-bit path applies the selection in
`stencil::mask_dabs` after the dab. The native brush instead samples the same
`Coverage` per pixel and blends the 16-bit result with the pre-dab native value
before narrowing. `apply_per_dab` skips `mask_dabs` for the native engine, so
the coverage is not applied twice.

**Saved native tiles for cancel.** `Saved` gains an optional native snapshot
keyed by the same 64×64 tile grid. `begin_tone` enables it for the colour plane
ids. `Saved::save` copies the pre-dab native tile beside the 8-bit tiles,
`restore` writes them back, and `grow` pads them when a stroke grows the layer.
History already treats native stores as copy-on-write metadata, so an
undo/redo restores them without a new history mechanism.

**Dirty on native-only changes.** A 16-bit change can narrow to the same 8-bit
byte. `apply_per_dab` unions the native dab's dirty rectangles into the stroke's
`written` box so the stroke still commits one history state and refreshes the
canvas.

## Risks / Trade-offs

- [The native saved planes allocate a full layer-sized zeroed buffer per colour
  plane] → the allocator leaves untouched pages uncommitted, matching the
  existing 8-bit `Saved` planes; only tiles a dab reaches are written.
- [Serial native dabs are slower than the 8-bit banded raster] → the native
  path is new and correctness-first; the `ponytail:` comment names the ceiling.
- [A store authoritative for only some colour planes] → the fill path already
  refuses that shape; tone does the same instead of writing a partial layer.
- [The 8-bit path changes for a 16-bpc document with no native store] → it does
  not; `Copy` keeps the existing `ToneBrush` and bytes.
