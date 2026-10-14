# Proposal

## Why

`Filter > Pictura Raw…` today opens an 11-field Basic modal
(`crates/pictura-app/cpp/pictura_raw_dialog.cpp`): a `QFormLayout` of
`QDoubleSpinBox`es with no preview and no histogram. The settings model and the
`Fltr` codec already model the PV2012 tab set the codec covers (#393), and the
render of the remaining tabs is delivered separately and first by #394. This
change is the last layer: replace the modal with the full tabbed ACR dialog shown
in the reference screenshots, over the render #394 provides.

## Dependencies

- **#394 has landed** (commit `20e9f695`, archived as `render-pictura-raw-tabs`):
  `render_pictura_raw` now applies every tab, and the model carries the
  calibration primaries, shadow tint, and Dehaze. This change does not modify
  `render_pictura_raw`; the live preview calls it unchanged.

## What Changes

- Replace the 11-field modal with the tabbed ACR dialog: an icon tab strip
  (Basic first) with a per-tab panel below it, a live preview pane, a histogram
  with clipping toggles and an RGB readout, and an OK/Cancel action bar.
- **BREAKING (internal bridge):** replace the 11-`f64` bridge contract
  (`apply_pictura_raw_filter`, `layer_pictura_raw_settings`) with an opaque
  serialized-settings form carried as a string, so the full settings tree
  round-trips without widening the bridge once per tab.
- Add a `pictura_raw_preview` bridge command that renders a downscaled copy of
  the layer through the existing pipeline for the dialog's preview pane.
- Move the dialog's GUI checks to Qt Test (`crates/pictura-app/cpp/tests/`) and
  retire the matching self-test check rather than adding new ones.

Panels expose the controls the reference screenshots list. A modelled field with
no panel control (lens profile, perspective, rotate, offset) round-trips
untouched through the settings text. This change makes no render-parity claim of
its own; the stages, including their `ponytail:` approximations, are #394's.

Non-goals: the dialog's toolbar tools (zoom, hand, white-balance eyedropper,
targeted adjustment, crop, straighten, spot, red-eye, brush/graduated/radial),
Presets persistence (stub menu only), local adjustments, Snapshots, filmstrip,
Workflow Options, output formats, raw **file** decode (#86), and any render
work (delivered by #394). The dialog stays a post-CS6 extension, outside the CS6
menu model.

## Capabilities

### New Capabilities
<!-- None: the full requirement set already lives in codec/pictura-raw. -->

### Modified Capabilities
- `codec/pictura-raw`: the "Pictura Raw dialog" requirement gains the tab strip,
  per-tab panels, live preview, and histogram (removing the "MUST NOT include
  ACR tabs / histogram / live preview" clause); the "Pictura Raw apply command"
  and "Pictura Raw settings readback" requirements carry the full settings
  rather than the 11 Basic values. The "Pictura Raw render" requirement is
  **not** modified here — it is owned by #394.

## Impact

- **Code:** `crates/pictura-codec/src/pictura_raw/` (a text encode/decode for
  the bridge); `crates/pictura-app/src/cxxqt_object.rs` (the `#[qinvokable]`
  signatures) and `crates/pictura-app/src/cxxqt_object/impl_pictura_raw.rs`
  (bridge contract); `crates/pictura-app/cpp/frame_menus.cpp` (the
  `Filter > Pictura Raw…` handler and its prefill/apply calls);
  `crates/pictura-app/cpp/pictura_raw_dialog.{h,cpp}` plus new per-tab panel
  files and a preview pane; `CMakeLists.txt` and
  `crates/pictura-app/cpp/tests/CMakeLists.txt` (explicit registration).
- **Tests:** new Qt Test suite(s) for the dialog; the 11-arg bridge callers in
  `crates/pictura-app/cpp/tests/tst_layers_smart_filters.cpp` updated; self-test
  `pictura_raw` (exit 528) retired and `scripts/selftest-budget.txt` lowered.
- **Dependencies:** none added. **Upstream:** #394 landed (`20e9f695`).
- **Docs:** `docs/GLOSSARY.md` defines "Pictura Raw" as "the 11 PV2012 Basic
  controls" and `docs/dev/STATE.md` says the dialog has no tabs/histogram/
  preview; both go stale and are **deferred** (`docs/` changes need
  `TASK-ALLOWS-DOCS`), not part of this change.
