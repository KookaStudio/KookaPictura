# Proposal

## Why

`Filter ▸ Pictura Raw…` currently force-converts a plain raster layer (or the
Background) into an embedded smart object before applying, so the filter is
always non-destructive and always retains its settings. Camera Raw in Photoshop
is destructive on an ordinary layer and only becomes a retained smart filter when
the target already is a smart object. Silently creating a smart object is
observable and wrong. #471 made a Background convertible, so the auto-convert now
fires on Backgrounds too — the wrong behavior is live, not prospective. This
change owns Pictura Raw's target-dependent behavior; #471 owns the conversion
predicate itself.

## What Changes

- Plain raster layer (including the Background): apply Pictura Raw
  **destructively** to the layer's own pixels over its `rect`, in one
  `Pictura Raw` history state. No smart object is created and no settings are
  retained (reopening the dialog starts blank).
- Embedded smart object: keep today's behavior — add/refresh the camera-raw smart
  filter and retain the settings.
- The Background keeps its Background identity (no unlock, no added alpha, no
  rename); only its color planes change.
- A new engine entry performs the destructive apply; a new bridge predicate
  governs menu enablement and is **decoupled** from `can_convert_to_smart_object`.
- Enablement covers any eligible raster pixel layer (RGB/Grayscale, any bit depth, not
  pixel-locked) and any replaceable embedded smart object; a group, adjustment layer,
  type layer, or channel-less layer stays ineligible.
- The dialog's live preview path is unchanged: `preview_pictura_raw` already
  handles a non-smart layer.

## Capabilities

### New Capabilities

- none

### Modified Capabilities

- `codec/pictura-raw`: adds the destructive-apply engine op and its no-retention
  contract; changes the bridge command to dispatch on target type instead of
  converting; changes menu-command enablement to a dedicated predicate that does
  not depend on `can_convert_to_smart_object`.

## Impact

- `crates/pictura-render/src/document_ops/pictura_raw.rs` — new
  `apply_pictura_raw_destructive`; `apply_pictura_raw` (smart-object path) and
  `preview_pictura_raw` unchanged. Re-exported from
  `document_ops/mod.rs` and `src/lib.rs`.
- `crates/pictura-app/src/cxxqt_object/impl_pictura_raw.rs` — bridge
  `apply_pictura_raw_filter` drops the convert-first step and dispatches by
  target; new `layer_can_apply_pictura_raw` predicate. The new method is declared
  in `crates/pictura-app/src/cxxqt_object.rs`.
- `crates/pictura-app/cpp/frame_menus.cpp` — `FilterPicturaRaw` enabled provider
  uses the new predicate.
- Tests: `tst_pictura_raw.cpp` (rewrite + the new enablement checks),
  `tst_acr_dialog.cpp` and `tst_layers_smart_filters.cpp` (setups that relied on
  auto-convert), plus engine unit tests in `pictura_raw.rs`.
- No new dependencies. No document-format change. Related: #471 (already merged).
