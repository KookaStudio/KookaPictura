# Proposal

## Why

`Layer ▸ Smart Objects ▸ Convert to Smart Object` refuses a Background. In CS6
the same command works on a Background: it implicitly performs the "Background →
Layer" unlock and then converts, as one history state. Here the user must convert
the Background to an ordinary layer first (#471). The Layers-panel row menu also
omits the command on a Background row, since the row table gates it to the `pixel`
kind only.

## What Changes

- Relax `pictura-render::can_convert_to_smart_object`: drop the Background
  exclusion, keeping every other guard (8-bit document, non-group, no adjustment
  data, no existing smart object, positive `rect`).
- Make `convert_to_smart_object` unlock a Background first — clear the flag,
  release the locks, add the opaque alpha channel when absent, and rename to the
  document's next free `Layer N` — then run the embed. The engine step is silent,
  so the bridge still records exactly one `Convert to Smart Object` history state
  and the composite is unchanged. A `write_psd` failure still returns `false`
  without mutating the document.
- Show **Convert to Smart Object** on a Background row in the Layers-panel
  context menu (kind mask `Pixel` → `Pixel | Background`), enabled by the same
  relaxed predicate. The Layers-panel *flyout* menu's disabled entry is
  pre-existing scaffolding and stays out of scope.
- `Filter ▸ Convert for Smart Filters` shares the same engine operation, so it
  becomes available on a Background too; this follows unavoidably from keeping the
  predicate single-sourced (D1 in `design.md`) and is CS6-coherent.
- Update the in-place docs/tests: shrink self-test `lpr_smart_object_convert`
  (277) by removing its now-invalid Background-refusal assertion and cover the
  Background→smart-object case in a new Qt Test.

Out of scope (split to #478): `Filter ▸ Pictura Raw…` currently converts a plain
raster layer to a smart object and retains the settings; it should apply
destructively on a normal layer and retain the smart filter only on an existing
smart object. This change does not modify the Pictura Raw code, but relaxing the
shared predicate means that until #478 lands, applying Pictura Raw to a Background
converts it to a smart object like any other raster layer — an interim consequence
of the shared predicate.

Not breaking: an existing smart object, a group, an adjustment layer, and a
non-8-bit document still refuse.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/smart-object-layer-actions`: `Convert to Smart Object` eligibility
  no longer excludes the Background; the operation unlocks a Background before
  embedding; the command records one state on a Background. (The eligibility
  requirement is retired and re-added rather than edited in place — the OpenSpec
  validator forbids renaming a scenario under `MODIFIED`.)
- `ui/layers-panel`: a Background row offers the `Convert to Smart Object` row.
- `imaging/filter-app-ui`: `Convert for Smart Filters` is available on a
  Background and unlocks it before converting.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/smart_object.rs` (predicate +
  unlock step; reuses `create::release_background` / `create::next_layer_name`).
- `crates/pictura-app/cpp/panels/layers_panel_menu.cpp` (Background kind mask).
- `crates/pictura-app/cpp/selftest_layers_smart_object.cpp` (remove the
  Background-refusal assertion and its setup; the file only shrinks).
- `crates/pictura-render/src/tests/smart_object.rs` (the `"background"` refusal
  case moves to a positive test; also covers the `convert_for_smart_filters` alias).
- `crates/pictura-app/cpp/tests/tst_smart_object_actions.cpp` (new
  Background→smart-object case and group-row omission).
- `crates/pictura-app/cpp/tests/tst_command_tree.cpp` (the exact Background row
  list gains the new row).
- `crates/pictura-app/cpp/frame_menus.cpp` (stale comment only).
- No new dependencies. No `docs/` change. `Filter ▸ Pictura Raw` behavior is
  tracked in #478.
