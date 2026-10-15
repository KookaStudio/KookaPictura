# Design

## Context

See `proposal.md` for motivation. The pieces that constrain the approach:

- `PicturaRawSettings` (`crates/pictura-core/src/raw_settings.rs`) carries every
  tab. #394 (`20e9f695`) landed the render and settled the fields: Tone Curve,
  Detail, HSL/Grayscale, Split Toning, Lens (distortion, defringe, vignette),
  Effects (dehaze, grain, post-crop vignette), and Camera Calibration
  (primaries, shadow tint, profile). Panels expose the controls the reference
  screenshots list; a modelled field with no control (lens profile, perspective,
  rotate, offset) round-trips untouched through the text.
- **The render across the tabs is #394's and has landed.** This change does not
  touch `render_pictura_raw`; it calls it, unchanged, for the live preview.
- The bridge today is `apply_pictura_raw_filter(11 floats)` and
  `layer_pictura_raw_settings -> "11 tokens"` (`impl_pictura_raw.rs`); its width
  grows one-for-one with the tab set, which is the shallow interface this change
  removes.
- `HistogramPanel`, its `HistogramView`, and `filter_param_controls` are keyed to
  `PictureView` and to flat `FilterParamSpec` lists; only `HistogramView` takes
  pre-computed bins and is directly reusable (see decisions).

## Goals / Non-Goals

**Goals:**
- One constant-width bridge contract for the full settings tree, so adding a
  control never widens the bridge again.
- A dialog whose every modelled control round-trips and, for the renderable
  ones, moves the preview.
- Dialog split so each file stays under the LOC cap and is registered explicitly.

**Non-Goals:**
- Any render change: extending `render_pictura_raw` across the tabs is **#394's**
  job, done (`20e9f695`). This change makes no render-parity claim.
- The dialog toolbar tools (zoom, hand, white-balance eyedropper, targeted
  adjustment, crop, straighten, spot, red-eye, brush/graduated/radial).
