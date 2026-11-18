# Design

## Decisions (owner, 2026-11-17)

| Question | Decision |
|---|---|
| Estimate only, or stream? | Stream the native save and re-estimate it |
| Raster Save As too? | Yes, rework it as well |
| ZIP channels | Stream them too: zlib is fed row by row |
| Encode cache | Keep it, with a byte budget |
| Which raster formats stream? | TIFF, PNG, BMP, TGA, Raw, Cineon. GIF, JPEG, EPS and DCS keep a whole buffer, under `ponytail:`, with an honest estimate |

## The sink

`pictura_codec` gets one internal sink type, `Sink<W: Write + Seek>`. It
counts the position and patches big-endian or little-endian integers at an
earlier offset (seek back, write, seek to the end). The public API:

- `write_psd_to(doc, psb, cache, &mut W)`: the one writer.
- `write_psd`, `write_psb`, `write_psd_cached` and `write_psb_cached` keep
  their signatures and run the same writer over `Cursor<Vec<u8>>`. Every
  existing byte-level test therefore checks the streamed writer too.
- `save_document_*` writes through the same sink, into a `BufWriter<File>`
  over the atomic temp file.

The app gets `safe_io::write_atomically_with(path, sync, |w| ...)`. It shares
the temp-file name, `create_new` and the rename/cleanup with
`write_atomically`. The save worker and recovery snapshots use it. An encode
error or a failed `before_write` guard removes the temp file and leaves the
target untouched, as today.

## The PSD container

Structure, with each placeholder and when it is patched:

```
header | color-mode data | image resources               (all in memory, small)
layer-and-mask length ◄─────────────────────────┐ patched after the section
  layer-info length ◄─────────────┐ patched     │
    count, records (channel lengths ◄ patched per channel)
    channel data … (streamed, one channel at a time)
    pad to 4
  global mask | tagged blocks | document extras (in memory, small)
image data: compression word, RLE count table ◄ patched, rows (streamed)
```

In the 16/32-bit layout the layer info goes inside a tagged block (`Lr16`,
`Lr32`), so its tag length is the placeholder instead. Each record is written
with zero channel lengths. The offset of each length field is recorded (record
start + 18 + i × (2 + 4|8) + 2) and patched once the channel's stream is
written.

## Channel encoding

`encode_scanlines` and `zip_scanlines` become band encoders writing into the
sink:

- **RLE.** Write the count table as zeros (`planes × height × 2|4` bytes,
  34 KB for a 17000-row plane). Then, for each band of `ENCODE_ROWS` (256)
  rows, read the band from the plane with `read_rect_into`, PackBits-encode
  its rows in parallel (rayon, one `Vec` per row), and write the rows in
  order. Each row's count goes into the in-memory table, which is patched at
  the end.
- **Raw.** Write band by band.
- **ZIP.** A `ZlibEncoder` over the sink, fed band by band. For
  ZIP-with-prediction, each band's rows get the forward delta first. The
  delta is per row, so bands give the same input sequence. Deflate output
  does not depend on how the input is chunked; the golden digests check that.

Parallelism moves from "one task per channel" to "one task per row inside a
band". The order of rows and channels in the file does not change, so the
bytes stay identical. The working set is one band per channel being written:
`256 × row bytes` read, plus the encoded rows (at most 1/128 more).

## Encode cache budget

The cache keeps its keys and pruning. Two changes:

- **Tee while streaming.** A cache miss writes the stream to the sink and
  also appends it to a buffer while `buffered + resident ≤ budget`. Once the
  stream would cross the budget, the buffer is dropped and that stream is
  not cached. The composite block works the same way.
- **The budget.** `min(CACHE_CAP = 512 MiB, ceiling / 8)`, and a stream is
  kept only when the governor's room still exceeds it. With no ceiling, the
  budget is `CACHE_CAP`. Before each encode the cache prunes to the budget,
  oldest key first, so a lower Memory Usage applies on the next save.

The cached bytes stay identical to a fresh encode, as `pec_hit_bytes`
requires.

## The merged image

`save_view` used to call `composite_tiled` (4 channels, tiled) and then copy
R, G and B into one flat buffer. Instead:

- `pictura_render::composite_tiled_channels(doc, n)` composites in bands into
  a tiled plane of the first `n` channels, so the RGB merged image never
  exists as an alpha-carrying or a flat copy.
- A stored composite that needs its alpha dropped is copied band by band into
  a tiled 3-channel plane.

