# Proposal

## Why

The CS6 Filter menu is inert: `command_tree.cpp` registers its whole tree as
`implemented = false` stubs and nothing calls `setHandler`, so the ~90 filter
kernels already in `pictura-filters` (plus the six kernels ported in
`port-missing-filter-kernels`) are reachable only through the control server and
tests. The engine is done; the missing half is the generic filter dialog and the
menu wiring that drives it, which is the remaining scope of issue #82.

## What Changes

- Add a generic parameterised filter dialog (ported from photorust
  `shell/src/dialogs/FilterPreviewDialog.{h,cpp}`): a CS6-style layout —
  thumbnail with magnifier zoom icons and a percentage readout, a Preview
  checkbox, one control per parameter (slider with its value box on the label
  line, angle wheel, choice/radio, range, checkbox, colour, shear curve, distort
  grid, kernel grid), and Radial Blur's Blur Center / Lens Flare placement
  layouts.
- Add a runtime parameter path across the bridge: build a `Filter` from
  `kind` plus a slot list of values, instead of only the fixed defaults.
- Add live canvas preview: a parameter change renders the filtered result on the
  canvas without recording history; OK commits one history state, Cancel
  restores the pre-filter pixels bit-identically. The preview is bounded to the
  visible document section (plus the filter's support), and the dialog thumbnail
  shows that section at the canvas zoom, so large documents preview at the cost
  of a viewport.
- Filter a layer's transparency with the same kernel when it is not
  transparency-locked (CS6), so a blur softens the layer's edges; a
  transparency lock keeps alpha and skips clear pixels.
- Wire the CS6 Filter menu: every entry with an engine kernel becomes
  implemented and opens its dialog (parameterless filters apply directly), and
  every dialog-opening entry carries a trailing ellipsis. The entries with no
  kernel (Filter Gallery, Liquify, Vanishing Point, Lens Correction, Adaptive
  Wide Angle, Extract, Pattern Maker, Browse Filters Online, Convert for Smart
  Filters, Blur Gallery, Lens/Smart Blur, Reduce Noise, Glass, Diffuse Glow,
  Diffuse, Glowing Edges, De-Interlace, NTSC Colors, Digimarc Embed/Read
  Watermark) stay disabled stubs.
- Make filters work on single-channel (Grayscale) layers: `apply_filter`
  replicates channel `0` across its working planes and writes back to channel
  `0`, so a Grayscale document no longer silently no-ops.
- Add Last Filter (`Ctrl+F`, re-applies with the last settings) and Last Filter
  Settings (`Alt+Ctrl+F`, reopens the dialog prefilled), including the dynamic
  menu label and enablement.
- Present every app dialog without a modal window hint, so the compositor's
  "Dialog Parent" dim never fires; the shell blocks the parent's input with an
  event filter and runs the dialog's event loop instead.

## Capabilities

### New Capabilities

- `ui/dialog-presentation`: app dialogs are shown non-modally with a parent
  input blocker rather than a compositor-dimming modal hint.

### Modified Capabilities

- `imaging/filter-app-ui`: the filter-kind mapping gains a runtime-parameter
  form; adds the parameter dialogue, live non-committing canvas preview,
  Filter-menu wiring (with dialog ellipses), and Last Filter / Last Filter
  Settings requirements.
- `imaging/filter-application`: color-channel extraction gains the Grayscale
  (single-channel) path.

## Impact

- `crates/pictura-render/src/filter.rs`: filter single-channel Grayscale layers
  in addition to the color-layer path; filter an unlocked layer's transparency;
  add `apply_filter_region`/`preview_apron` for viewport-bounded previews.
- `crates/pictura-app/src/cxxqt_object/filter_map.rs`: add
  `filter_from_kind_params(kind, &[f64])`, the `FILTER_ARITIES` table, and
  `filter_param_arity`; `helpers.rs`'s `filter_from_kind` delegates to it with an
  empty slot list.
- `crates/pictura-app/src/cxxqt_object/filter_tools.rs`: add the
  `apply_filter_params`, `filter_preview`, `filter_preview_cancel`, last-filter,
  and `filter_target_ready` bridge methods.
- `crates/pictura-app/src/cxxqt_object/impl_filters.rs`: add parameterised
  apply, non-committing preview, preview cancel, and last-filter state.
- `crates/pictura-app/cpp/`: new `filter_preview_dialog.{h,cpp}` and
  `filter_commands.{h,cpp}`; new `frame_menus_filter.cpp`; `command_tree.cpp`
  keeps the hand-written CS6 Filter tree and `frame_menus_filter.cpp` lights up
  its implemented rows from the table; `frame_menus.cpp` delegates.
- `crates/pictura-app/cpp/dialogs.{h,cpp}`: add `runDialog`; route every app
  dialog's `exec()` through it.
- `CMakeLists.txt`: add the new `.cpp`/`.h` files explicitly.
- `crates/pictura-app/cpp/tests/`: new `tst_filter_menu` Qt Test suite.
- Docs: `docs/dev/STATE.md` resume note, `docs/dev/m6c-filter-integration.md`
  Grayscale semantics, `docs/02-ui-ux/application-frame.md` dialog rule.
- No new dependencies.

