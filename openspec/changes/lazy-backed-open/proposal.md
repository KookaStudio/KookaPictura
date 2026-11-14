# Proposal

## Why

Issue #1218, step 10 of #1209. Opening a large PSD decodes every channel of
every layer before anything appears, although most tiles are never looked at
or edited in a session. Since #1273 the decode streams into tiles, but M12
(17000², four dense layers) still takes 9.3 s and peaks at 1585 MiB before the
first paint. A large photo barely compresses with LZ4 (about 1.2–1.5×), so
packing saves little. The opened file already holds every tile; an unedited
tile can be dropped and decoded from it again, with no scratch writes.

A first attempt made the layer tiles lazy and nothing else. It missed the
targets: a layered RGB document keeps no stored composite (#1216), so the
first zoomed-out paint composites every layer tile, which decodes all of them
anyway. The file carries its own merged image, and the first paint can show
that instead.

## What Changes

- **Lazy layer channels.** On open, an 8-bit layer or mask channel stored raw
  or RLE is indexed (payload offset, compression, each RLE row's offset) and
  not decoded. Its tiles are **backed** by the file: they start away and decode
  on first read from the 256-row band they cover. ZIP channels, 16/32-bit
  channels and the other formats decode as before (`ponytail:`).
- **Backed tiles in the evictor.** A clean backed tile drops without a write,
  and the evictor frees backed tiles before history-held and live ones. The
  first write detaches a tile from the file.
- **A pinned source.** The document keeps the file's descriptor open and
  decodes with `pread`, not a memory map. Before each decode the descriptor's
  size and modification time are compared with the open's: a file rewritten in
  place makes its unread tiles unreadable (#1217's typed refusal). A file
  replaced or deleted needs nothing: the descriptor still reads the old data.
- **Save over the source** needs no copy: saves already land through a temp
  file renamed over the target (#844), so the old data stays readable through
  the descriptor until the document closes. This replaces the issue's R4
  wording, by owner decision.
- **The file's merged image is the first picture.** A lazily opened layered RGB
  document whose merged image is usable shows it, decoded on demand, until its
  first edit. Then the display composites the layers, as today. Saves, exports
  and Copy Merged composite the layers throughout, so they are byte-identical to
  an eager open.
- **Transient reads.** The Layers panel's thumbnails read backed tiles without
  keeping them resident.

## Capabilities

- **ADDED** `codec/lazy-psd-open`
- **ADDED** `document/source-backed-tiles`

## Impact

- `pictura-core`: `tiled_plane/body.rs` (the backed slot, transient pins),
  `tiled_plane.rs` and `plane.rs` (`Plane::backed`), `memory/eviction.rs` (the
  backed class).
- `pictura-codec`: a new `lazy.rs` (source pin, channel index, band decode,
  merged preview), `read/decode.rs` and `read.rs` (the lazy plan), the hostile
  input tests.
- `pictura-app`: `open_jobs/imp.rs` (lazy open), `tiles.rs` and `picture.rs`
  (the preview), the thumbnail path.
