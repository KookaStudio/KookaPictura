# Proposal: add-calculations

## Why

Issue #610 (from the #486 parity audit). `Image ▸ Calculations…` is an inert
catalogue row (`crates/pictura-script/src/command_catalogue/entries.rs`,
`implemented: false`). `docs/08-selection/channel-based-masking.md` (SEL-013)
specifies the command: blend two individual channels from one or more open
documents into a new document, a new channel, or a selection. Apply Image
(#609, capability `imaging/apply-image`) shipped the sibling command and
deliberately left Calculations out of scope.

## What Changes

- **`pictura-render`**: a `calculations` module that reads two source planes
  through the existing Apply Image readers, applies Invert, the layer blend
  modes and Add/Subtract with Scale and Offset, opacity, and an optional mask,
  and produces one grayscale plane at document size.
- **`pictura-app`**: calculations entry points on the existing Apply Image
  bridge and a modal `CalculationsDialog` with two source pickers, Blending,
  Opacity, an optional Mask group, Result (`New Document` / `New Channel` /
  `Selection`), and a result thumbnail preview. Selection and New Channel each
  commit one `Calculations` history state; New Document opens a new grayscale
  image and leaves the source untouched.
- **`pictura-script`**: `image.calculations` is implemented and classified as a
  modal command; the generated command-id header and catalogue goldens change
  for that one row.
- **Not BREAKING**: no existing command, spec, or file changes behavior.

## Capabilities

### New Capabilities

- `imaging/calculations`: `Image ▸ Calculations…` — the dialog, its two-source
  single-channel blending, the optional mask, and the three result targets.

### Modified Capabilities

None. `imaging/apply-image` behavior is unchanged; the new kernel reuses its
readers and blend arithmetic without altering them.

## Impact

- Crates: `pictura-render`, `pictura-app`, `pictura-script`. No new dependency.
- The catalogue goldens change for the one row, and `command_ids.h` is
  regenerated.
- `docs/08-selection/channel-based-masking.md` records the resolved open
  questions (Result default, Selection combine) and the preview deviation;
  `docs/dev/doc-code-parity-gaps.md` and `ROADMAP.md` record the shipped row.

## Provenance

CS6 Help as cited in SEL-013, plus the #609 precedent. Decisions the docs leave
open are recorded as inferred, not verified parity:

- **Result default**: `New Document` (docs say the default is not stated).
- **Selection result**: replaces the live selection (docs say the combine rule
  is not stated).
- **Preview**: the dialog shows a live result thumbnail; CS6 previews on the
  canvas. The canvas needs a new arbitrary-plane render path this change does
  not add.
- **Mask**: masked-out areas are black ("black hides, white reveals").
- **High-depth output**: arithmetic runs in the unit `f32` domain at every
  depth; every result narrows to 8 bits because channels and selections are
  8-bit planes and the new document is 8-bit grayscale.
