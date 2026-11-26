# Proposal: knockout-oracle

## Why

`knockout-composite` implemented the punch-through mechanism but its only
evidence is self-consistent unit tests — the docs' open question
(`docs/05-layers/layers-overview.md:308`) asks for "controlled CS6 test PSDs and
pixel diffs". A controlled PSD and an independent pixel reference are the half
that can be produced here. `psd-tools` ships a compositor that implements
knockout (its `composite.py` documents deep-knockout behavior "Verified against
Photoshop 2026"), so it is a usable independent oracle, exactly as it already is
for codec decode.

## What Changes

- **A committed knockout fixture** `knockout.psd`: 8x8 RGB, a Background
  (red), a middle green layer, and a top blue layer at 50% opacity carrying
  `knko = Deep` — authored by `psd-tools` in `scripts/generate-fixtures.py`
  alongside the other fixtures.
- **A psd-tools-derived pixel reference** under
  `crates/pictura-render/tests/fixtures/`, produced by a new
  `scripts/psd_knockout_reference.py` from `psd.composite()`, in the same raw
  interleaved-RGBA8 format as the ImageMagick oracle.
- **A render oracle test** `crates/pictura-render/tests/oracle/knockout_oracle.rs`
  that reads the fixture through `read_psd`, asserts the typed `Knockout::Deep`,
  composites with `composite_rgba`, and diffs against the reference within a
  documented tolerance (self-skips when `psd_tools` is absent, like the other
  oracles). It also asserts the semantic punch-through: the middle layer's green
  does not contribute.
- **No behavior change.** If the oracle disagrees with the inference beyond
  tolerance, that is reported, not hidden by widening the tolerance.
- **BREAKING**: none.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `knockout-compositing`: add a requirement that the CPU knockout has a
  psd-tools-derived differential check.

## Impact

- `scripts/generate-fixtures.py` + a new `scripts/psd_knockout_reference.py`.
- `crates/pictura-codec/tests/fixtures/knockout.psd` (generated).
- `crates/pictura-render/tests/fixtures/knockout_*.rgba` (generated) +
  `crates/pictura-render/tests/oracle/knockout_oracle.rs`.
- Fixtures README and the roadmap/STATE open-question notes.
- No new dependency (psd-tools and python3 are existing oracle tools).
- Ceiling: the reference is psd-tools, not a Photoshop pixel dump; the manual
  Photoshop reopen stays deferred.
