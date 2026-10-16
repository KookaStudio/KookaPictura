# Proposal: add-match-color

## Why

Issue #352 (from the #167 filter audit). `Image ▸ Adjustments ▸ Match Color`
is the last CS6 destructive adjustment still a disabled stub
(`ponytail: Match Color stays a disabled stub` in `frame_menus_adjust.cpp`).
The behavioral contract exists (`docs/04-image-ops/adjustments/match-color.md`,
ADJ-031): a statistics-transfer operation that matches a target image's colours
to a source image, layer, or selection, with Neutralize, Luminance, Colour
Intensity, and Fade modifiers. There is no photorust source, so this is a new
engine.

## What Changes

- **`pictura-adjust`**: a new `match_color` module — `MatchColorParams`
  (target/source statistics, `neutralize`, `luminance` `1..=200`,
  `color_intensity` `1..=200`, `fade` `0..=100`) and
  `Adjustment::MatchColor`. Each pixel is transferred in CIELAB by the
  Reinhard-style mean/std mapping, then Luminance scales `L`, Colour Intensity
  scales `a`/`b` (1 is exactly achromatic), and Fade blends the result toward
  the original. `match_color_stats` computes weighted Lab statistics over
  an RGBA image, optionally restricted to a selection plane. Dialog-only, so
  there is no PSD encoding and native-depth apply refuses it, like Replace
  Colour.
- **`pictura-app`**: an `image_match_color` bridge — target statistics from the
  active layer, source statistics from another document's merged composite or
  a chosen layer (both optionally restricted to that document's selection),
  preview/commit through the shared adjustment preview machinery, and a new
  `ignore_selection` flag on the apply path for the "Ignore Selection When
  Applying Adjustment" option. A modal `MatchColorDialog` (Source and Layer
  combos, the three selection checkboxes, Neutralize, the three sliders,
  Preview, Save/Load Statistics) replaces the stub wiring.
- **Tests**: `pictura-adjust/tests/match_color.rs`, bridge tests, a new
  `tst_match_color` Qt suite, and `tst_image_adjustments` flips its
  "Match Color stays disabled" assertion.

## Capabilities

### Added Capabilities

None (new requirement in an existing capability).

### Modified Capabilities

- `imaging/image-adjustments`: adds the "Match Color adjustment" requirement.
  The dialog-enablement requirement is unchanged; Match Color satisfies it.

## Impact

- Crates: `pictura-adjust`, `pictura-render` (type re-export),
  `pictura-app` (bridge + dialog + menu wiring). No new dependency; the Lab
  conversion is implemented locally in `pictura-adjust` (consolidation into
  `pictura-color` can follow #269).
- Two of the draft doc's open questions are resolved by this change and pinned
  in `design.md`: the Fade direction (Help-sourced: right reduces, so 0 is the
  full match and 100 is identity, default 0) and the Save/Load Statistics
  format (an in-tree binary format; Adobe's is undocumented).
- The mean/std transfer in Lab approximates Adobe's closed kernel
  (`ponytail:` in code); no parity is claimed.

## Provenance

- Behavioural facts from the CS6 Help (RGB-only, dialog controls, Luminance /
  Colour Intensity 1–200/100, Fade direction, Save/Load Statistics) as cited
  in `docs/04-image-ops/adjustments/match-color.md`.
- The mean/std statistics-transfer model follows Reinhard et al., *Color
  Transfer between Images* (2001), as the docs contract proposes. Adobe's
  algorithm and colour space are unpublished.
- Ceiling (`ponytail:`): the Info-panel eyedropper feedback while the dialog
  is open is not implemented; the dialog is modal like the other adjustment
  dialogs. Cross-profile statistics use the working composite without profile
  conversion.
