# Design

## 1. The backed slot (core)

`tiled_plane/body.rs` gains a third away state beside `Packed` and `Spilled`:

```rust
enum Slot { Resident, Packed(Box<[u8]>), Spilled, Backed }
struct Cell { slot: Slot, extent: Option<Extent>, source: Option<Source<T>> }
```

`Source<T>` is the tile's rectangle in its channel and a decode closure,
`Arc<dyn Fn(TileRect, &mut [T]) -> Result<(), ()> + Send + Sync>`, shared by
every tile of the channel. A closure, not a trait: it has one implementation,
in the codec (rule 3), and core never names a codec type.

- `Body::backed(len, source)` starts **away** in `Slot::Backed`, holding no
  samples. The first pin decodes into a fresh allocation.
- `source` is kept across a restore, as `extent` is: it is "a copy of these
  samples elsewhere, while it matches them". `Body::dirty` (every write) clears
  both, so the first write detaches the tile from the file.
- `pack` on a body whose source is set drops the samples and returns to
  `Slot::Backed`: `Freed { written: 0 }`. No LZ4, no scratch.
- A failed decode takes the existing lost-tile path: default samples, the
  unreadable flag, and #1217's typed refusal of the edit or save that read it.
- **Transient pins.** `memory::transient(|| …)` sets a thread flag. Under it, a
  pin of an away body in `Slot::Backed` decodes into a buffer the `Pin` owns
  and does not install it, so a whole-layer reader (thumbnails) leaves nothing
  resident. `tiles_mut_at` uses `pin_raw`, which always restores.

`Plane::<T>::backed(width, height, decode)` builds a one-channel tiled plane
whose every tile is a backed dense body. Nothing is known about uniformity
until a decode, so no tile is `Uniform`; a backed tile that decodes uniform
still allocates (the price of never reading at open).

## 2. Eviction order (core)

`Evictable` gains `backed() -> bool` (a source is set). `run_pass` adds a
first class before #1217's: cold backed bodies, held by history or not,
since dropping one writes nothing.
Then the existing order runs. A backed body reached in a later class still
drops free. This is #1217 R2's "priority 2".

## 3. Index and band decode (codec, `lazy.rs`)

- `SourcePin { file: File, path, size, mtime }`, opened by the app beside the
  memory map. `check()` reads `fstat` on the descriptor: a size or mtime that
  differs from the open's means the file was rewritten in place, and every
  later decode fails. A rename over the path or an unlink does not touch the
  descriptor's inode, so it needs no action (owner decision, 2026-11-13).
- `ChannelIndex { pin, offset, width, height, kind }`, `kind` being `Raw` or
  `Rle(row_starts: Box<[u64]>)`, `height + 1` cumulative offsets read from the
  count table at open. The index checks at open that the payload holds every
  row (the count table, then the sum of the counts, within the declared
  length) and that the end lies within the file.
- `decode_rows(rows, out)`: one `pread` of the rows' bytes (checked again
  against the current file size), then PackBits per row with the existing
  bounds-checked `decode_packbits`. A raw channel reads only the rows asked.
- **The band cache.** A tile asks for 256 rows and keeps 256 columns, so
  without help a 17000-wide band would be decoded 67 times. Decoded bands go to
  one process-wide LRU (`(channel id, band) → Arc<[u8]>`), capped at 32 MiB:
  a row-major sweep decodes each band once.

## 4. Where the lazy path applies (codec)

`OpenOptions` gains `source: Option<Arc<SourcePin>>`. With one, Phase B of the
layer decode (`decode_channel_tasks`) indexes instead of decoding a task when
the depth is 8, the channel is modeled (colour, -1, -2), its compression is raw
or RLE, and its rectangle is non-empty. Every other task decodes as today. The
tile budget is charged as before: an edit may make every tile resident.

A malformed row (a PackBits overrun inside a row) is found only when that band
is decoded, so a lazy open can succeed where an eager one fails; that tile
then reads as unreadable. The checks at open cover the offsets and lengths.

## 5. The merged preview

`read_psd_lazy(bytes, options, budget) -> (Document, Option<MergedPreview>)`.
In the existing `skip_layered_merged` branch (layered, RGB, 8-bit, raw or RLE)
it indexes the three colour channels of the merged image. The preview is
offered only when the bottom layer is a visible Background, so the picture is
opaque, and when the read converted no profile (`source_icc` is `None`). A
transparent document is left out: its merged image carries a transparency
channel only when the layer count is negative, a sign the reader does not
keep, and without it Photoshop flattens on white, which the canvas does not
show. Those documents composite their layers, as today (`ponytail:`).

`MergedPreview::region(rect) -> PixelBuffer` is planar straight RGBA with alpha
255, decoded through the band cache; it keeps nothing else.

## 6. The app

- `decode_psd_with` opens a `SourcePin` and reads with `read_psd_lazy`; the
  preview goes to `OpenedDocument` and on to `PictureViewRust::file_preview`.
- `display_region` answers from the preview while one is set; `TileJob`
  carries a clone.
- The preview is dropped, and every composite tile damaged, by the first
  change: `refresh_display_rect`, the tile damage paths, history restores,
  and replacing the document.
- `from_tiles` (the picture from the display cache) refuses while a preview is
  set, so `picture`, exports and Copy Merged composite the layers.
- Saving goes through `write_atomically` as before. Saving over the source
  leaves the old inode open behind the descriptor until the document closes.

## 7. Measurement

M12 at 17000² (eager through a temporary switch, same binary): time to the
first fit-to-screen picture 9433 → 3598 ms (38 %; the issue's target was
25 %, #1321), peak 1540 → 711 MiB. Two changes beyond the lazy open got it
there: `MergedPreview::region` writes its buffer in place, and
`Region::premultiplied` runs on every core. A band decode on every core was
tried and dropped: it held a chunk's bands at once (peak 1300 MiB) and saved
little. M24 (open, pan across, fill on one layer, save) under `MemoryMax=2G`:
open, pan and fill pass with the unconstrained run's pixels (peak 1792 MiB);
the save is refused by `save_bytes`, about 11 GB here, eager or lazy (#1320).
