# M12 — Document operations (resize, canvas, orientation)

Goal: apply the destructive `Image > Image Size / Canvas Size / Image Rotation`
ops to a whole `pictura_core::Document`, wrapping the pure per-buffer functions
landed in M10 (`pictura-ops`). New `pictura-render::document_ops` module. Specs:
`docs/04-image-ops/image-size.md` (`IMG-001`), `canvas-size.md` (`IMG-002`),
`image-rotation-and-flip.md` (`IMG-003`).

This change landed the module split, the public contract, the document-tree
math, and the structural oracle (`crates/pictura-render/tests/document/document_oracle.rs`).

## Scope

In:
- `resize_document` — whole-document resample. Scale every layer's `rect` and
  resample its channel planes (color + alpha) and mask from the old rect size to
  the new; update `doc.width` / `doc.height`.
- `resize_canvas_document` — translate every rect by the anchor offset and
  re-extend the document into the new canvas; the added canvas area is
  transparent (fill 0).
- `rotate_document` — exact 90/180/270 index remaps of every channel plane and
  mask, transforming each layer's rect; 90/270 swap the document dimensions.
- `flip_document` — exact horizontal/vertical index remap of every channel plane
  and mask, transforming each layer's rect.
- Recurses through groups; transforms layer masks and the document-level
  `channels`; then recomputes `doc.composite = composite_rgba(doc)`.

Out (later): arbitrary-angle document rotation, Bicubic Smoother / Sharper,
CMYK/Lab, 16/32-bit, the app UI, undo/history, Smart Objects.

Semantics (8-bit, per-channel, no resampling for orientation ops):
- **resize** scales each layer's `rect` and resamples its channel planes (and
  mask) from the old rect size to the new with the chosen kernel.
- **canvas resize** translates rects by the anchor offset and re-extends the
  document channels with fill 0 — the new canvas is transparent.
- **rotations / flips** are exact index remaps of rects and planes; 90°/270°
  swap the document dimensions.
- Adjustment layers have no color channels (their mask still transforms).
  Empty-rect groups only recurse.

## Contract

```rust
// crates/pictura-render/Cargo.toml
pictura-ops = { path = "../pictura-ops" }

// crates/pictura-render/src/lib.rs
pub mod document_ops;
pub use document_ops::{flip_document, resize_canvas_document, resize_document, rotate_document};

// crates/pictura-render/src/document_ops/mod.rs
mod canvas;
mod orient;
mod resize;
pub use canvas::resize_canvas_document;
pub use orient::{flip_document, rotate_document};
pub use resize::resize_document;

// crates/pictura-render/src/document_ops/resize.rs
pub fn resize_document(doc: &mut pictura_core::Document, width: u32, height: u32,
    resample: pictura_ops::Resample) -> Result<(), pictura_ops::OpsError>;

// crates/pictura-render/src/document_ops/canvas.rs
pub fn resize_canvas_document(doc: &mut pictura_core::Document, width: u32,
    height: u32, anchor: pictura_ops::Anchor)
    -> Result<(), pictura_ops::OpsError>;

// crates/pictura-render/src/document_ops/orient.rs
pub fn rotate_document(doc: &mut pictura_core::Document, quarter_turns: u8)
    -> Result<(), pictura_ops::OpsError>;
pub fn flip_document(doc: &mut pictura_core::Document, horizontal: bool);
```

`flip_document` is an exact remap and cannot fail, so it returns `()` rather
than `Result`. Unknown / undecodable adjustment payloads stay no-ops as in
`composite_rgba`.

## Oracle

- **Structural** — after an op, `write_psd` → `read_psd` preserves document
  dimensions and layer count, and `psd-tools` opens the written PSD with the
  expected size and layers.
- **Composite consistency** — `doc.composite` equals `composite_rgba(doc)` after
  every op.
- **Exactness** — four 90° turns = identity; flip twice = identity; centered
  canvas grow-then-shrink = identity (opposite anchors restore dimensions only,
  since they shift the content).

## Task DAG

| ID | Task | Owns |
|---|---|---|
| M12-A1 | `resize_document` — rect scale + per-channel/mask resample, recomposite | `crates/pictura-render/src/document_ops/resize.rs` |
| M12-A2 | `resize_canvas_document` — anchor offset + document channel extension | `crates/pictura-render/src/document_ops/canvas.rs` |
| M12-A3 | `rotate_document` / `flip_document` — exact remaps of rects and planes | `crates/pictura-render/src/document_ops/orient.rs` |
| M12-B | Structural + exactness oracle, composite-consistency check | `crates/pictura-render/tests/**` |
| M12-C | OpenSpec change + reconcile + verify | `openspec/**`, `docs/dev/**` |

## Exit gate

- `cargo fmt --all`, `cargo build -p pictura-render`,
  `cargo test -p pictura-render`, and
  `cargo clippy -p pictura-render --all-targets -- -D warnings` all green.
- Structural and exactness oracle checks pass (composite equals
  `composite_rgba` after every op).
- `openspec validate --all --strict` green with the new `m12-document-ops`
  change.
