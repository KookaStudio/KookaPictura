# Design

## Context

See `proposal.md` — Why. Relevant current state:

- `pictura_render::apply_pictura_raw` (`document_ops/pictura_raw.rs:23`) is
  smart-object-only: it renders the embedded source, filters it, writes the
  proxy's color channels via `write_proxy`, and attaches the settings.
- `preview_pictura_raw` (`:95`) already renders a plain raster layer through
  `raster_preview_buffer` (`:150`), so the dialog needs no new preview path.
- The bridge `apply_pictura_raw_filter` (`cxxqt_object/impl_pictura_raw.rs:43`)
  converts a plain raster layer to a smart object first (using
  `can_convert_to_smart_object`) and then calls the engine op.
- `frame_menus.cpp:947` enables `Filter > Pictura Raw…` from
  `can_convert_to_smart_object || can_replace_smart_object_contents`.
- The engine edits an 8-bit working plane (`Layer.channels: Vec<u8>`); 16/32-bit
  documents retain native samples in `Layer.source_channels`, re-emitted on save
  only when the stored native plane still equals the working plane
  (`codec/write.rs:786-793`, the `native_plane` value guard; the store fetch is
  `layer_retained` at `:816`). An edit changes the working plane, so the guard
  fails and the edited 8-bit plane is widened — no explicit invalidation is
  needed. `apply_op_region` (`filter.rs:95-242`) is the reference: it edits
  `layer.channels` in place and never touches `source_channels`.

## Goals / Non-Goals

**Goals:**

- Split Pictura Raw into the two target-dependent behaviors the spec now
  requires, with no smart-object conversion on the raster path.
- Reuse the existing engine conventions for destructive pixel edits (RGB/alpha
  handling, grayscale replication) and the existing lock gate.

**Non-Goals:**

- Changing the smart-object attach path, the dialog, or the preview path.
- Adding color management, GPU acceleration, or a float pipeline for Pictura Raw.
- Changing `can_convert_to_smart_object` (#471 already relaxed it for Backgrounds).
- A selection/mask parameter on the destructive path (the generic filters take
  `Option<&LayerMask>`; the issue does not ask for one).
- Wiring the destructive apply into `last_filter` (Ctrl+F repeat); Pictura Raw
  does not set it today either.

## Decisions

### D1: A separate engine op, not a mode on `apply_pictura_raw`

Add `pictura_render::apply_pictura_raw_destructive(doc, path, settings) -> bool`
and leave `apply_pictura_raw` untouched. The two ops share resolve/reject
plumbing but differ in source and in whether they attach settings; a boolean
"mode" would make one function mean two things and force the documented
smart-only contract to change. Alternatives rejected: a dispatching
`apply_pictura_raw` (blurs the contract), and modelling Pictura Raw as a
`pictura_filters::Filter` for the generic `apply_filter` path (not possible —
Pictura Raw is `pictura_adjust::render_pictura_raw`, not a filter kind).

### D2: No bit-depth gate; no native-store invalidation

The working model is always 8-bit (`doc.depth` is always `BitDepth::Eight`; a
16/32-bit file records its depth in `source_depth` and retains typed samples in
`source_planes`/`source_channels`). Every filter edits the 8-bit working planes
and re-widens on save; `render_pictura_raw` being 8-bit is the engine norm, not a
Pictura Raw limitation. Eligibility is therefore depth-agnostic, and the op clears
nothing: `write.rs:786-793` already re-widens the edited plane because the
retained native plane no longer equals it. `raw_channels` must not be cleared —
it holds non-color channels (spot/user channels, `read.rs:618-636`) that this op
does not touch.

### D3: RGB and Grayscale only

`render_pictura_raw` is RGB(A)-only, and Camera Raw in Photoshop converts
CMYK→RGB and refuses non-RGB/Grayscale modes. `raster_preview_buffer` already
treats channels 0/1/2 as R/G/B, which would silently mis-filter a CMYK layer, so
both the op and the enablement predicate require RGB or Grayscale. Grayscale
mirrors `apply_op_region`: replicate channel 0 into the buffer, filter, and write
the result back to channel 0 only.

### D4: Bridge dispatches on target type; no conversion

`apply_pictura_raw_filter` deletes the convert-first block and dispatches:
embedded smart object → `apply_pictura_raw` (attach, retain); raster leaf →
`apply_pictura_raw_destructive` (no attach); otherwise false. Signature, record
label (`"Pictura Raw"`), and recomposite logic are unchanged, so the dialog,
`acr_preview_pane`, and history are untouched.

### D5: A dedicated enablement predicate that mirrors the op

Add `layer_can_apply_pictura_raw(path)` to the bridge, returning true for an
embedded smart object with a payload or a raster leaf that the op will accept:
not a group, no adjustment, no smart object, a present color channel, a positive
rect, an RGB/Grayscale document, and not pixel-locked. The predicate deliberately
mirrors the op's conditions so the menu never enables a target the op refuses.
`frame_menus.cpp` uses it instead of `can_convert_to_smart_object ||
can_replace...`, decoupling the menu from the conversion predicate.

### D6: Gate on the pixel lock, like the generic filters

`apply_op_region` refuses a `PIXELS`-locked layer (`filter.rs:105-107`); the
destructive op does the same. A Background from Photoshop normally carries
transparency/position locks rather than the pixel lock, so it stays eligible; a
type layer carries the pixel lock (`read.rs:899-903`), so it is excluded without a
separate type check. The op ignores the transparency lock because it never
changes alpha. (A Background locked with `PIXELS` would be refused, matching the
generic filters.)

## Risks / Trade-offs

- [The predicate and the op drift and the menu enables a target the op refuses]
  → D5 mirrors the op's conditions and the predicate gets a unit test; a Qt Test
  covers the raster/pixel-locked enablement.
- [16/32-bit save shadows edited pixels] → not a risk: `write.rs:786-793`
  re-widens the edited plane; a unit test saves and re-reads a 16-bit document
  after a destructive apply to prove it.
- [CMYK/other-mode layers silently mis-filtered] → mode gate in both the op and
  the predicate (D3).
- [Existing tests rely on auto-convert] → `tst_acr_dialog.cpp` and
  `tst_layers_smart_filters.cpp` must convert to a smart object before applying;
  `tst_pictura_raw.cpp` is rewritten for the new behavior.
