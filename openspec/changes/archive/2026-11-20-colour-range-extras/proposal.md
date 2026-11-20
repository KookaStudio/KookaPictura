# Proposal

## Why

Select > Color Range shipped as a sampled-colour band selector only. The
behavioural contract (`docs/08-selection/color-range.md`, `SEL-005`) specifies
Skin Tones, Localized Color Clusters with a Range slider, Out Of Gamut,
Selection/Image previews, the image-window Selection Preview modes, plus/minus
sampled colours, and Save/Load of settings. The parity register records the
whole set as missing (`docs/dev/doc-code-parity-gaps.md`, row `SEL-5`), and the
current ceilings are marked in `crates/pictura-select/src/color_range.rs:7` and
`crates/pictura-app/cpp/color_range_dialog.h:23`. Issue #613 asks for them.

## What Changes

- `pictura-select` evaluates Color Range from a settings value instead of one
  RGB triple: an ordered sample list with document positions, Fuzziness,
  Localized Color Clusters plus a 0–100 Range, Invert, and the existing hue and
  tonal bands.
- Sampled Colors adopts the same match model as Replace Color: best Chebyshev
  distance across samples, optionally weighted by a spatial Gaussian around
  each sample. The shared helper and its sigma move to `pictura-core` so both
  dialogs use one kernel; Range scales the spatial falloff strength.
- Add **Skin Tones** as a select mode: a chroma-space skin heuristic with
  Fuzziness widening the band, marked as a behavioural approximation.
- Add **Out Of Gamut**: an RGB-document mode evaluated against the view's
  current proof setup (Working CMYK by default) through `pictura-color`'s gamut
  check. The item is offered only on RGB documents.
- The dialog gains Replace / Add / Subtract eyedroppers (Shift adds and Alt
  subtracts temporarily), Localized Color Clusters with its Range slider, Skin
  Tones, a Selection / Image display preview, the Selection Preview combo
  (None / Grayscale / Black Matte / White Matte / Quick Mask) applied to the
  image window, and Save/Load settings.
- The dialog preview honours a live selection: it shows the intersection OK
  will commit, instead of the unclipped mask it shows today.
- The image-window Selection Preview is a display-only state on the view; the
  canvas computes candidate coverage for the presented level and rectangle,
  then renders the selected mode. The document composite and selection are
  untouched until OK.
- Save/Load writes a versioned JSON settings file including sample positions
  and colours. CS6's `.axt` binary format stays closed.

Out of scope, and recorded as remaining ceilings: Detect Faces (no face
detector exists and the CS6 algorithm is unpublished), Alt-click Reset,
CS6 `.axt` import, Color Range availability for Bitmap/Indexed documents,
Grayscale behaviour, Properties-panel layer-mask refinement, the "No pixels are
more than 50% selected" outcome, and per-session persistence of the dialog
settings. 32-bit documents remain unavailable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/color-range`: the coverage requirement expands from a single sampled
  colour and fixed bands to the CS6 settings set; the dialog requirement gains
  the samplers, localized clusters, gamut mode, both preview surfaces, and
  Save/Load, and states that previews match the committed intersection.

## Impact

- `crates/pictura-core`: shared colour-match/sigma helper (moved from
  `pictura-adjust`, no external dependency).
- `crates/pictura-select`: `ColorRangeSelect` grows `SkinTones`; Out Of Gamut
  stays in the app bridge; `color_range` takes a settings value with samples.
- `crates/pictura-app`: Color Range bridge and dialog; sample parsing; view
  proof setup reuse; canvas display branch for the Selection Preview modes.
  If the dialog outgrows its file ceiling, Save/Load splits into a sibling
  `.cpp` and is registered in `cmake/sources/`.
- `pictura-adjust`: Replace Color calls the moved shared helper; behaviour is
  unchanged.
- Documentation: `docs/dev/doc-code-parity-gaps.md`, `docs/dev/STATE.md`, and
  `ROADMAP.md` record what shipped and what remains; the docs change is covered
  by `TASK-ALLOWS-DOCS` in the commit.
- No new external dependencies and no new document fields.
