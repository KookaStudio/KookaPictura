# Design

Decisions agreed with the owner on 2026-11-13:

- Ship all three items of #1299.
- Fold #1273 in fully: the PR closes both issues.
- In a session with one memory-refused document, recover the others and remove
  their files. Keep only the refused document's files, and keep the directory.

## D1. Recovery: three outcomes per document

`recoverOrphans` sorts each document of an orphaned session into one of three
outcomes:

- **recovered**: it opens and gets a new snapshot. Its files are removed from
  the orphaned session.
- **refused**: `recovery_open` fails and `open_refused_for_memory()` holds. Its
  snapshot and manifest stay where they are. The session is added to
  `retained`, so it is not discarded.
- **unreadable**: any other failure, quarantined as before. A failed
  quarantine move adds the session to `keep`, as now. `keep` means nothing in
  the session is touched, the recovered copies included.

`retained` only prevents the directory delete; the recovered documents' files
still go. The next `orphans()` scan reads only manifests whose snapshot exists,
so it lists just the refused document. Discard still deletes the session, and
Later keeps it.

After the loop, when anything failed, one `QMessageBox` (object name
`recoveryFailures`, warning icon) lists each failed document on its own line:

- refused: `<name>: not enough memory to open it. Its snapshot is kept at
  <path> and will be offered again at the next launch.`
- unreadable: `<name>: the snapshot could not be read. It was moved to
  <quarantine dir>.`

The box opens with `open()`, window-modal and without a nested event loop, and
has `WA_DeleteOnClose`. That way `offerRecovery`'s own modal box returns before
it, and a test that calls `recoverOrphans` directly is not blocked. The
status-bar message is dropped, since the dialog replaces it.

## D2. Row decoders

`read/stream.rs` holds a `RowSource` per encoding. Each one yields the next
scanline of `stride` bytes into a caller buffer:

- **Raw**: a slice of the payload. The length is checked up front, giving the
  same "raw channel too short" error.
- **RLE**: the byte-count table is read up front, as now, and each row is a
  `decode_packbits` of its slice, so the errors are unchanged.
- **ZIP**: a streaming `ZlibDecoder`, read with `read_exact` a row at a time.
  After the last row, one more read must return 0 bytes. A non-EOF error at any
  point restarts the channel with `DeflateDecoder`, which is the dense
  `inflate`'s fallback. A clean end before `expected` bytes gives "ZIP payload
  too short", and a further byte gives "ZIP payload too long", the dense
  errors. **Prediction** is undone per row with the dense per-row arithmetic,
  including the 32-bit unshuffle, which is already per row.

`TiledSink<T>` wraps `Plane::filled` plus a strip buffer of `TILE` rows. Each
full strip is flushed with `write_rect_from` + `settle`. `finish` flushes the
partial strip and returns the plane. An 8-bit channel uses one sink. A 16- or
32-bit channel uses a native sink (`u16`/`f32`, converted per row from big
endian) and an 8-bit sink fed by the existing narrowing formula applied per
row. A depth-1 channel stays dense: it is bit-expanded through the RGB helper
and tiled afterwards, as now.
`ponytail:` depth 1 and the merged image section stay dense. For layered
8-bit RGB the merged colour planes are already skipped, but its extra channels
(merged transparency, saved selections) still decode as one dense plane each.

Phase B keeps `par_iter` over channel tasks. Each task holds one strip
(width × 256 samples per sink) instead of a whole channel. Phase C is
unchanged except that it receives planes: a `-2` without a mask record is
flattened to keep today's flat `data.into()`. The first error in record order
still wins.

The dense decoders stay in use for patterns and composites. The tests compare
the streamed plane with `decode_channel_data` / `decode_depth_channel`.

## D3. Tile budget

`ImageBudget::max_tiled_bytes` defaults to `max_document_bytes`.
`DecodeBudget` keeps two counters. `charge` (contiguous) stays as it is.
`charge_tiled` applies the same overflow and per-allocation checks against
`tiled_remaining`. The error text is the same "… exceeds the decode budget", so
the app's memory detection is unchanged. A streamed channel charges
`width × height × (1 + native bytes)` against tiles, and one strip per sink
against contiguous memory (`width × 256 × bytes`). Charges still happen in
Phase A, before any decode. Depth 1 keeps `charge_plane`. The strips of the
largest channels, one per worker, are each charged as their own allocation.

The composite's planes are charged once the layer section has been read, so
the charge matches what is allocated. A skipped merged image charges only its
extra channels. A layered file with no merged image gets a tiled blank
stand-in (`Plane::filled`), which holds no allocation and is not charged.
Every charge still precedes its allocation.

The existing literal constructors (tests and `webp.rs`) set
`max_tiled_bytes` to their `max_document_bytes`, so every recorded budget
scenario keeps its result.

## D4. Mapped read

`pictura_codec::MappedFile::open(path)` uses `mmap(PROT_READ, MAP_PRIVATE)`
through `libc`, with `madvise(MADV_SEQUENTIAL)`, and derefs to `&[u8]`. An
empty file maps nothing. `Drop` calls `munmap`. The resident pages are page
cache, which the kernel can reclaim, so they do not count against the
governor's anonymous room.
`ponytail:` truncating the file while it decodes raises `SIGBUS`. Saves land
by atomic rename (`safe_io`), which replaces the file without truncating it,
so Pictura's own writers never do this.

## D5. Open budget

`open_budget()` returns `max_document_bytes = min(default, ⅔ room)` and
`max_tiled_bytes = min(default tiled, ⅔ room + scratch available)`. The flag is
true when either one is bounded. `bounded_budget` takes the scratch bytes.
With no governor reading, the budget keeps the defaults.

## D6. Measurement

M12 runs at 12000² unconstrained (`profile-tools.py --peak`) and at 17000²
under `MemoryMax=2G`. The results are recorded in `docs/dev/STATE.md`. The
`open_psd` heap guard baseline is lowered to the new 4250² peak.
