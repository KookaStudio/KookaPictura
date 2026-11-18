# Design

`export_webp` mirrors `export_png` but selects the engine WebP encoder:

- `lossless: true` or a negative `quality` writes VP8L through
  `pictura_codec::encode_webp`.
- Otherwise it writes lossy VP8 through `encode_webp_lossy` at `quality`
  clamped to 0..=100. The default quality is 90.
- The Rust bridge flattens the composite, checks the destination is not a
  protected source, and writes atomically.
- The control method validates the path like `export_png`, and
  `control_server_gate.cpp` treats it as a file-writing method so restricted
  sessions need the Save grant.

`export_png` is unchanged.
