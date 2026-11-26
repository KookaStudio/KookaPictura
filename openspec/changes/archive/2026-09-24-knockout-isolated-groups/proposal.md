# Proposal: knockout-isolated-groups

## Why

`knockout-groups` applies knockout inside a pass-through group using the
document background as the base; a knockout inside an **isolated** group (a
non-PassThrough blend, reduced opacity, or a mask) is inert. `psd-tools`'
compositor applies it: its `composite.py` documents that deep knockout "escapes
pass-through groups but stops at an isolated one", so an isolated group's
knockout child punches through the group's own earlier layers to the group's
initial (transparent) backdrop, and the group then composites over the layers
below it. A discriminating fixture (red Background, a yellow layer, an isolated
group of green + half-fill blue `knko = Deep`) gives `(126, 127, 128)`; the
current inert behavior gives `(0, 127, 128)`, so the oracle proves the change.

## What Changes

- **Apply knockout inside an isolated group.** An isolated group's child whose
  knockout is Shallow/Deep uses the group's initial backdrop (its empty inner
  canvas) as the knockout base, punching through the group's earlier children,
  not the document background. Pass-through groups keep the document background;
  the innermost enclosing group decides the base.
- **A discriminating fixture + psd-tools reference** and an oracle test.
- **BREAKING**: none.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `knockout-compositing`: knockout now applies inside an isolated group against
  the group's initial backdrop; the new oracle requirement is extended.

## Impact

- `crates/pictura-render/src/composite.rs`: the isolated group branch threads an
  empty initial-backdrop canvas as the children's base.
- `scripts/generate-fixtures.py`, `scripts/psd_knockout_reference.py`,
  `crates/pictura-render/tests/oracle/knockout_oracle.rs`.
- No new dependency, no app UI, no PSD byte-layout change.
- Ceilings unchanged: nested-shallow targets beyond the enclosing group,
  clipping bases, `Transparency Shapes Layers`, non-Background bottom nuance.
