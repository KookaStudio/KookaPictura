# Design

## Context

See `proposal.md` for motivation. The constraints that shape the approach:

- `Selection.data` is a `Plane<u8>` that may be flat (`coverage_selection`, `stored_mask_selection`) or tiled (the live selection, a channel cloned after `to_channel`, and a colour row derived into a tiled plane). The transfer paths tile the acquired coverage on the destination. `to_channel`/`from_channel` are refcount clones (`crates/pictura-select/src/lib.rs:379`, `:388`), and `fit_centered` is a clone at same size (`:474`). The engine can already share coverage; only the bridge defeats it.
- The bridge currently exposes three whole-selection readers (`impl_selection/exports.rs`) and five byte-consuming appliers (`impl_selection.rs:573-730`). Every adapter calls a reader, then an applier.
- `copy_layers_from_document` (`crates/pictura-app/src/cxxqt_object/clipboard/imp.rs:64`) is the established cross-document shape: `view: Pin<&mut PictureView>, source: &PictureView`, and its contract requires distinct documents because aliased `&`/`&mut` is unsound.
- Most duplicate operations are same-document: Duplicate Channel defaults to the active document (`duplicate_channel_dialog.cpp:35`). The Channels panel's New Channel drop (`channels_panel.cpp:718-741`) takes a `QPointer<QObject>` source guard from the mime and branches on `source == view_`, so it may be cross-document without dereferencing a closed source. The Load drop accepts only a live source from its own document. `FileDropRouter` enforces different documents on its drops (`file_drop_router.cpp:208,257`).
- `ui/view-seam-adapters` forbids a new bridge file exposing two or fewer functions, confines generated headers to `cpp/api/`, and requires removing a superseded invokable once no caller remains. `cpp/api/selection_bridge.h` already includes the generated `impl_selection/bridge.cxxqt.h`.
- `crates/pictura-app/src/cxxqt_object/impl_selection.rs` is 1,186 lines against the 1,200 hard cap while the retired appliers are present; deleting them at `573-730` leaves about 1,030 lines, and the new logic still lands in `transfer.rs` to keep the transfer seam separate.
- The heap harness measures peak bytes and materializations with a fixed pool (`crates/pictura-app/src/cxxqt_object/tests_profiles/memory.rs:86`), and `Plane::shares_tile` (`crates/pictura-core/src/plane/access.rs:99`) can prove sharing.

## Goals / Non-Goals

**Goals:**

- One acquisition point and one apply point per destination behaviour, reachable for same-document and cross-document transfers. `save_selection_to` delegates to `transfer_source` and `transfer_save`, so the same-document Save is on the owner too.
- No canvas-sized contiguous buffer for transport; tile sharing on tiled live/alpha sources.
- A guard that distinguishes necessary destination work from transport, not just a materialization count.
- Behavioural parity with the current dialog, drop, and panel tests.

**Non-Goals:**

- Changing the colour-row derivation's values; it now derives into a tiled plane, and that one plane is necessary output. `fit_centered`'s dense scratch is the resampler owner's necessary work.
- Changing fit, interpolation, combine algebra, channel-cap policy, or history labels.
- Removing the low-level coverage readers; they remain legitimate reads for tests and `tst_calculations`.
- A general transfer framework for data other than selection coverage.

## Decisions

### 1. Rust owns the seam; no new bridge file

Add `crates/pictura-app/src/cxxqt_object/impl_selection/transfer.rs` holding:

- `transfer_source(&PictureViewRust, spec, mask) -> Option<Selection>` — the single acquisition point. `spec` reuses the existing vocabulary: `""` is the live selection, `alpha:<n>` an extra channel through `selection_from_target` (sense-aware, never a raw plane clone), `mask:<path>` through `selection_from_mask` (a disabled or data-less mask is full coverage; a destination mask save keeps `stored_mask_selection` as its raw base), and `color:<row>` a colour row with the visible-channel `mask`.
- `transfer_channel`, `transfer_save`, `transfer_load`, and `transfer_document` apply helpers, each taking the owned `Selection` and operating on `PictureViewRust`; the bridge thunks wrap them in `edit` with the existing labels. `save_selection_to` delegates to `transfer_source` and `transfer_save`, so the same-document Save shares the owner.
- The appliers apply Invert before fitting and before combining, enforce the dialogs' same-size rule, and record the `New` label for a new document (`impl_selection.rs:729`).

