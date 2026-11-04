# Design

## Context

See `proposal.md` for motivation and scope.

Current constraints that shape the approach:

- `pictura-core::Document` owns `composite: PixelBuffer`. `pictura-core` cannot depend on `pictura-render`; the dependency runs the other way.
- `Document.composite` is not only a render cache. `pictura-codec` reads and writes it as the PSD merged composite, and history tracks its planes. Moving the buffer out of `Document` would change codec read/write signatures, history snapshots, and every `document_ops` assignment.
- `pictura-render` already owns the compositors: `composite_rgba`, `composite_rgba_region`, `composite_active`, and `composite_region_active`. The missing piece is the cache write and its shape validity, which live in `pictura-app`.
- The app's edit transaction (`Edit` + `refresh_for`) already decides whether an edit is full or bounded; the cache should consume that decision, not re-derive it. An empty `Edit::Regions` is upgraded to a full rebuild (`crates/pictura-app/src/cxxqt_object/edit.rs:296-300`).
- A banded composite's planes can be shared with history. A region patch must write rows through `Plane::row_mut`, not replace the whole plane.

## Goals / Non-Goals

**Goals:**

- One render-owned type for store, patch, refresh, and shape validity of the maintained composite.
- A render unit test that proves a patched region equals the same slice of a full composite.
- One cache-write seam: `refresh` for callers that need compositing, `store`/`patch` for callers that already hold a rendered buffer, and `render` for a read-only region. The app keeps only display, history, notify, and live-state orchestration.
- No behavior change and no new dependency.

**Non-Goals:**

