# Canvas tools honour the mask edit target

## Context

`begin_paint` already routes a stroke to `begin_mask_paint`, which runs an
ordinary `Stroke` on the mask-sized grayscale document from
`pictura_render::mask_document`; `paint_dab` / `end_paint` already dispatch on
the resulting `mask_stroke`. Only the entry points the canvas tools call were
missing the branch.

## Decision

- `begin_brush` and `begin_eraser` check `active_mask_target` first and call
  `begin_mask_paint` with `mask_stroke_config`, which maps both the foreground
  and the background to their Rec.601 luma (shared with `begin_paint`). The
  Eraser's mask stroke paints the background colour, Pencil mode aliased.
  Erase To History is refused on a mask (there is no mask history source); Block
  mode is approximated by the brush tip.
- The shared `begin` (other painting tools) and `fills::apply` refuse while a
  mask target is set. `edit_fill` keeps its own `fill_mask` branch ahead of
  `apply`.
- The Layers panel emits `editTargetChanged(bool mask)` when the view's mask
  target flips (thumbnail clicks, selection changes, document switches, a mask
  deleted underneath). The main window keeps the inactive target's FG/BG pair
  and exchanges it with the colour state on each flip, seeded black/white.

## Alternatives

- Route every painting tool to the mask: the clone, healing, smudge, and toning
  engines sample RGBA surfaces; wiring each to the grayscale mask document is
  larger than this fix and left for a follow-up.
- Per-document colour pairs: CS6 keeps one application-wide FG/BG, so the pair
  is per target kind, not per document.
