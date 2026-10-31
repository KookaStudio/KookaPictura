# Design

## Context

See `proposal.md` — Why, and `specs/codec/psd-decode-parallelism/spec.md` for
the contract. The current read path and the seams this change touches:

```
read_layer_section            read.rs:588
  └─ read_layer_info          read.rs:651
       phase A  parse all layer records (read_layer_record)
       phase B  per record, per channel: read_channel_data(&mut r)   serial
       phase C  build_tree / derive_background

read_container                read.rs:105
  └─ image data  read_rle      decode.rs:44    serial scanline loop
                 inflate       decode.rs:19    one stream
                 undo_prediction depth.rs:118  serial byte loop + full-buffer copy
```

Constraints that shape the approach:

- `Reader` is only `{ data: &[u8], pos }` with bounds-checked `take`, so a
  channel can be decoded from its own slice with no change to
  `read_channel_data`.
- The writer already runs the same problem in the other direction
  (`write_layers.rs:515`): flat task list, `par_iter`, ordered `collect`, serial
  reassembly, first error via `?`. `write_cache.rs:265` already tests it against
  a one-thread rayon pool.
- `read.rs` is 1135 lines against the 1200-line code cap; the new decode helper
  has to live in `read/decode.rs` (293 lines).
- The decode budget must be charged before the allocation it guards, and
  `codec/untrusted-input-limits` scenarios must keep their recorded results.

## Goals / Non-Goals

**Goals:**

- Decode layer channels, merged RLE rows, and prediction rows on all cores
  without changing a decoded byte.
- Preserve the first failure in record/row order, so error identity is
  independent of task scheduling.
- Keep the parallel/serial comparison a plain unit test with no
  production-visible switch.
- Stay inside the file-size caps without a new file.

**Non-Goals:**

- Parallel deflate. The merged ZIP stream is one stream; splitting it needs a
  parallel inflate implementation and a new dependency.
- Row-level parallelism inside a single RLE layer channel. Channels already
  supply the parallelism; nesting row tasks under them adds scheduling cost for
  small rows.
- `Samples::narrow_to_u8`. It is one linear pass, and the layer path already
  runs it inside the parallel closure.
- Skipping the open-time recomposite when a merged composite exists
  (`compositing/composite-view`).

## Decisions

### Mirror the writer's ordered parallel pattern

Build a flat `Vec` of channel tasks in phase A, `par_iter` over it in phase B,
`collect` into a `Vec<Result<…>>` (an indexed collect preserves order), then
reassemble in phase C and propagate errors with `?` in order. This is exactly
`write_layers.rs`, so the read and write paths stay recognisably the same shape.

*Alternative rejected:* the `#[cfg(test)] FORCE_SERIAL` atomic the render crate
uses. It needs a process-global switch plus a test lock, and this crate already
has the one-thread-pool oracle.

### Phase A does no bounds check

Phase A accumulates each channel's offset with `saturating_add(len)` and
records a task. A task is one of two variants:

- `Raw { offset, len }` for a depth-8 unmodeled channel. Phase B takes `len`
  bytes from that channel's own slice and stores the full on-disk stream,
  including the 2-byte compression header, as `RawChannel` requires. It does
  not call `read_channel_data`.
- `Decode { offset, len, w, h }` for every other channel. Phase B hands the
  task `Reader::new(data.get(base.saturating_add(offset)..).unwrap_or(&[]))` and
  calls the existing `read_channel_data`. The slice is always taken from the
  whole file, never clipped to `info_end`, so an over-long channel fails with
  `Truncated` from the same `take` that fails today rather than a changed error.

Phase C keeps the header/no-header distinction: `Raw` results go to
`raw_channels` verbatim, while `Decode` results go through the existing assembly
for `-2` masks, `-1` transparency, color, and raw channels.

A truncated file then fails with `Truncated` from the same `take` that fails
today, and an absurd length saturates into a slice lookup that fails the same
way. No new bounds arithmetic, no new error path.

Phase A stays serial for the things the spec keeps serial: `charge_channel` in
record order, and the first-seen `layer_compression` decision (which phase C
makes, in the same order, from the recorded compression word). Only decoded
channels are charged: the depth-8 `Raw` branch keeps its existing no-charge
behaviour. Phase A also returns the total channel length; phase C sets
`r.pos = base + total` before the existing `if r.pos > info_end` check, then
`r.pos = info_end`, so the "layer records exceed layer info" refusal is
preserved.

