# Proposal

## Why

Issue #795 reports that `Layer > Merge Down` composites the source layer with
`Normal` instead of the source's own blend mode. The merge path does not do
that: `merge_down` clones both layers into a scratch document and composites
them through the document compositor, which applies each layer's own blend
mode and opacity. What is missing is the check — the ImageMagick oracle
covers `composite_rgba` only, and the existing merge tests put their
non-`Normal` blend on the *lower* layer, whose blend is a no-op against a
transparent backdrop.

The report was filed on a `ponytail` comment in the content-move path. Its
warning is right *for that path* — that merge composites a `Normal` cut copy
— and its generalisation to `merge_down` is wrong, as is its proposed fix,
which would apply the blend twice.

The issue's *symptom* can still occur through the **target's** inherited
blend mode and opacity, but that rule is the corpus's `*(inferred)*` one: a
product decision, followed up in #813.

## What Changes

- **The merge path joins the ImageMagick oracle.** A Merge Down of a source
  layer with a non-`Normal` blend mode is diffed against the committed
  ImageMagick blend-operator references, for the 19 modes ImageMagick
  implements with the same formula and in both the solid and ramp scenes, and
  against live ImageMagick for `Multiply`. No new fixture is generated: the
  references already exist.
- **The contract is pinned by a test with hand-computed pixels.** The source's
  blend mode is baked into the merged pixels rather than carried onto the
  merged node, and the merged node's pixels equal the hand-computed `Multiply`
  composite of the pair.
- **The misleading comment is corrected.** The `ponytail` comment in
  `crates/pictura-render/src/document_ops/layer_ops/move_content.rs` warns
  that the content-move merge composites with `Normal` — true for the cut copy
  it is attached to — and generalises that to `merge_down`, which is false. It
  also proposes to composite with the source's blend instead, which would
  apply the blend twice and multiply the moved pixels by the ones they land
  on. The comment is replaced with both facts.
- **The guarantee is stated.** `compositing/layer-management` gains the
  requirement *Merge Down honours the source layer's blend mode*.

Not changed by this proposal: what the merged node inherits. The target's blend
mode and opacity stay the merged node's, as the existing requirement states
*(inferred)*, so a target with non-default settings still renders the merged
result differently from the pre-merge canvas. The finding, with the numbers, is
recorded in `design.md`, and #813 follows up the product decision.

## Capabilities

### New Capabilities

None. Every behaviour below extends an existing capability.

### Modified Capabilities

- `compositing/layer-management`: a new requirement, *Merge Down honours the
  source layer's blend mode*, stating that the source's blend mode and
  opacity govern the merge composite and are not replaced by `Normal`, that
  the source's blend mode is baked into the merged pixels, and that the merge
  path is diffed against the ImageMagick blend operators for the modes
  ImageMagick implements with the same formula.

## Impact

- `crates/pictura-render/tests/merge_oracle.rs` — the merge joins the
  ImageMagick differential oracle and gains the hand-computed blend test; the
  committed references are reused.
- `crates/pictura-render/src/document_ops/layer_ops/move_content.rs` — the
  comment is corrected. No behaviour change.
- `crates/pictura-render/tests/README.md` — the merge oracle is documented
  beside the compositing oracle.
- `openspec/specs/compositing/layer-management/spec.md` — rewritten by
  `openspec archive` when the change lands.
- No engine behaviour changes, no new dependency, no new fixture, and no
  change under `docs/`.
- Closes #795.
