# Proposal

## Why

The Layers panel currently draws a non-`None` color label as a narrow tinted
stripe inside the eye toggle plus a separate 3 px chip before the thumbnail.
Photoshop CS6 instead fills the whole visibility cell with the label color,
behind the eye glyph, and has no chip. The chip came from an earlier
`layers-panel-surface-completeness` change that misread CS6; this change
corrects the presentation and the spec that describes it (#722).

## What Changes

- A labeled row fills the **whole 30 px visibility column** with the opaque
  label color, behind the eye glyph, instead of tinting only a 12–16 px
  `eyeRect` at alpha 90.
- The color-label **chip is removed**, including its layout reservation, so the
  thumbnail and name start at the same x on labeled and unlabeled rows.
- The eye glyph flips between a dark and a light tint based on the label fill's
  luminance, so yellow, green, orange, and gray labels keep a legible glyph.
- A `None` label leaves the column on the normal gutter surface, unchanged.
- The `None` color-label name is unchanged.
- **BREAKING**: none. The label fill is paint-only and the visibility-column
  hit target is unchanged; removing the chip moves the labeled row's thumbnail
  and name hit rects to match unlabeled rows.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `ui/layers-panel`: the "Layer row badges and delegate" requirement is replaced
  by "Layer row badges, labels, and delegate" — a color label fills the whole
  visibility column, paints no chip, and keeps the eye glyph legible. The
  replacement lets the `lpr_label_chip` scenario be dropped.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_internal.h` — the label fill,
  chip constants/helpers/advance, and the optional glyph tint.
- `crates/pictura-app/cpp/panels/layers_panel.h` and
  `crates/pictura-app/cpp/panels/layers_panel_test.cpp` — remove the chip test
  accessor and sample the full eye column.
- `crates/pictura-app/cpp/tests/tst_layers_panel.cpp` — replace the chip
  assertions with full-column and no-reservation assertions.
- `openspec/specs/ui/layers-panel/spec.md` — updated by archiving this change:
  modify `lpr_label_tint` and `lpr_label_tint_none`, remove `lpr_label_chip`.
- No Rust change; no new dependencies.
