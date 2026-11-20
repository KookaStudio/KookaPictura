# Design

## Context

The shipped path is a single RGB target: the dialog passes `select`, one
packed RGB, Fuzziness, and Invert to `color_range` (`crates/pictura-select/src/color_range.rs:104`),
which returns a `Selection`; the bridge shrinks a reduced composite for the
220 px thumbnail and streams full-width bands for OK
(`crates/pictura-app/src/cxxqt_object/color_range/imp.rs:53,90`). A streamed
band arrives with its first-row `offset` and a band-local buffer, so any
sample-position work must translate into that band and use the document's
geometry, not the band's. The canvas already has a display-only branch for
Quick Mask in `canvas_present_image`
(`crates/pictura-app/src/cxxqt_object/impl_core/display.rs:41-64`) with a
level/rect crop in `quick_mask_crop` (`quick_mask/imp.rs:422`).

Adjacent machinery this change should reuse rather than rebuild:

- Replace Color already carries multiple samples with canvas positions,
  Replace/Add/Subtract droppers, localization, and Selection/Image thumbnails:
  `ReplaceColorSample`/`ReplaceColorParams` (`pictura-core/src/content_params.rs:400-416`),
  `match_weight` and `sigma_sq` (`pictura-adjust/src/replace_color.rs:20-67`),
  and `replace_color_dialog.cpp:88-280`.
- The view holds the live proof setup (`ProofState`, `proofing/imp.rs:9-86`) and
  `pictura-color` can already produce a per-pixel gamut mask
  (`print_gamut_mask[_source]`, `proof.rs:142-189`).
- Qt Test reads the presented canvas through
  `PictureView::canvas_display_image` (`tst_quick_mask.cpp:80`), and the
  existing bounded preview is `image_size::preview_image`
  (`image_adjust/image_size.rs:34`).

Constraints: engine crates stay Qt-free; Fuzziness is 0–200 and the documented
Range is 0–100; 32-bpc documents keep the command unavailable; the CS6
algorithms for skin tones, localization, and the gamut rule are unpublished, so
each approximation is marked and tested as behaviour, not parity.

## Goals / Non-Goals

**Goals:**

- One settings-driven coverage evaluation with many samples and positions.
- Skin Tones, Localized Color Clusters + Range, Out Of Gamut.
- Replace/Add/Subtract sampling, Selection/Image thumbnails, the five
  image-window Selection Preview modes, and Save/Load.
- A preview that matches the committed intersection with a live selection.

**Non-Goals:**

- Detect Faces, Alt-click Reset, CS6 `.axt` import, Color Range availability
  for Bitmap/Indexed, Grayscale behaviour, Properties-panel layer-mask
  refinement, the "No pixels are more than 50% selected" outcome, and
  per-session persistence. These remain recorded ceilings.
- Changing Replace Color's observable behaviour while sharing its kernel.
- A whole-document preview mask or a new document field.

## Decisions

### Settings value and buffer-space samples

`pictura-select` exposes:

```text
ColorRangeSettings {
    select, samples: Vec<ReplaceColorSample>, fuzziness,
    localization, spatial_sigma, invert
}
color_range(img: &PixelBuffer, settings: &ColorRangeSettings) -> Selection
```

The existing `ReplaceColorSample { x, y, rgb }` is reused, so the shared kernel
takes the sample slice directly; a negative `x` or `y` means "no document
position" and carries no proximity weight. The engine never learns document
geometry: callers scale sample positions into the buffer's pixels, pass
`spatial_sigma` in those pixels, and pass `localization` in `0..=1`. Two small
helpers own the mapping: `spatial_sigma(diagonal_px) = 0.18 * diagonal_px` and
`localization_strength(range) = 1 - range / 100`. The dialog disables the
Range control while localization is off; per-session persistence remains a
recorded ceiling.

`ColorRangeSelect` gains only `SkinTones`, in CS6 list order after Sampled
Colors. That shifts later bridge indices, so the dialog items, `from_index`,
and the existing Qt Test `findData(7)` move together. Out Of Gamut is not an
engine variant; it is an app-level UI mode (see below).

Sampled Colors adopts the shared Chebyshev/fuzziness model instead of the
current normalised Euclidean distance. The docs leave the metric unspecified,
the existing band tests still hold, and one model now serves two dialogs.
Alternative rejected: a second distance formula and a second localization
kernel.

### Shared match kernel

Move `match_weight` and `sigma_sq` from `pictura-adjust` to `pictura-core`
(which already owns `ReplaceColorParams` and has no colour-management
dependency) and call them from both Replace Color and Color Range. The spatial
term becomes a strength rather than a boolean:

```text
weight *= 1 - localization * (1 - exp(-d^2 / (2 * spatial_sigma^2)))
```

`sigma_sq` is generalised to a diagonal fraction. Replace Color passes
`localization = 1.0` when localized and `0.0` otherwise, with its 0.18
fraction, so its output is unchanged. Color Range passes
`localization = 1 - range / 100`: Range 0 is a tight cluster, and Range 100
applies no spatial penalty, matching the documented "lowering Range rejects
matching colours far from the samples". The reverse mapping (sigma varying
with Range while the strength stays 1) is rejected because a Gaussian at
0.18 × diagonal is effectively zero at the far side of the image, so no
achievable Range would include a far cluster.

### Skin Tones

A chroma heuristic in BT.601 YCbCr, not a new detector: coverage falls off with
elliptical Cb/Cr distance from a skin centre, Fuzziness scales the reach, and
very low saturation, very dark, and very light pixels are excluded. Constants
are chosen so a canonical skin swatch is selected and saturated blue and
neutral grey are not. Marked as a behavioural approximation; Detect Faces stays
absent because the repository has no face detector and the CS6 algorithm is
unpublished.

