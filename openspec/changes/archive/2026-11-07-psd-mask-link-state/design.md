# Design

## Context

See `proposal.md` for motivation and the three delta specs for behavior.
The reader in `crates/pictura-codec/src/read.rs` copies the mask flags byte
unchanged into `LayerMask.flags`. `mask_params::write_record` preserves bit 0
while separately reconciling disabled and parameter flags. The error is in
`document_ops/layer_ops/layer_masks.rs`: its getter tests bit 0 as linked, its
setter sets it to link, and new masks start with that bit set. The same
constant seeds masks in vector-mask rasterization and Auto Blend. The Layers
snapshot reads the render predicate, so the reversed state reaches the UI.

The existing `compositing/layer-masks` requirement explicitly specifies this
reversed polarity; the delta explicitly removes it and adds `Raster mask link
flag polarity`, retiring the inverted scenario titles rather than silently
dropping them from a modified block. Adobe calls bit 0 "position relative to
layer", but GIMP's PSD
interoperability correction documents clear=linked, set=unlinked and absolute
mask coordinates:

- https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/
- https://mail.gnome.org/archives/commits-list/2017-September/msg02005.html

Read-only inspection with psd-tools found raw mask flags `0x00` in both user
documents. The user confirmed that `_auxien/20261009_01.psd` opens linked in
Photoshop. Its non-empty Smart Object mask has layer bounds
`(157, 1176, 1400, 2833)` and mask bounds `(344, 1184, 1237, 2747)`; preserving
those different document-space origins is part of acceptance. The empty
Watermark mask also has flags `0x00`. Neither supplied file contains a
bit-0-set mask, so neither independently proves the unlinked state.

The row delegate already draws `layers.link2` or `layers.unlink2`, rotated
90 degrees, through `LayerRowDelegate::linkPixmap`. Raster and vector mask
controls share that helper and retain clickable rectangles in both states.
Existing Qt tests check state and clicks but do not assert glyph opacity.
`tst_layers_panel.cpp` is already 1396 lines; new checks belong in a separate
suite. Graph coverage is unavailable for this worktree; these observations
were verified against current source and existing tests directly.

## Goals / Non-Goals

**Goals:**

- Use one PSD-native flags byte as the source of raster linkage; exercise
  decode, edit, encode, and row presentation together.
- Make the opacity change local to the mask glyph so it cannot dim other row
  content or leak painter state.

**Non-Goals:**

- Add a second linked boolean, rewrite codec flags on load, or translate mask
  coordinates according to the misleading bit label.
- Change vector-mask flag encoding, transform/move semantics, layer-to-layer
  linking, glyph artwork, or the current clickable geometry.
- Infer the intended state of old Pictura-authored files or import the
  supplied artwork into the repository.

## Decisions

### Keep raw flags; correct the predicate, setter, and creation sites

Rename the exported raster-mask constant to `MASK_FLAG_UNLINKED` and update
its re-exports and callers. The predicate uses `(flags & bit) == 0` for a
present mask; a missing layer/mask still reports false. Linking clears the
bit, unlinking sets it, and repeated state writes still return false.
Ordinary mask creation, vector rasterization, and Auto Blend initialize
linked masks with bit 0 clear. `LayerMask::default()` already uses zero flags.

Do not invert flags in the reader or writer: that would lose their native
meaning and make a no-edit save change external bytes. Do not add a boolean:
it would duplicate state and require synchronization in codecs and history.
Other flags, coverage, defaults, density, feather, and bounds stay intact.

### Dim the existing unlinked glyph in the shared rendering helper

Retain the current asset selection and rotation. In the shared `linkPixmap`
path, return the linked pixmap at its existing opacity; for an unlinked mask,
paint the rotated glyph onto a transparent pixmap at opacity 0.5, retaining
its size and device-pixel-ratio metadata. This confines opacity to the glyph
and serves both raster and vector controls without duplicating paint logic.
Keep row event routing and history handling unchanged.

Planning default: the user's "link icon" means both mask controls, since
they have the same visual purpose and rendering path. The separate
layer-to-layer badge and toolbar button are excluded. Preserve the existing
broken-chain artwork rather than replacing it with a dim intact chain.

### Use small independent fixtures and checks at each boundary

Extend `scripts/generate-fixtures.py` with new small linked/unlinked PSD
fixtures authored through psd-tools, with different non-zero layer/mask
origins and a non-uniform coverage plane. Keep all pre-existing fixtures
unchanged. Commit the small fixtures so core read/save regressions do not
require Python at runtime; independent re-read tests may self-skip when
psd-tools is unavailable, following the existing oracle convention.

Add `crates/pictura-codec/tests/mask_link_state.rs`. The codec already has
`pictura-render` as a dev dependency, so tests can assert flags and the actual
render link predicate rather than merely round-tripping the same mistake.
Cover both input states, coherent unrelated flag bits, native coordinate
preservation, link edits, and PSD/PSB output. Have psd-tools independently
decode the written flags, bounds, and coverage for both states.

Extend the existing renderer link-bit test and add checks for the mask
creation paths. Keep the original assertions about preservation and no-ops;
correct their expected bit polarity instead of weakening them.

Add `tst_mask_link_state.cpp` and explicitly register it in
`cpp/tests/CMakeLists.txt`. Reuse the normal window fixture and existing
`rowImageForTest`, glyph geometry, and click seams. Test raster and vector
states at device pixel ratios 1 and 2. Compare the rendered glyph area to
the corresponding asset composited over the known row background at the
specified opacity; do not compare linked/unlinked pixel intensities directly
because their artwork differs. Check geometry and unaffected row pixels too.
Exercise click, undo, and redo through the public UI path and check the
clicked row, history count, role, and rendered appearance. Open the small
codec fixtures through the application for an import-to-row integration check.

Keep the two original files as local reviewer acceptance references:

- `/mnt/work/PhotoshopRequest/2026.10/_auxien/20261009_01.psd`
- `/mnt/work/PhotoshopRequest/2026.10/ilikequeso13/20261014_01.psd`

Generated fixtures verify the explicit encoding, not Photoshop behavior by
themselves. A reviewer check on the user-confirmed linked file complements
the independent decoder evidence without claiming a verified unlinked
Photoshop oracle that has not been supplied.

## Risks / Trade-offs

- Old Pictura files can encode the opposite of their intended state -> follow
  the stored PSD bit without a provenance heuristic; retain this limitation
  in the implementation result. Explicit link edits can correct such files.
- Inverting only the read predicate would make new masks wrong -> update and
  test every identified creation site and the setter in the same change.
- A duplicated linked field could drift during undo/save -> retain raw flags
  and existing document-history storage.
- Painting comparisons can vary at antialiased edges -> use the actual theme
  assets with controlled backgrounds/DPR and allow only byte-rounding
  tolerance, while checking the full glyph region.
- Existing large suites are near their caps -> use the new focused Qt suite;
  keep changes to shared production helpers small.

## Migration Plan

No file migration or dependency installation is needed. Ship the corrected
runtime interpretation, corrected creation defaults, spec deltas, and glyph
opacity together. Preserve imported flag bytes on an unchanged save. A rollback
reverts that interpretation and presentation together; it cannot infer or
repair the intended state of files saved under either convention.

During apply, finish source and test edits before compiling, use at most 80%
of available CPUs, then run targeted Rust/Qt checks and the repository gates.
Archive this change in the implementation PR after validation, keeping manual
acceptance rows in that PR's reviewer checklist.