Uniform tiles hold no allocation, and dense tiles can be packed or spilled
(#1217) while the encoder reads them.

## Raster saves

`Raster` splits in two:

- **`RasterRows`** (streaming). It holds the composite as a tiled planar
  `PixelBuffer` (the app builds it band by band), the mode, the retained
  Indexed indices (a flat source store, sliced), and a lazily computed `alpha`
  flag from a tile scan of the alpha plane. `band(y0, rows)` returns
  interleaved RGBA, RGB, gray or index rows for one band.
- **`Raster`** (whole image) stays for the `ponytail:` formats. It is built
  from `RasterRows` by reading every band.

Writers and how each streams:

| Format | Streams | Notes |
|---|---|---|
| PNG | rows → zlib → one IDAT | The IDAT length is patched and its CRC is computed incrementally. Adam7 interlace reads every pass's rows from the tiled composite (7 passes over the bands) |
| BMP 16/24/32 | rows bottom-up (unless Flip) | Bands are read bottom band first, rows reversed inside the band. The file size is patched |
| BMP ≤ 8 bit | whole image | `ponytail:` quantize needs every pixel |
| TGA | rows | Raw packets of 128 pixels continue across rows and bands; the remainder carries over |
| Raw | interleaved: rows. Planar: each plane at offset `header + c × n` | pwrite-style seeks, one pass |
| Cineon | rows | |
| TIFF | strips | Chunky: one strip. Planar: one strip per channel, one pass each. PackBits and LZW are rewritten as streaming encoders with the same output (PackBits look-ahead is at most 130 bytes; LZW state carries over). ZIP feeds the deflater. The pyramid level is a second pass reading every other row and column. The layer section (tag 37724) is streamed with the PSD layer writer, and its count is patched. The IFD goes at the end, as before. Metadata injection (`embed`) runs on the file: it reads the header and the last IFD back, appends, and patches the header |
| TIFF JPEG | whole image | `ponytail:` Qt's encoder takes one buffer |
| GIF, EPS, DCS | whole image | `ponytail:` |
| JPEG, WebP (Qt path) | whole image | `ponytail:` |

CMYK and Lab raster outputs convert or re-emit whole planes (`ponytail:`,
like the native path).

The output of every streaming writer is byte-identical to the previous
writer. A golden-digest test records the previous writers' digests for a
matrix of documents and options before the change, and checks them after.

## Memory gate

`memory_gate::save_cost(doc) -> CommitCost`:

- **working** = `tile_table_bytes(doc)` (the job's copy) + `SAVE_BAND_BYTES`
  (one compositor band) + `encode_working(doc)`. The encode working set is
  `ENCODE_ROWS × widest row × 2 × min(threads, 8)` plus the count tables.
- **spillable** = the merged image's tiles: `3 × canvas` when the composite
  is not authoritative, else 0.

The native save reserves `working` for the job's lifetime and checks
`spillable` against the room plus the scratch disks' free space (as
`admit_spilling` does). Either refusal is a memory refusal.

`raster_save_cost(doc, kind, options)`:

- **Streaming kinds:** working = tile tables + band + `ENCODE_ROWS × row
  bytes × 2`; spillable = `4 × canvas` (the tiled composite).
- **Whole-image kinds:** working adds `Raster::bytes` (RGBA + RGB + gray +
  CMYK or Lab when the mode needs them) and the encoded file's upper bound
  (`n × bytes per pixel + 1 %`). GIF and BMP ≤ 8 bit add the quantizer's
  working set.

`save_engine`, `save_qt_raster` and `write_gif` all reserve their cost. A
refusal sets the memory refusal, which the bridge reports as "not enough
memory", as the native save does.

The heap-peak harness's `estimates_cover_the_measured_peaks` checks
`save_psd`, plus new `save_png`, `save_tiff` and `save_gif` scenarios, at
4250². `save_psd`'s baseline is lowered to the measured value.

## Risks

- **Deflate chunking.** If flate2 or miniz_oxide output depended on input
  chunking, ZIP files would differ byte for byte from the old encoder. They
  would still be valid, but the golden-digest check would fail. Mitigation: the
  digests catch it. Fall back to feeding the deflater in the same 64 KiB
  pieces the old one-shot write used internally.
- **Slower serial-channel encodes.** RLE rows run in parallel within a band,
  but ZIP is sequential per channel, and before this change channels deflated
  in parallel. Measure the ZIP save time on M12 and record it in STATE.
- **Seeking on the temp file.** `BufWriter::seek` flushes, and patches happen
  once per channel, so the cost is a few syscalls per channel.