### Out Of Gamut lives in the app, not the select crate

The mode is a colour-management test against the view's proof setup, so the
bridge recognises the dialog's Out Of Gamut index before mapping any other
index to `ColorRangeSelect`, and asks `pictura-color` for a gamut mask. The
proposed `GamutCheck` is built from the document's working ICC
(`doc.source.working_profile()`, falling back to sRGB) and the view's proof
setup, reused from `proofing::imp.rs::setup`, which becomes visible to the
sibling bridge modules. `pictura-color` gains the reusable `GamutCheck` (built
once per settings change) so a full-resolution apply does not reload profiles
once per streamed band; `print_gamut_mask_source` becomes a thin wrapper over
it with unchanged output. The mode is offered only for RGB documents, matching
the documented rule that CMYK documents do not offer it and that Grayscale and
Bitmap/Indexed are not colour-matching modes.

Alternatives rejected: adding `pictura-color` to `pictura-select` (pulls lcms2
into a pure selection crate for one mode), adding an engine variant the engine
cannot evaluate, and per-band profile construction (wasteful and difficult to
test).

### Preview surfaces

- Thumbnail: the bridge returns the mask or a bounded composite for the Image
  display, built from `picture::preview_display` (visible to `cxxqt_object`
  descendants); the generated `image_size::preview_image` is the C++ wrapper
  over the same path. The dialog refreshes the Image view only when the display
  mode changes, not on every slider step.
- Image window: a display-only `ColorRangePreviewState` on the view holding
  settings, sample positions, and the selected mode. `canvas_present_image`
  branches to a crop that matches `color_range`/`GamutCheck` on the presented
  level and rectangle, then renders None (skip the branch), Grayscale,
  Black Matte, White Matte, or Quick Mask. Matching always uses the unproofed
  composite, the same pixels OK commits; only the displayed matte may be
  proofed. Fully transparent pixels take no coverage, and premultiplied tile
  pixels are un-premultiplied before matching. The dialog sets the state on
  every control change and clears it on close.
  Alternative rejected: caching a document-resolution mask, which costs a
  canvas-sized buffer and keeps a stale copy on every edit.
- Live selection: preview coverage is multiplied by the current selection at
  document coordinates, in both surfaces. OK still commits the intersection,
  so the preview finally shows what will happen.

### Sampling gesture

`setCanvasSampler` takes a callback of
`std::function<void(const QPointF&, Qt::KeyboardModifiers)>` plus the
`continuous` flag. `ToolController::handlePressed` forwards the event's
modifiers; Replace Color ignores them and keeps its drag behaviour, while Color
Range samples once per press and maps Shift to Add and Alt to Subtract. Passing
modifiers with the event is required because the sampled click is not visible
to `QGuiApplication::queryKeyboardModifiers()` in an offscreen Qt Test.
Subtract follows Replace Color: drop the samples within Fuzziness of the click.

### Save / Load

A versioned JSON document (`format`, `version`, stable select name, Fuzziness,
localized, range, invert, and samples with positions and RGB) read and written
through path-based `saveSettings(path)` / `loadSettings(path)` helpers; the
`QFileDialog` is only a thin chooser over them, so tests drive the helpers
directly. Load rejects a malformed file, a wrong format, or an unsupported
version, clamps out-of-range values, and refuses a select mode unavailable on
the current document (Out Of Gamut on a non-RGB document) with an inline
message and unchanged controls. CS6's `.axt` is closed and is not imported.
Alternative rejected: the app-state preset store, because CS6's Save/Load is
file-based and must carry sample positions to reproduce a selection.

### Banded apply

`color_range_apply` streams bands through `stream_picture`. For each band the
bridge translates every sample's `y` by the band's `offset` and passes the
document-diagonal-derived `spatial_sigma`; the band's own diagonal is never
used. A test uses a document taller than one `TILE` (256 rows) so a missing
translation fails.

### File layout and bounds

The dialog is expected to stay under the 800-line target (currently 210) and
the Rust bridge under it too (currently 156); no new C++ file is planned. If
the dialog passes the target, Save/Load moves to a sibling `.cpp` registered
in `cmake/sources/dialogs.cmake`. The 220 px thumbnail keeps `preview_picture`
plus `shrink`; sample positions map through `>> level` and the shrink grid, one
pixel of rounding allowed.

## Risks / Trade-offs

- **Reduced-level previews differ from the full-resolution commit at edges** →
  Recomputing the exact mask on OK keeps the commit exact; previews are marked
  approximate, as the existing preview already is.
- **Localized matching is O(pixels × samples)** → Samples are user clicks, so
  the multiplier is small; banding keeps memory flat and Rayon rows are
  available if a large document shows a stall.
- **Canvas preview cost scales with the visible region** → Only while the
  dialog is open, only for the presented rect, and display-only; a drag can
  still be heavy on very large viewports, so the control changes remain the
  throttle. The Image thumbnail is bounded and refreshes only on a mode change.
- **Alt-drag can be intercepted by some window managers** → Documented
  limitation; the dialog's Subtract button remains the reliable path.
- **Skin-tone and Range constants are approximations without a CS6 oracle** →
  Tests pin the described behaviour, comments mark the approximation, and the
  docs record the remaining parity gap.
- **The gamut check on a reduced preview can flag a different pixel set than
  the full-resolution commit** → Same edge trade-off as above; the OK path
  uses the full-resolution check.
- **Shared helper move touches Replace Color** → A pure move with the existing
  Replace Color contract tests as the guard.
