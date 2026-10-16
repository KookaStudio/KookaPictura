# Proposal: port-photorust-lens-flare

## Why

Kooka's Lens Flare is a placeholder: a Gaussian core, a few grey ghosts, and
pixel-sized geometry (`center · (w − 1)`), so a flare on a small proxy and on
the full image are different pictures. Its dialog is the generic slot dialog:
a small pad, two numeric boxes for the centre, and a Lens Type combo. CS6's
dialog is a large preview with a draggable crosshair, Brightness, and a Lens
Type radio group. Its live canvas preview also renders against the visible
crop, so the centre lands in the wrong place (#168). Issue #225 asks to port
photorust's Lens Flare and give it a dedicated dialog. Lighting Effects, the
other half of #225, is a separate change.

## What Changes

- `render::lens_flare` becomes photorust's model, moved to
  `render/lens_flare.rs`. It adds an inverse-square core in a soft glow, a
  halo ring, rays (six, eight, or four) or the Movie Prime's anamorphic blue
  streak, and tinted hexagonal ghosts and rings strung from the flare through
  the middle of the frame. Every size is a fraction of the half-diagonal and
  the centre a fraction of the frame, so the flare is size-invariant.
- `Filter::LensFlare`, `LensType`, the `lens-flare` kind, its four slots
  (brightness, centre x, centre y, lens), and its defaults are unchanged.
  Brightness outside 10..=300 is still rejected.
- Lens Flare previews against the whole layer (`filter_preview_needs_whole_layer`),
  fixing #168.
- Filter ▸ Render ▸ Lens Flare… and Last Filter Settings open a dedicated
  `LensFlareDialog`. It has a 250 px preview of the whole picture with the
  flare drawn on a proxy under a draggable crosshair, OK / Cancel / Preview
  to the right, a Brightness field over a slider, and a Lens Type radio
  group.

## Capabilities

### Modified Capabilities

- `imaging/render-filters`: Lens Flare is photorust's size-invariant model.

### New Capabilities

- `imaging/filter-app-ui` gains *Lens Flare dialog*: the dedicated dialog and
  whole-layer preview.

## Impact

- `pictura-filters` `render`; `pictura-app` `filter_tools.rs`,
  `frame_menus_filter.cpp`, and the new `lens_flare_dialog.{h,cpp}` plus
  `tst_lens_flare_dialog`.
- **Output changes:** every Lens Flare result changes. No golden baseline
  covers Lens Flare.
- **Oracle:** Lens Flare stays no-equivalent. It is covered by property tests
  ported from photorust and by Qt Test cases for the dialog. No new self-test
  checks (rule 11).
- **Docs:** `docs/06-filters/render-filters.md` describes the model and the
  dialog, in a separate `TASK-ALLOWS-DOCS` commit.