Place the new bridge declarations in the existing `impl_selection/bridge.rs` (207 lines) and their thunks in `impl_selection/bridge/imp.rs` (287 lines). Its generated header is already included by `cpp/api/selection_bridge.h`, so no new bridge file, no bridge-count change, and no new membrane. `impl_selection.rs` gains `mod transfer;` and loses the retired applier methods at `573-730`; it lands near 1,030 lines, so no allowlist entry. The generated header stays confined because only the existing `impl_selection/bridge.rs` changes and `cpp/api/selection_bridge.h` already includes it; verify the workspace and gates with `bash scripts/verify-fast.sh`. `scripts/check-shell-seams.py` is the frame/panel/menu seam check, not a generated-header gate.

Alternatives rejected: a new bridge file (two-or-fewer-function rule and bridge-count cap), and putting the logic in `impl_selection.rs` (file-size cap while the appliers are present).

### 2. Split in-view and ref-pair entry points over shared helpers

- In-view: same document. `mod transfer` reads the coverage from `&PictureViewRust`, drops the borrow, then edits the same view. The Channels panel's New Channel drop branches on the guarded `source == view_`: in-view for a same-document drag, ref-pair otherwise. The row-list Load drop accepts only a live source from its own document.
- Ref-pair: different documents. The bridge takes `view: Pin<&mut PictureView>, source: &PictureView`, matching `copy_layers_from_document`; acquisition completes and the source borrow ends before the destination edit.

Both duplicate thunks check `session.read_only()` and the channel cap before acquiring or applying. The save/load thunks and in-view load paths check `session.read_only()` before acquiring or applying: `apply_selection_labeled` mutates the selection before `edit` refuses at `edit.rs:416`, so the owner must refuse first. A New-document transfer targets a fresh view. The shell keeps open-view checks, and channel mime carries a `QPointer<QObject>` guard so a closed source is rejected before dereference.

### 3. Source acquisition returns an owned tiled `Selection`

`acquire` clones the live selection or a tiled channel plane (refcount), applies the channel's sense or the mask's disabled flag, or derives a colour row. Because the result is owned, it can outlive the source borrow and be passed to any applier. The appliers consume it exactly as the current byte path does:

- New channel: `to_channel` clones the plane, so a tiled source shares every tile with the destination.
- Same-size load with New/Replace: the current selection is replaced by a tiled view, not a dense copy.
- Boolean combine: `zip_tiles` allocates a tiled result; this is necessary output.
- Fit to another size: `fit_centered` builds a dense scratch and tiles it; the scratch belongs to the resampler owner and is excluded from the transport bound. The fitted result is tiled.

### 4. Adapters reduce to one call each

| Adapter site | New intent |
|---|---|
| `frame_selection_dialogs.cpp:100-129` Save cross/same/new | branch before acquiring; same document uses `save_selection_to` (delegating to `transfer_source` and `transfer_save`), other documents and New use the ref-pair owner; `saveDialogThroughFrame` and `saveDialogNewImage` cover it |
| `frame_selection_dialogs.cpp:163-190` Load same-doc colour / cross-doc | in-view colour-row load (with Invert) / ref-pair load; `loadDialogThroughFrame` covers the cross-document and same-document paths |
| `frame_selection_dialogs.cpp:264-301` Duplicate same/cross/New | in-view duplicate / ref-pair duplicate / ref-pair new document |
| `file_drop_router.cpp:217-235` channel drop | ref-pair duplicate |
| `file_drop_router.cpp:265-275` selection drop | ref-pair load with fit |
| `channels_panel.cpp:728-741` New Channel drop | branch on the guarded `source == view_`: in-view duplicate, otherwise ref-pair; `crossDocumentNewButtonDrop` covers the cross-document case |
| `channels_panel.cpp:555-573` Load row drop | accept only a live source from the panel's own document; `closedChannelSourceDropsIgnored` covers a closed source |

Every destination-edit thunk checks read-only before acquiring or applying, and both duplicate thunks also check the channel cap before source acquisition. New-document creation targets a fresh view. The adapters no longer compute source dimensions for the bridge, check `coverage.size()`, or build a `rust::Slice`.

### 5. Retire four transfer-only byte invokables; keep the readers

