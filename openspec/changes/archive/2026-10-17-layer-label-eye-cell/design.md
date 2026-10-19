# Design

## Context

See `proposal.md` for motivation. The label paint lives in
`LayerRowDelegate::paint`: it fills the 30 px gutter with `gutterSurface`, fills
`eyeRect` with the label at alpha 90, then draws the eye glyph and the gutter
separator. A separate chip uses `labelChipAdvance()` in `paint` and
`imageThumbBox`, and `thumbRect`/`nameRect` inherit that advance. The eye glyph
is the `layers.eyeOn`/`layers.eyeOff` SVG rendered by `SvgIconEngine` in the
palette's `ButtonText` at 80% opacity.

## Goals / Non-Goals

**Goals:**

- Fill the whole visibility column with the label color at full opacity.
- Remove the chip and its layout reservation so labeled and unlabeled rows share
  the same thumbnail and name x.
- Keep the eye glyph legible on every one of the seven CS6 label colors.
- Leave unlabeled rows unchanged, keep the eye-column hit target, and align the
  content hit rects with unlabeled rows.

**Non-Goals:**

- Changing the label palette, the `None` menu name, or the Blend If chip.
- Changing the eye column's hit target or the row selection surface.
- Any Rust or document-model change.

## Decisions

**Fill the full column before the row rule.** The label fill becomes
`QRect(rect.left(), rect.top(), kEyeColumn, height)` painted after the gutter fill
and before the 1 px bottom rule, glyph, and separator. Painting it before the
rule keeps the row separator continuous across the eye column. The fill uses the
label color unchanged (no alpha), matching the CS6 reference in issue #722.

**Replace the requirement instead of modifying it.** OpenSpec 1.13.2 rejects a
`MODIFIED` requirement that omits an existing scenario, and `lpr_label_chip` must
go. The delta removes "Layer row badges and delegate" and adds its successor,
"Layer row badges, labels, and delegate", carrying every other scenario over
unchanged. Alternative: keep a stale `lpr_label_chip` scenario, which would
contradict the new behavior. Archive appends the replacement at the end of the
main spec; requirement order is not behavior, so no follow-up move is needed.
The older gutter and surface requirements say "label tint"; that phrase now
means this full-column fill and remains selection-invariant, so they need no
delta.

**Delete the chip geometry instead of zeroing it.** `kLabelChipWidth`,
`kLabelChipGap`, `labelChipAdvance()`, and `labelChipRect()` have no other
purpose; keeping an advance that returns zero would leave dead configuration.
Removing the advance from `imageThumbBox` and `paint` shifts labeled content left
by 6 px, aligning it with unlabeled rows, and `thumbRect`/`nameRect` follow
because they derive from `imageThumbBox`.

**Flip the glyph tint by label luminance.** `paintAsset` gains an optional tint
(default invalid, preserving every other caller). A labeled row passes a dark
tint when `qGray(label.rgb()) >= 85` and a light tint otherwise. Qt's `qGray` is
`(r*11 + g*16 + b*5) / 32`; the sheet yields red 87, orange 164, yellow 215,
green 102, blue 90, violet 84, and gray 153. That picks dark for every color
except violet, which gets the light glyph. With the existing 80% opacity the
worst case is violet light at ~3.98:1 (dark would be ~2.57:1); every other
color is ≥3.2:1. Alternatives: keeping the theme glyph leaves yellow at ~1.2:1
and green at ~1.4:1; always using a dark glyph leaves violet at ~2.57:1. The
threshold is sheet-specific, so a `ponytail:` comment in the delegate names that
ceiling.

**Sample the full column through the existing test seam.**
`rowGutterColorForTest(path, int x = 1)` samples `rect.left() + x` at
`rect.top() + 1`, above the glyph. The default `x = 1` turns the existing
self-test token `lpr_label_tint` into a left-edge assertion; the Qt test adds
`x = kEyeColumn - 2` to prove the span and selects the labeled row to prove the
fill survives selection.

## Risks / Trade-offs

- **Pixel-sampling brittleness** → sample above the glyph and inside the fill;
  an opaque fill makes the sampled RGB exact.
- **Luminance threshold is tuned to the CS6 sheet** → the `ponytail:` comment
  states the ceiling; a manual pass checks all seven labels.
- **The label fill can obscure the row rule** → paint the fill before the rule.
- **Violet sits just below the threshold** → `qGray` 84 against the 85 cutoff;
  blue is 90, so the manual pass checks violet and yellow first.
