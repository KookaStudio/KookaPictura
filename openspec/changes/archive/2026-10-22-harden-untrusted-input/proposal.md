# Proposal: Harden the untrusted-document edges

## Why

A source audit (#844) found that a small crafted PSD, PSB, or PDF could abort
the process: layer and mask rects, PSB headers, and compressed streams sized
allocations with no budget. Malformed PDF numbers, an AESV2 key of the wrong
length, and deep nesting could panic or overflow the stack, and a page tree
that points back at itself looped forever. Ghostscript ran with no timeout or
resource limits. Qt could allocate up to 4 GiB before the import budget
checked the size. The control socket accepted any process of the same user.
Saves wrote through a predictable `<path>.tmp`.

## What changes

- One decode budget, derived from `ImageBudget`, for every PSD/PSB plane: no
  single allocation over `max_alloc_bytes`, and a document total of eight
  times that. Group nesting is capped at 64.
- PDF streams inflate within a per-stream limit and a per-file budget.
  Predictor and object-stream numbers are bounded and their arithmetic is
  checked. AESV2 must have a 128-bit key. The lexer and the page and form
  walks have depth caps, and the first-leaf walk keeps a visited set.
- PDF import lists, probes, and thumbnails at most 500 pages and 500 images.
  Renders and crops stay inside the image budget. The PNG reader for
  Ghostscript output is bounded by its header and by the budget.
- Ghostscript is killed after 300 s. On Unix it runs under address-space,
  CPU, and file-size limits. Its text output is capped. A release older than
  9.55 is refused. An input path that contains the list separator is read
  through a private copy.
- Qt's allocation limit is the import budget, and a declared size over budget
  is refused before decoding. Raster imports are read with a size cap. The
  Import PDF dialog's thumbnail unpacking checks every size.
- Control requests must carry a session token. The token is a random value in
  an owner-only file next to the socket.
- Saves write through a temp file with a random name, opened with
  `create_new`. Salts, file IDs, and scratch names use the OS random source.
- Per-finding regression tests and a seeded mutation test over the PSD, PDF,
  and PNG readers.

## Capabilities

- New: `codec/untrusted-input-limits`, `document/atomic-save`.
- Modified (added requirement): `interop/agentic-control`.

## Impact

- `pictura-codec`: `budget.rs` (new), `read.rs` (decoders moved to
  `read/decode.rs`), `pdf/import/*`, `pdf/read.rs`, `pdf/crypt.rs`, `png.rs`,
  `ghostscript.rs`.
- `pictura-core`: `PsdRect` extents saturate.
- `pictura-app`: `safe_io.rs` (new) and its callers; `decode_image.cpp`,
  `import_pdf_dialog.cpp`, and `control_server.*`; the control clients
  (`selftest_control.cpp`, `scripts/verify-control.sh`,
  `scripts/profile-tools.py`).
- Dependencies: `getrandom` and, on Unix, `libc`, both already in
  `Cargo.lock`.