- Presets persistence (stub menu only).
- Local adjustments, Snapshots, filmstrip, Workflow Options, output formats, raw
  file decode (#86).
- Widening the `docs/GLOSSARY.md` "Pictura Raw" definition or updating
  `docs/dev/STATE.md` (both stale after this change; `docs/` needs
  `TASK-ALLOWS-DOCS`).

## Decisions

### Serialized settings text is the bridge contract

`PicturaRawSettings` lives in `pictura-core`, so the text form is **free
functions in the codec**, not inherent methods: `pictura_codec::pictura_raw_to_text(&PicturaRawSettings) -> String`
and `pictura_raw_from_text(&str) -> PicturaRawSettings`. The bridge carries the
string opaque:

- `layer_pictura_raw_settings(path) -> QString` (full settings, `""` when none)
- `apply_pictura_raw_filter(path, settings_text) -> bool`
- `pictura_raw_preview(path, settings_text, max_dim) -> QImage`

The grammar uses the **same 4-byte `Fltr` keys** as the codec: `key=value` pairs
joined by `;`; numeric keys as decimal, text/enum keys as their stored bytes,
curve lists as comma-separated integers. Because a key may end in a space
(`Crv `, `LNR `, `CNR `, `STB `) or a text value may hold a trailing NUL, the
format escapes rather than trims: in both keys and values `\` → `\\`, `;` →
`\;`, `=` → `\=`, and any byte below `0x20` or at `0x7f` → `\xNN`. An empty
string means "no settings". A key's trailing space is part of the key, never
whitespace to strip.

The codec owns the canonical encode/decode. C++ mirrors the grammar in **one**
file (`crates/pictura-app/cpp/acr/acr_settings.{h,cpp}`): an `AcrSettings` value
map with `encode()/decode()`. The vocabulary necessarily exists in both
languages — the UI labels each control — so the win is that the *bridge
interface* stays constant as tabs grow, not that C++ is vocabulary-free. A Rust
round-trip test plus a committed fixture string both sides parse pin the two
mirrors together.

Alternatives rejected: a typed cxx-qt struct (the interface would become the
whole struct and churn codegen per field); Rust-side typed setters (a
pass-through layer atop the same map, duplicating the settings model in Rust and
C++); moving the text methods into `pictura-core` (it would put the on-disk key
vocabulary in the model crate, away from the codec that owns it).

### Panels are hand-built layout over one flat value map

Panel chrome is heterogeneous per the reference screenshots (sub-tabs,
right-aligned links, curve graphs, action menus), so panels are laid out by hand
per tab, each reading/writing the shared `AcrSettings` map. Each panel exposes
the controls the reference screenshots list for its tab; a modelled field the
reference does not show (lens profile, perspective, rotate, offset) is left to
round-trip untouched. A descriptor-table generator was considered and rejected
as premature: it would not express the bespoke chrome, so it would be a second
mechanism beside the hand layout.

### Preview and histogram

`pictura_raw_preview` resolves the layer at `path`, renders a downscaled copy
through the **#394 pipeline**, and returns it. For a plain raster layer it
renders the layer's pixels directly; for an embedded smart object it renders the
payload. It never converts the layer and never mutates the document — the smart
object conversion happens only on apply. (This reuses the resolution the
existing filter-preview proxy path performs, but for the raw settings tree rather
than a registered filter kind.)

The dialog's preview pane draws that image (transparency checkerboard, zoom
readout, and shadow/highlight clipping tints). The histogram is computed in C++
from the returned preview image, reusing `HistogramView` (which already takes
`Series{bins,color}`) — the surrounding `HistogramPanel` is not reused because it
reads `PictureView` pyramid pixels, not the previewed image. No new Rust bins
command is added.

The dialog is a `QDialog` embedding its own preview pane rather than using
`ImageView`'s canvas overlay (`setFilterPreview`), because ACR's preview lives
inside the dialog, not on the document canvas.

### File layout

`pictura_raw_dialog.{h,cpp}` becomes the chrome (tab strip, preview pane, action
bar); one `.cpp` per tab under `crates/pictura-app/cpp/acr/`
(`acr_basic_panel`, `acr_tone_curve_panel`, `acr_detail_panel`,
`acr_hsl_panel`, `acr_split_toning_panel`, `acr_lens_panel`, `acr_effects_panel`,
`acr_calibration_panel`, `acr_presets_panel`), plus `acr_preview_pane` and
`acr_settings` (the value-map encode/decode). Every file is added to the root
`CMakeLists.txt` explicitly (no globbing), both `.cpp` and `.h` (AUTOMOC), each
under the LOC cap.

## Risks / Trade-offs

- [Detail/effects render cost makes a drag lag on large layers] → preview renders
  a downscaled copy; the full-resolution render only runs on OK, mirroring the
  existing filter-preview proxy path (render cost itself is #394's concern).
- [The text grammar drifts between the Rust codec and the C++ mirror] → the
  escaping rules are specified here; a Rust round-trip test and a committed
  fixture string parsed by both sides pin them.
- [A modelled field drifts from the reference panel set] → panels expose the
  reference controls only; the settings text round-trips every field, so unshown
  fields (lens profile, perspective, rotate, offset) survive untouched.
- [Retiring self-test 528 breaks the self-test budget guard] → the budget is
  lowered in the same change, the check is unwired from
  `selftest_layers_controls.cpp`, and no exit code is reused.
- [Curve widgets are reused against a different model] → the tone-curve panel
  maps region sliders to their own control; the existing `CurveWidget` serves
  only the point-curve sub-tab.

## Migration Plan

Depends on #394 landing first. The bridge signature changes in one change, so
every caller moves together: `frame_menus.cpp`, the `#[qinvokable]` signatures,
and `tst_layers_smart_filters.cpp`. `Filter > Pictura Raw…` keeps its id, label,
and enablement; only the dialog it opens and the settings it carries change. No
document format changes (the same `Fltr` keys are written).

## Open Questions

- Ranges/defaults the CS6 Help PDF does not tabulate are `(community)`-marked in
  `docs/06-filters/camera-raw-filter.md` and can be tuned during implementation
  without changing the specs or the task shape.
