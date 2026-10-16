# Proposal: image-adjustments-menu

## Why

Issue #83 (reported again from the app: "all Image > Adjustments options are
grayed out"): the Image > Adjustments submenu and Image > Auto Tone / Auto
Contrast / Auto Color were 28 `leaf(...)` stubs with no handler. Ported from
photorust's per-adjustment dialogs (`LevelsDialog.cpp`, `CurvesDialog.cpp`, …).

## What Changes

- `pictura-adjust`: `Adjustment::Equalize` — one lookup from the cumulative
  histogram of every colour sample (behavioural approximation; CS6's is
  unpublished).
- `pictura-render`: `apply_adjustment_region` shares the Filter menu's masked,
  region-bounded write-back but leaves transparency alone;
  `default_adjustment_block` gives each of the fourteen dialog kinds its CS6
  dialog defaults (Gradient Map runs foreground → background).
- `cxxqt_object/impl_filters.rs`: the active-layer apply / preview / baseline
  path takes an `ActiveOp` (filter or adjustment), so adjustments get the
  same layer checks, selection masking, and exact Cancel.
- `cxxqt_object/image_adjust.rs` (new bridge): dialog blocks (default, page,
  set, curve), preview over the visible section, apply as one state named for
  the adjustment, and the direct commands.
- `panels/adjustment_controls.*` (new): the descriptor-driven controls moved
  out of the Properties panel, shared by the panel and the new
  `adjustment_dialog.*` (controls, Preview, OK / Cancel).
- `frame_menus_adjust.cpp`: Brightness/Contrast…, Levels… (Ctrl+L), Curves…
  (Ctrl+M), Exposure…, Vibrance…, Hue/Saturation… (Ctrl+U), Color Balance…
  (Ctrl+B), Black & White… (Alt+Shift+Ctrl+B), Photo Filter…, Channel Mixer…,
  Posterize…, Threshold…, Gradient Map…, Selective Color… open dialogs; Invert
  (Ctrl+I), Desaturate (Shift+Ctrl+U), Equalize, and Auto Tone / Contrast /
  Color (in the submenu and on the Image menu) apply at once. All are enabled
  when the active layer can take a destructive edit.
- Tests: Equalize and `apply_adjustment_region` unit tests, the dialog-default
  test, and the Qt Test `tst_image_adjustments` (on an opened image).

## Capabilities

### New Capabilities

- `imaging/image-adjustments`: the Image > Adjustments menu.

## Impact

- `pictura-adjust`, `pictura-render`, `pictura-app` (bridge, C++). No new
  dependency.

## Provenance

Dialog set and defaults from photorust and CS6 Help. Ceiling (`ponytail:`):
Color Lookup, Shadows/Highlights, HDR Toning, Match Color, and Replace Color
stay disabled; the dialogs have no presets, Auto / eyedropper buttons,
histograms, or Load / Save; Equalize ignores CS6's selection-only options.