- Moving `composite` out of `Document`.
- Changing the codec, history plane tracking, or `document_ops`.
- Collapsing `composite_active` / `composite_region_active` into one request (issue #1163).
- Rerouting read-only `composite_rgba` calls that composite a temporary scope document (clone-stamp/eyedropper sampling, filter proxies, test oracles). Those are not cache writers.
- Adding a document revision counter or a per-tile validity ledger.

## Decisions

### D1. Borrowing facade, buffer stays in `Document`

Add `DocumentComposite<'a> { doc: &'a mut Document }` in `pictura-render`. It owns the cache operations; the pixels stay in `doc.composite`.

Alternatives:

- **Move the buffer into a render-owned struct.** This is the purest reading of "owning", but it forces codec read/write, history snapshots, and `document_ops` to carry a second value. It is a separate, larger change.
- **App-held facade with a validity stamp.** A plane-identity stamp only catches pixel writes; opacity, blend mode, visibility, clipping, effects, and reordering change the composite without touching a layer plane. A correct stamp would need a document revision counter. Deferred.

### D2. Render-level scope and result

```rust
pub enum CompositeScope {
    Full,
    Regions(Vec<PsdRect>),
}

pub struct Refreshed {
    pub buffer: PixelBuffer,
    pub rect: PsdRect,
    pub backend: Backend,
}
```

`refresh(scope, gpu) -> Vec<Refreshed>` composites through the active backend, then writes the cache. A region covering the whole document is detected and stored whole, not row-patched (a performance decision covered by the existing #1145 whole-region benchmark, not by the unit test); every other region is patched row-wise. `Refreshed.buffer` is the raw straight-alpha composite (the app performs the sRGB conversion), `Refreshed.rect` is the clamped document rectangle, and `Regions` are applied in order; duplicate or overlapping regions are harmless because they write the same source state. It returns one element for `Full` and one per non-empty clamped region for `Regions`. Returning per-region results (not one union) preserves the app's per-rectangle level-0 patch and damage fold.

`refresh` is the entry point for callers that need compositing. `store` and `patch` are the entry points for callers that already hold a rendered buffer; they never composite again. Together they are the one cache-write seam. `render` is the read-only region path.

### D3. Separate `render` from `refresh`

`refresh` writes the cache; `render` returns the same `Refreshed` values without writing it. The live mask-stroke present path uses `render` because the maintained composite is deliberately kept at its pre-stroke value (`mask_dab` has already written the changed mask into the real document), so the present must not patch it. Every other caller uses `refresh`.

### D4. Move the generic buffer patch too

`patch_buffer_region` is used for the level-0 display frame, filter bases, and display previews, not only `doc.composite`. Move it to `pictura-render` as a public utility and delete the app copy. The app imports the render symbol; no wrapper.

### D5. Validity split

`DocumentComposite::is_shape_valid(doc)` checks only the cache's own shape: RGB mode, four channels, document dimensions. Writers guarantee freshness; this is not a revision check. The app's `picture::is_current` becomes the live-state gates that render cannot know (live stroke, lifted transform, display-only preview, retained native depth) AND `DocumentComposite::is_shape_valid(doc)`. This keeps the render type free of UI/session state while moving the cache's shape checks to the crate that owns the cache.

### D6. Layerless full refresh preserves the embedded composite

`refresh(Full)` on a document with no layers does not composite the empty stack; it stores the embedded composite through the same store rule (`store`), so an RGB 3-channel embedded composite (`crates/pictura-core/src/lib.rs:288-300`) becomes 4-channel exactly as before, and a non-RGB composite keeps its plane count. This preserves `composite_full`'s current behavior. A layerless `Regions` refresh is unreachable (no layer edit) and is a no-op, rather than patching transparent pixels over the embedded image.

### D7. Preserve the shared-plane rule

`patch` keeps the row-wise `Plane::row_mut` write and the `min(4, dst.channels)` plane mapping. This avoids flattening a banded composite plane that history still shares and keeps a grayscale composite at one plane. A test asserts the plane stays shared and a history snapshot's bytes are unchanged.

### D8. Differential oracle in render

Move the app's patch helpers into render and add an inline `#[cfg(test)] mod tests` inside `crates/pictura-render/src/document_composite.rs` (not an integration test under `tests/`): `composite_whole` is `pub(crate)` (`crates/pictura-render/src/composite.rs:35`), so only an inline module can pin it. For deterministic fixtures, seed the cache with a distinct marker (or mutate the layer stack between the seed and the patch), then compute a full composite, patch a clone with a region composite, and assert the region changed to that full slice while the outside keeps the seed. Cover RGB, grayscale, clipping, a region crossing a band boundary, and a document taller than `2 * BAND`; also pin `composite_whole == composite_rgba` on the tall fixture so a shared banded defect cannot make the region check vacuous. Knockout and effects fixtures stay with the existing `pictura-render` region suites. The CPU composite is the reference; existing GPU parity tests (`crates/pictura-render/tests/gpu_parity/region.rs`) cover the GPU.

## Risks / Trade-offs

- **Layerless full refresh expands the embedded composite** → preserve the current behavior: do not composite the empty stack, but store the embedded composite through the same store rule (RGB 3-channel becomes 4-channel; non-RGB keeps its plane count). Test against `store_composite(current_buffer(doc))`.
- **Layerless region refresh corrupts the embedded composite** → guard `Regions` on a layerless document as a no-op (unreachable in practice) and test it.
- **A live mask-stroke present writes the cache** → use `render`, not `refresh`, on that path.
- **A region patch flattens a history-shared plane** → keep the row-wise `row_mut` write and test `Plane::shares`/a history snapshot.
- **A whole-document region row-patches** → detect it and store whole; the existing #1145 whole-region benchmark covers the performance path. The unit test asserts only observable bytes: the whole-document region equals a full composite and the previously seeded composite's bytes are unchanged (no in-place mutation).
- **The app's per-rectangle level-0 patch regresses to a union** → return `Vec<Refreshed>` and fold each rect separately.
- **The oracle shares the defect it checks** → seed the cache with a distinct marker or mutate the layer stack between the seed and the patch; pin the banded full composite against the single-canvas composite on a tall fixture; keep the fixture matrix to RGB, grayscale, clipping, and tall documents, with knockout and effects covered by the existing region suites.
- **Scope creep into read-only `composite_rgba` calls** → only cache writers move; sampling and test oracles keep calling the compositor directly.
- **#1163 overlap** → use the existing `composite_active` / `composite_region_active` pair; #1163 can collapse them later without changing this seam.

## Migration Plan

1. Add `pictura-render/src/document_composite.rs` and export it from `lib.rs`.
2. Copy `store_composite`, `patch_composite_region`, `patch_buffer_region`, and `rgba_frame` into the new module; keep the app copies and their wrappers so the app still compiles.
3. Add the inline render oracle and the edge-case tests; run them before touching the app.
4. Repoint app imports and route the edit transaction and the other cache writers through the new type; move `rgba_frame` consumers to the render symbol.
5. Delete the app copies and the app oracle tests that only duplicate the render cache tests.
6. Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run --workspace`.

Rollback is mechanical: while step 2 only copies, reverting steps 4-5 restores the app seam. After step 5 the old helpers must be restored from version control.

## Open Questions

None. The #1163 overlap is resolved by using the existing backend-aware compositors; a later collapse does not change this contract.
