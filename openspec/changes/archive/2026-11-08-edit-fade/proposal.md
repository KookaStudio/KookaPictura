# Edit > Fade

## Why

`Edit ▸ Fade…` is an inert leaf in the command catalogue and has no handler.
Photoshop's Fade changes the opacity and blend mode of the most recently applied
destructive effect without re-running it, which is the documented way to soften
an Emboss, sharpen, blur, adjustment, brush stroke, or erase. The behaviour is
specified in `docs/06-filters/filters-overview.md` and the gap is recorded as
SYS-3 in `docs/dev/doc-code-parity-gaps.md` (issue #491).

## What Changes

- A new `pictura-filters::fade` compositor takes the pre-effect and post-effect
  layers plus `Fade { opacity, mode }` and returns the faded layer. It is a
  W3C source-over composite with the selected blend mode, so 0 % restores the
  original pixels, 100 % Normal reproduces the effect, and alpha is handled.
- The document session retains the pre-effect and post-effect layer pixels and
  the effect name after a destructive filter, Image adjustment, brush/pencil
  stroke, eraser stroke, or Magic Eraser commits on a pixel layer. The record is
  discarded by any later pixel, structural, undo/redo, or history-restore edit.
  Smart Filters, layer-mask targets, and Quick Mask targets do not create one.
- `Edit ▸ Fade…` gains a handler, an enabled provider, and a dynamic label
  (`Fade Gaussian Blur…`). It opens a dialog with Opacity 0–100, the 27 layer
  blend modes, and a Preview checkbox. OK records one `Fade <effect>` history
  state; Cancel restores the pre-dialog pixels bit-identically.
- Reopening Fade prefills the last opacity and mode and recomputes from the
  original effect output rather than compounding on the already-faded pixels.
- On a Lab document the six restricted modes (Color Dodge, Color Burn, Lighten,
  Darken, Difference, Exclusion) are shown disabled.

## Capabilities

- `imaging/filter-fade` — new capability: the fade compositor, the retained
  last-effect record, the command availability rules, the dialog, and the
  commit/reopen semantics.

## Impact

- `crates/pictura-core` — the W3C blend function moves from `pictura-render` to
  `pictura-core` so the filter crate can share it without a render dependency.
- `crates/pictura-filters` — new `fade` module and `Fade` type.
- `crates/pictura-app` — fade state in `PictureViewRust`, capture hooks in the
  filter/adjustment worker and the paint/erase paths, the cxx-qt bridge, the
  command wiring, a new `FadeDialog`, and Qt/Rust tests.
- `ROADMAP.md`, `docs/dev/STATE.md`, and `docs/dev/doc-code-parity-gaps.md`
  record the shipped feature and drop SYS-3.