### Errors are collected, then scanned in order

Phase B returns one `Result` per task. Phase C walks tasks in order and returns
the first `Err`. Parallel work may finish out of order; the reported error never
does. Same for RLE rows: one `Result` per scanline, scanned in row order.

### Merged RLE: prefix sum over the count table, `par_chunks_mut`

The count table is already read up front. Take the payload as the file tail
without advancing the reader, `let payload = &r.data[r.pos..]`, not
`r.take(sum_of_counts)` (which would fail before an earlier row's error); the
image-data section is last. Compute each row's payload offset with a checked
prefix sum (overflow is a typed error), then zip
`out.par_chunks_mut(row_bytes)` with `counts.par_iter()` — both indexed
iterators, so chunk *i* is row *i*, which is the same
`channel * plane + row * row_bytes` the serial loop computes. Each closure
slices `payload.get(off..off + count)` using checked offsets, decodes into its
chunk, and returns a `Result` (`Truncated` when the slice is absent); the
collected vector is scanned in row order. Advance `r.pos` by the consumed total
only when every row succeeded. The truncated count table read stays serial and
first, unchanged.

### Prediction inversion per row

`undo_prediction` is already row-local at every depth: depth 8 and depth 32 are
byte-wise deltas within a row, depth 16 is a `u16` running sum within a row.
Replace the outer `step_by`/row loops with `par_chunks_mut(row)` and the same
inner loop. For depth 32, move the unshuffle into the same per-row closure so it
works on one row's `4 * width` bytes instead of copying the whole buffer with
`data.to_vec()`. Preserve the current per-depth remainder semantics: depth 8
processes a short final chunk; depth 16 skips a short row; depth 32 delta-decodes
a short chunk but skips the unshuffle unless `chunk.len() == 4 * width`. Keep the
`width == 0` early return before `par_chunks_mut`, and preserve the `rows` bound
with `.take(rows)` for depth 16 and 32.

### New code goes in `read/decode.rs`

The parallel channel decode helper (phase A/B) lands in `read/decode.rs`;
`read_layer_info` keeps phase C, which is the existing loop with
`read_channel_data` replaced by "take the next precomputed result". `RawLayer`
and its fields are already visible to the child `read::decode` module, so no
visibility change is needed. This keeps `read.rs` under its cap without adding a
file.

### No thread-count knob

Use the global rayon pool. The document is already decoded on a worker thread,
so the pool is idle and there is no contention to arbitrate. A per-document cap
would be a constant with no caller.

## Risks / Trade-offs

- **Budget refusals now precede decode errors in earlier channels** → documented
  in the proposal and pinned by the `pd_budget` scenario; the refusal variant
  and the existing budget scenarios are unchanged.
- **Malformed files do more work** — phase B decodes every task before phase C
  reports the first error, so a malformed file does more work than the serial
  loop; it is bounded by the decode budget.
- **Peak memory grows transiently for dropped folder/divider channels** — phase
  B holds every decoded plane until phase C drops the ones `build_tree`
  discards. The growth is bounded by the decode budget: a hostile file can
  declare large placeholder channels within budget. The retained channels are
  held anyway today.
- **Little parallelism on a two-layer file** → the merged RLE row path covers
  the case where channel count is low, which is also the flattened-file case.
- **Rewriting the depth-32 unshuffle can change bytes** → the `pd_prediction`
  scenario compares 8/16/32-bit prediction against the serial pool, and the
  existing depth oracles still run.
- **Rayon overhead on tiny documents** → one split check per stage; the profile
  task measures a 4×4 fixture alongside the 24 MP one to confirm the open path
  does not regress.
- **Rayon tasks inside a caller's rayon context** → phase B is itself a
  production rayon context, and `undo_prediction`'s `par_chunks_mut` runs inside
  a channel task at 16/32-bit. Nested rayon is expected and handled by
  work-stealing; the extra split checks add scheduling overhead on small
  planes.

## Open Questions

- Whether a single RLE layer channel should also parallelise its rows. Deferrable:
  it only matters for a document with one huge layer and no merged composite,
  and it can be added behind the same pattern without changing the specs.