`selection_coverage_bytes`, `channel_coverage`, and `color_row_coverage` stay as low-level reads (tests and `tst_calculations`). Retire `duplicate_channel_coverage`, `save_selection_coverage`, `load_selection_coverage_fit`, and `new_selection_document`; keep `load_selection_coverage` as the raw-coverage primitive. Rewrite these callers: `tst_channel_options.cpp:417-460` (`duplicateSelectedChannelNormalizesToMasked`), `tst_channels_panel.cpp:933,1092` (`duplicateChannelInvert`, `duplicateChannelAtCapRefused`), `tst_selection_save_load.cpp:298-345` (`saveDialogNewImage`, now frame-driven), and the production call sites in `frame_selection_dialogs.cpp:104,123,277,289`, `file_drop_router.cpp:229,271`, and `channels_panel.cpp:737`.

### 6. Guard necessary destination work, not the materialization counter

Add memory-profile tests at 4250² using `measure`/`alone_in_process`:

- Tiled source (live selection or alpha channel): the destination retains one plane, so assert the peak stays below that one plane plus 1 MiB, and assert every destination tile shares with the source via `shares_tile`.
- Colour row: `to_channel` shares the derived plane into the same-size destination, so the bound is one derived/retained plane plus 12 MiB. One `TILE`-row band is 4250×256×4 bytes (`picture.rs:145`), and a profiled document may add a band-sized `buffer_to_srgb` copy.
- Capped destination: refuse before inversion, fitting, or source acquisition. A 4250² destination at the channel cap transfers a small different-size tiled shape below 1 MiB with no materialization.

Necessary destination work is the retained destination output plus the result of a combine or resample. A mask-target save's full-canvas base and written result, an existing-channel combine's result, and `fit_centered`'s dense scratch are destination work, not transport. The old tiled and colour-row paths peak above their respective bounds.

The heap tests drive the owner core on `PictureViewRust` without a QObject. They do not observe the C++ readers or the shell's `edit`/`present`; `transfer_adapters_do_not_export_dense_coverage` checks all three adapter sources with `include_str!`, and the frame-driven Save/Load tests cover the real path. This is required because `coverage_bytes` reads tiles without incrementing `materializations()`. Two ignored red reproducers keep the exported bytes alive while cloning them into the receiver: `dense_path_exceeds_the_transfer_bound` for a tiled selection and `dense_colour_row_exceeds_the_transfer_bound` for a colour row. They model the removed handoff and are not measurements from a clean origin/master build.

### 7. File-size placement

`transfer.rs` carries all new logic. `impl_selection.rs` gains `mod transfer;` and loses the retired applier methods at `573-730`, landing near 1,030 lines; no allowlist entry is added.

## Risks / Trade-offs

- [Aliased source and target] → The ref-pair entry points are only for distinct documents; drops already refuse `source == target`, and the Channels panel's New Channel drop branches on `source == view_` instead of assuming a self-drop.
- [Dangling source view] → Channel mime retains a `QPointer<QObject>` guard, the panel and channel drop router reject null or foreign sources before dereference, and `FileDropRouter` still checks frame membership. `transfer_source` also returns `None` for a view without a document.
- [Read-only or capped destination] → The owner checks `read_only` before acquiring or applying, and duplicate transfers check the channel cap before source acquisition, inversion, or fitting. A refused transfer records nothing and changes nothing.
- [Colour rows stay dense] → Accepted: the derivation is necessary output. The guard measures one plane plus the band allowance, not zero.
- [Test churn from retiring invokables] → Four invokables retire; behavioural tests keep their assertions and only the setup call changes. `tst_selection_save_load` and `tst_channels_panel` remain the acceptance suites.
- [Allocator noise in the guard] → Fixed rayon pool and one test per process, as the existing memory profile does.
- [Archived design conflict] → `archive/2026-11-16-channel-selection-drag/design.md:62` accepted dense bytes for one-shot drags. This change supersedes that transport decision; the archived behaviour (fit, history, refusals) is unchanged.

## Migration Plan

1. Add the module, acquisition, appliers, and Rust tests (sense, disabled mask, read-only, absent source/deleted target, invert order).
2. Add the bridge declarations and thunks; keep the old byte appliers until the last caller moves.
3. Move the three adapters (panel `source == view_` branch, read-only checks, frame-driven Save/Load tests), then delete the four retired invokables and rewrite their test callers.
4. Commit the old-path measurement/test and prove the bound fails there before the adapter move; then keep the guard on the owner core.
5. Full verification with `bash scripts/verify-fast.sh`; archive the change in the same pull request. Rollback is a revert of the module and call sites; no persisted data changes.

## Open Questions

None blocking. `load_selection_coverage` becomes test-only after the migration; its removal is deferred until no test caller remains.
