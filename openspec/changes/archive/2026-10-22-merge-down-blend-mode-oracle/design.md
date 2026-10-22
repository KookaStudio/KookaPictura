# Design

## Context

See `proposal.md` for the motivation. What follows is the current state that
shapes the approach.

`merge_down` (`crates/pictura-render/src/document_ops/layer_ops/merge.rs:177`)
resolves the target and the source, clones both, and hands them to
`bake_inputs`, which builds a scratch document and composites it:

```
merge.rs:195        let inputs = vec![lower.clone(), upper.clone()];
merge.rs:422-424    scratch.layers = LayerStack::from_layers(inputs);
                    crate::composite_rgba(&scratch)
```

The clone carries the source's `blend` and `opacity`, and the compositor
applies both per layer:

```
composite_rows.rs:93    let opacity = layer.opacity as f32 / 255.0;
composite_rows.rs:107   blend_parts_row(..., src_a * opacity * fill * mask * gate,
composite_rows.rs:115                             layer.blend);
```

The scratch composite is the merge's only compositor; there is no second one.
`crates/pictura-render/tests/oracle.rs` already diffs `composite_rgba` against
the committed ImageMagick `-compose` references for the 19 modes ImageMagick
implements with the same formula, in the `solid` and `ramp` scenes, at
tolerance 0 (Vivid Light 1). Because the merge's scratch composite *is*
`composite_rgba` of exactly the two layers, the merge inherits that proof —
but nothing in the test suite exercises it through the merge, and
`tests/merge_oracle.rs` covers the `Normal` source-over case only.

The guarantee is not undocumented, only unchecked. The corpus states that
"the upper layer is composited over the lower layer using the upper layer's
blend mode / opacity" (`docs/05-layers/merge-and-flatten.md:129-132`), and
the spec states that the merge composites the inputs "through the document
compositor" (`openspec/specs/compositing/layer-management/spec.md:662-663`).
Neither says what the merged node inherits without the `*(inferred)*` marker
the corpus uses for unverified behaviour, and neither is checked at the merge
level.

The misleading statement lives in the content-move path, not in the merge. The
comment at `move_content.rs:408-411` — rewritten by this change — is attached
to a merge of the *cut copy*, and the cut copy is a `Normal` layer:

```
move_content.rs:412   merge_scope(doc, MergeScope::Down(&new_path))
                       new_path is the cut copy, not the layer the pixels came from
via.rs:71             copy_source(...) -> transparent_layer(...)
create.rs:29          blend: BlendMode::Normal, opacity: 255
```

The comment this change replaces warned that the merge composites with
`Normal`. That warning is right for the path it is attached to: that merge
really does composite a `Normal` copy over the layer. Its generalisation to
`merge_down` is what is false, and it is what the issue was filed on.

## Goals / Non-Goals

**Goals:**

- The merge path is checked against ImageMagick for every mode ImageMagick
  implements with the same formula, so a dropped blend mode fails a test
  rather than a report.
- The contract is stated in the spec and pinned by a test whose expected
  pixels are hand-computed.
- The misleading comment is replaced with both facts it gets wrong and the
  one it gets right.

**Non-Goals:**

- No engine behaviour change. The merge composites the same pixels it does
  today.
- Deciding what the merged node should inherit when the target's blend mode
  or opacity is not the default. That is a product decision about Photoshop
  parity, recorded under Risks, and it is followed up in #813.
- The other merge scopes. `bake_inputs` is the shared scratch-composite path
  for all of them, so `merge_visible` and `merge_clipping_mask` composite
  their inputs with their own blend modes exactly as `merge_down` does, and
  `merge_selected` resets the result to `Normal`/255. The same guarantee
  therefore holds for the family; this change proves it for Merge Down,
  which is the issue's scope, and says so in the requirement.

## Decisions

**Diff the merge against the committed ImageMagick references; generate no new
fixture.** The references for the 19 supported modes already exist as
`tests/fixtures/<scene>_<Mode>.rgba`, and `scripts/im_compose.py check`
regenerates and diffs them, so a stale reference is caught independently of
this change. *Alternatives considered:* a hand-computed expected value per mode
(19 × 2 values to maintain, and it would only prove the compositor again, not
the merge); a live-only oracle (slower, and it would not run in an
environment without `magick`); extending the compositing oracle's scene helper
into a shared test module (a wider refactor for one duplication — skipped, the
table mirrors the two existing ones and a drift surfaces as a missing
fixture).

**Cover the 19 modes ImageMagick implements with the same formula, in the two
opaque scenes, plus one live `Multiply`.** The scenes and their references are
already committed, so 38 comparisons cost nothing, and the mode table is the
same one the compositing oracle uses. The eight modes ImageMagick cannot
reproduce — Dissolve, Darker/Lighter Color, Soft Light, and the four
non-separable HSL modes — stay covered by the hand-computed W3C unit tests
`compositing/blend-modes` already requires, and are not part of the merge
oracle. A partial-alpha source is excluded for the same reason the compositing
oracle excludes it: ImageMagick's blend operators do not reproduce the W3C
source-over mix there, so the `alpha` scene stays `Normal`-only. The live case
keeps the committed reference honest against a running `magick` and self-skips
when `magick` is absent.

