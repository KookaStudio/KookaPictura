# Proposal: knockout-groups

## Why

`knockout-composite` applies a knockout only at the document root; a knockout
inside a group is inert. The documented canonical setup
(`docs/05-layers/layer-groups.md:81`) is exactly that case — "place the layers
to punch through in a group; the top layer in the group then punches through the
grouped layers to the next layer below the group" — so a real document ignores
the knockout. `psd-tools`' compositor applies knockouts inside pass-through
groups (probed: a red Background + a pass-through group of green and a half-fill
`knko = Deep` blue composites to `(126, 0, 128)`, the green punched through),
giving an independent reference.

## What Changes

- **Apply knockout inside a pass-through group.** A group with a `PassThrough`
  blend, full opacity, and no mask recurses its children onto the running
  backdrop; a non-bottom child whose knockout is Shallow/Deep now uses the
  document background as its knockout base, exactly as a top-level layer does.
  The document-background canvas is built whenever any layer, at any depth,
  carries a non-`None` knockout.
- **Isolated groups stay inert.** A group with a non-PassThrough blend, reduced
  opacity, or a mask still composites a knockout child as before (a documented
  ceiling).
- **A committed fixture + psd-tools reference** for the pass-through case, and
  an oracle test that diffs it, in the same style as `knockout-oracle`.
- **BREAKING**: none.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `knockout-compositing`: the CPU knockout requirement now covers a knockout
  inside a pass-through group; a new requirement covers the group oracle.

## Impact

- `crates/pictura-render/src/composite.rs`: thread the knockout base through the
  pass-through group branch; make the knockout-presence check recursive.
- `scripts/generate-fixtures.py` + `scripts/psd_knockout_reference.py` +
  `crates/pictura-render/tests/oracle/knockout_oracle.rs`: the group fixture and its
  reference.
- No new dependency; no app UI; no PSD byte-layout change.
- Ceiling: isolated-group knockout, nested-group **shallow** stopping points
  beyond the document background, and clipping bases stay open.