**Pin the merged pixels with hand-computed values, in the integration test
file.** The existing merge unit tests in `merge_tests.rs` are not enough, and
the reason is worth stating precisely: they put their non-`Normal` blend on
the **lower** layer (`merge_tests.rs:151-152` puts `Screen` at 200 opacity on
the target; `merge_tests.rs:275` puts `Multiply` on the bottom selected
layer). A lower layer is the bottom of the scratch composite, and a blend is a
no-op against a transparent backdrop — the blend term is multiplied by
`as * ab` and `ab` is zero there — so those tests have no teeth for the
*source's* blend at all. The `inputs_composite` helper in the same file is not
the problem: it composites the same inputs through the same compositor, so a
blend dropped *at the merge level* still shows up as a difference against it.
Its blind spot is a compositor-level bug, which is the compositing oracle's
job. What was missing was a test whose non-`Normal` blend sits on the
*source*.

The new test therefore uses a `Multiply` **source** over a `Screen` target and
asserts the merged pixels are the hand-computed composite
`Multiply((40, 60, 80), (120, 130, 140)) = (19, 31, 44)`, plus the
pre-existing contract that the merged node keeps the *target's* blend mode
and opacity. The target's blend is `Screen` rather than `Normal` so that
assertion discriminates: a merged node that hardcoded `Normal` would fail
it. The target's blend mode does not change the merged pixels, because
inside the scratch it is the bottom layer over a transparent backdrop and
the blend term is multiplied by `as * ab` with `ab` zero there — the
target's blend reaches the backdrop only later, through the merged node. It lives in `crates/pictura-render/tests/merge_oracle.rs`:
`merge_tests.rs` is at 795 lines against a 800-line target, the integration
file has the test budget and room, and the `layer` helper the test needs is
already there.

**Keep the cut copy `Normal` and correct the comment rather than implement
its suggestion.** The merge composites the source and the cut copy, and the
merged node inherits the *source's* blend mode, so the blend is applied once,
by the result. The cut copy must therefore stay `Normal`: with it `Normal`,
the moved pixels land in the merged content as their own colour, and the
merged node's blend applies them against the backdrop once. With the copy
carrying the source's blend, the blend would be applied twice — once against
the layer's remaining pixels inside the scratch, once against the backdrop at
render — and the moved pixels would be multiplied by whatever they land on:

```
scratch:  lower = original(Multiply) over transparent ->  S
          upper = cut copy(Normal)  over S            ->  moved        ✔
          upper = cut copy(Multiply) over S            ->  S * moved   ✘ the moved
                                                                 pixels are
                                                                 multiplied by
                                                                 the ones they
                                                                 land on
result:   R = moved, inherits the original's Multiply ->  multiply(backdrop, moved) ✔
```

The general result is `multiply(remaining, moved)`, not a self-product; the
self-product is what it degenerates to on a uniform layer. Implementing the
comment's suggestion would apply the blend twice. The `Normal` cut copy is
correct *because* the merged node re-applies the source's blend.

**Do not change what the merged node inherits.** `docs/05-layers/merge-and-flatten.md`
records the inheritance rule as `*(inferred)*` at `:129-132`; its open
question at `:335-338` covers Merge Layers' naming and inheritance and echoes
Merge Down's *name*. `openspec/specs/compositing/layer-management/spec.md`
states the rule as `*(inferred)*` too. Changing it would alter observable
behaviour on the strength of an unverified rule, and the alternatives are
themselves product decisions (see Risks). The new requirement therefore states
only what the merge path *does* with the source's blend, and leaves the
inheritance rule where the corpus already marks it as unverified.

## Risks / Trade-offs

- **The merged result does not always render as the pre-merge canvas.** The
  merged node inherits the target's blend mode and opacity while its content
  already has them baked in, so the target's opacity is applied twice — the
  source's contribution is weighted by it too — and the target's blend mode
  lands on the source's pixels, which never had it. The merge also composites
  the pair against a transparent backdrop, so the source's blend sees the
  target's *raw* colour rather than its *rendered* colour. The repository's own
  fixture shows the first half: a target of `Screen` at 200 opacity
  (`merge_tests.rs:151-152`) is inherited by the merged node, and the merged
  node therefore renders at alpha 200 where the pre-merge canvas was 255 —
  a derived consequence of the inheritance the test asserts, not something the
  test asserts itself. → *Mitigation:* recorded here and not changed; the spec
  delta claims the source's blend is honoured and does not claim the
  appearance is preserved; the product decision is followed up in #813, which
  records the three effects and the two candidate rules.
- **The oracle test could pass for the wrong reason** — a merged node that
  carried the target's pixels unchanged would also equal the target's
  reference in some scenes. → *Mitigation:* the hand-computed test is the
  durable teeth: `(19, 31, 44)` is neither input's colour, so a merge that
  dropped the blend would produce `(120, 130, 140)` and fail. The manual
  check in `tasks.md` 2.4 is a one-time sanity check on the new test, not the
  durable guard.
- **The mode table is a third copy** of the mapping in
  `scripts/im_compose.py` and `tests/oracle.rs`. → *Mitigation:* it mirrors
  both, and `im_compose.py check` regenerates the references. That catches a
  changed reference, but not a mode added to or removed from the mapping:
  `im_compose.py check` treats an extra committed fixture as a note, and
  nothing cross-checks the three tables against each other, so the drift is
  silent in both directions. A shared helper module would close it, and is a
  wider refactor than this change carries.
- **The live oracle spawns `magick` once per case** — two cases now instead of
  one — and is skipped without `magick`. → *Mitigation:* the committed-fixture
  test is the one that always runs; the live case only keeps the reference
  honest.
