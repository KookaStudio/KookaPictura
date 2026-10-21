# Document Model

- **Spec ID:** `ARCH-008` (provisional; see `INDEX.md`)
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 extends 3D/video handling and adds Color Lookup and Properties-panel context; the layer/channel/path core is unchanged from CS5. **Correction (2026-09): artboards are NOT part of CS6** — they were added in CC 2015 and are out of parity scope (`05-layers/artboards.md`).
- **Depends on:** `ARCH-007` color-management, `ARCH-009` undo-history, `ARCH-011` file-formats, `05-layers/layers-overview.md`, `05-layers/smart-objects.md`, `05-layers/artboards.md`, `05-layers/layer-styles.md`, `10-workflow-io/document-lifecycle.md`

> All module and type names below are **design proposals**. The repository
> contains a working implementation; this file stays the behavioural contract
> for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. PSD/PSB field facts are taken from the fetched
> Adobe File Formats Specification; anything else is marked *(inferred)*.

## CS6 behavior

A Photoshop document is a canvas plus a stack of layers that composite bottom to
top onto that canvas. CS6 layer kinds:

| Kind | Behavior |
|---|---|
| Pixel / raster | Owns a raster channel plus optional alpha/transparency and a layer mask |
| Adjustment | Holds adjustment parameters (and can carry a mask); applies to the composite of the layers below (or to the clipped stack) |
| Fill | Solid Color, Gradient, or Pattern; carries a mask |
| Type | Editable text (point or paragraph), rendered to a raster or kept as vector glyphs |
| Shape | Vector path with fill/stroke; resolution-independent |
| Smart Object | Container holding embedded or linked source content (raster or vector), transformable non-destructively; supports smart filters |
| Group | Folder of layers passed through as a unit; may be a clipping-adjacent pass-through container or isolated |
| 3D (Extended) | Scene-graph layer with meshes, materials, lights |
| Video / frame animation | Timeline-driven content (out of core parity scope) |

Each layer can have a **layer mask** (raster) and/or a **vector mask** (path),
plus **layer style** effects (drop shadow, inner/outer glow, bevel/emboss,
satin, color/gradient/pattern overlay, stroke). CS6 composes all visible layers
plus the active selection, channels, and paths.

**Channels** are per-document: the color channels implied by the mode (RGB,
CMYK, Lab, Gray, Indexed, Bitmap, Multichannel, Duotone), optional alpha
channels saved as spot/selection masks, and spot-color channels. Up to 56
channels total including alpha. Color channels drive compositing; alpha
channels are selemask storage; spot channels are extra inks for output.

**Paths** are vector outlines stored on the document (the Paths panel and the
"working path"), separate from layer vector masks. A clipping path can be named
for EPS export.

**Metadata**: file resolution and physical size, pixel aspect ratio, color
profile, EXIF, IPTC, and XMP.

Layer order in the Layers panel is display order (topmost first); compositing
runs bottom-to-top. Groups composite their children into the group's own
buffer, then the buffer is composited into the parent unless the group's blend
mode is `Pass Through`, in which case children blend directly against the
backdrop (inferred behavior).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Layers panel | Dock | `F7` | Tree: groups, layer thumbnails, visibility, lock, blend mode, opacity, fill, clipping, style badge |
| Channels panel | Dock | n/a | Color/alpha/spot channels; per-channel visibility; "load channel as selection" |
| Paths panel | Dock | n/a | Named paths, working path, fill/stroke, make selection, clipping path name |
| Properties panel | Dock | n/a | Selected layer's transform/physics; live-edits masks, smart objects, adjustments |
| Adjustments panel | Dock | n/a | Creates adjustment layers (not a document model, a creation surface) |
| Layer > New | Menu | `Ctrl+Shift+N` | Layer type creation |
| Layer > Layer Mask / Vector Mask | Menu | n/a | Add/reveal/delete/hide masks |
| Layer > Layer Style | Menu | n/a | Effect dialogs |
| Layer > Smart Objects | Menu | n/a | Convert to/from smart object, edit contents, replace |
| Layer > Group / Ungroup | Menu | `Ctrl+G` / `Ctrl+Shift+G` | Group container handling |
| ~~Layer > New Artboard~~ | Menu | n/a | Not CS6 (CC 2015); no document-level artboard container in the CS6 model. |
| File > File Info | Dialog | `Ctrl+Alt+Shift+I` | EXIF/IPTC/XMP metadata |
| Image > Mode | Menu | n/a | Color mode + bit depth; destructive |
| Image > Image Size / Canvas Size | Dialog | `Ctrl+Alt+I` / `Ctrl+Alt+C` | Document geometry |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Document width/height | int (px) | new dialog | 1–30,000 (PSD), 1–300,000 (PSB) | Filespec limits |
| Resolution | double (ppi) | 72 (screen) | > 0 | Image resource 1005 |
| Color mode | enum | RGB | Bitmap, Grayscale, Indexed, RGB, CMYK, Multichannel, Duotone, Lab | PSD mode field |
| Bit depth | enum | 8 | 1, 8, 16, 32 | 1 only for Bitmap mode; 32 is float |
| Pixel aspect ratio | double | 1.0 | > 0 | Image resource 1064 |
| Channels | int | mode-dependent | 1–56 total | Header channel count |
| Layer name | string | "Layer N" | — | Pascal, padded to 4 bytes in PSD |
| Layer blend mode | enum | Normal | 27 CS6 modes / group `Pass Through` | PSD 4-char key |
| Layer opacity | int percent | 100 | 0–100 mapped to 0–255 in PSD | — |
| Fill opacity | int percent | 100 | 0–100 | Affects layer pixels, not effects |
| Clipping | bool | off | base / non-base | PSD clipping byte |
| Visibility | bool | on | — | PSD flags bit 1 |
| Transparency protection | bool | off | — | PSD flags bit 0 |
| Mask default color | byte | 255 or 0 | 0 / 255 | PSD mask record |
| Mask density | int percent | 100 | 0–100 | Added via mask parameters (PSD flag bit 4) |
| Mask feather | double (px) | 0 | ≥ 0 | 8-byte double |
| Vector mask density/feather | int percent / double | 100 / 0 | as above | Present when vector mask exists |
| Layer ID | int | generated | seed in image resource 1044 | — |
| Layer style: blur | int (px) | mode default | ≥ 0 | Effects layer record |
| Layer style: intensity | int percent | mode default | 0–100 | Effects layer record |
| Layer style: angle | int degrees | 30 (global) | 0–359 | Global angle resource 1037 |
| Layer style: distance | int (px) | mode default | ≥ 0 | Effects layer record |
| Layer style: opacity | int percent | mode default | 0–100 | Effects layer record |

## Algorithms & pipeline

### Compositing order

1. Start from the transparent (or background) canvas for the document.
2. Walk layers bottom-to-top in panel order.
3. For each layer: build its pixel contribution = layer pixels × layer mask ×
   vector mask × fill-opacity; apply layer styles as additional internal passes
   before the layer is blended into the backdrop; then blend using the layer's
   blend mode and opacity.
4. Groups composite children into a group buffer first; if `Pass Through`, the
   children blend directly against the parent backdrop; otherwise the group
   buffer is blended as a unit.
5. Clipping masks constrain the clipped layer to the opaque area of the base
   layer below.
6. Adjustment and fill layers apply to the accumulated composite below them (or
   to their clipping base) at their position in the stack.

Blend modes follow the standard separable/non-separable formulas (Normal
through Hard Mix, then Difference/Exclusion/Subtract/Divide, then
Hue/Saturation/Color/Luminosity). The PSD 4-byte keys are the canonical
enumeration: `norm`, `diss`, `dark`, `mul `, `idiv`, `lbrn`, `dkCl`, `lite`,
`scrn`, `div `, `lddg`, `lgCl`, `over`, `sLit`, `hLit`, `vLit`, `lLit`, `pLit`,
`hMix`, `diff`, `smud`, `fsub`, `fdiv`, `hue `, `sat `, `colr`, `lum `, and
group `pass`. Behavioral parity for blend math is defined against the PDF blend
specifications; the exact Adobe integer rounding is closed.

### PSD/PSB serialization mapping

The PSD container is five sections: header, color-mode data, image resources,
layer-and-mask information, and image data. Layer/mask section shape:

- **Layer info** (`Layr` for 8-bit, `Lr16`, `Lr32`): layer count (negative means
  the first alpha channel holds merged transparency), then per-layer records,
  then channel image data in layer order.
- **Layer record**: content rectangle `(top, left, bottom, right)`; channel count;
  per channel a 2-byte ID (`0`=red…, `-1` transparency mask, `-2` user layer
  mask, `-3` real user mask when both user and vector masks exist) and a 4-byte
  length (8 in PSB); blend signature `8BIM` + 4-char key; opacity; clipping;
  flags (bit 0 transparency protected, bit 1 visible, bit 3 "flags valid", bit 4
  pixel data irrelevant to appearance); filler; extra-data length; then layer
  mask data, blending ranges, layer name (Pascal, padded to 4), and additional
  layer information blocks.
- **Channel image data**: a 2-byte compression code then payload. Codes:
  `0` raw, `1` RLE/PackBits (per-scanline byte counts, 2-byte in PSD and 4-byte
  in PSB), `2` ZIP without prediction, `3` ZIP with prediction.
- **Additional layer information**: a series of tagged blocks (signature `8BIM`
  or `8B64`, 4-char key, length rounded to even). In PSB the following keys use
  8-byte lengths: `LMsk`, `Lr16`, `Lr32`, `Layr`, `Mt16`, `Mt32`, `Mtrn`,
  `Alph`, `FMsk`, `lnk2`, `FEid`, `FXid`, `PxSD`.
- **Effects layer** (`lrFX`): version, effect count, then `8BIM` + effect key
  (`cmnS`, `dsdw`, `isdw`, `oglw`, `iglw`, `bevl`, and `sofi` solid-fill added in
  Photoshop 7.0).
- **Adjustment layer keys** (adjustment parameters are stored as the same data
  as that adjustment's load file): `SoCo`, `GdFl`, `PtFl`, `brit`, `levl`,
  `curv`, `expA`, `vibA`, `hue `/`hue2`, `blnc`, `blwh`, `phfl`, `mixr`,
  `clrL`, `nvrt`, `post`, `thrs`, `grdm`, `selc`.
- **Merged/composite image** lives in the final image-data section. If "Maximize
  Compatibility" is off, the composite may be absent and the image must be
  rebuilt from layers.
- **Byte order** is big-endian on all platforms.

### Internal representation proposal

*(inferred design)* Keep the document as a flat, id-addressed arena of nodes plus
explicit child lists, not a tree of boxed nodes. This makes undo (which
references nodes by id), the Layers model/view, and PSD layer-order round-trips
straightforward, and avoids Rc/RefCell cycles.

```text
Document {
  id, width, height, resolution, pixel_aspect,
  color_mode, bit_depth, color_profile,
  root: NodeId,
  channels: Vec<ChannelId>,          // color + alpha + spot
  paths: Vec<PathId>,                // named paths + working path
  metadata: Metadata,                // EXIF/IPTC/XMP/other
  color_table: Option<[u8; 768]>,    // Indexed
  duotone_spec: Option<Vec<u8>>,     // opaque, preserved
}

Node {
  id, parent, children: Vec<NodeId>,
  kind: NodeKind,
  name, visible, blend, opacity, fill_opacity, clipping, lock_flags,
  mask: Option<MaskRef>,             // raster and/or vector
  styles: Vec<StyleEffect>,
  transform: Affine2D,
  additional: Vec<AdditionalBlock>,  // unknown PSD keys preserved verbatim
}

NodeKind = Pixel{raster} | Adjustment{params} | Fill{source}
         | Text{text_engine_data} | Shape{path, fill, stroke}
         | SmartObject{source_ref, filters, transform}
         | Group{pass_through} | Artboard{rect} | ThreeD{scene_ref}
```

### 8/16/32-bit storage

Pixels are stored at the document bit depth as `u8`, `u16`, or `f32` per
channel, planar per channel/row (matching PSD's per-channel blocks and easing
RLE/ZIP codecs). *(inferred)* Internally a tiled block store (e.g. 128×128) is
preferred so edits, undo, and PSD rectangular layer bounds map cleanly.

## Rust module mapping

Proposals:

- `pictura_core::document` — `Document`, `DocumentSettings`, geometry, color
  mode/depth, profile id, metadata. Owns the node arena.
- `pictura_core::node` — `NodeId`, `NodeKind`, `Node` flags, child ordering
  (`insert_at`, `move`, `remove`), group/artboard semantics.
- `pictura_core::channel` — `ChannelId`, `ChannelKind { Color, Alpha, Spot }`,
  channel buffers and names; merged-transparency handling.
- `pictura_core::mask` — `RasterMask`, `VectorMask` (path + density + feather),
  clipping-mask resolution.
- `pictura_core::path` — `BezierPath`, subpaths, fill rules, fixed-point
  conversion for PSD paths (8.24 fixed point, 26-byte records).
- `pictura_core::style` — `StyleEffect` enum + parameters for the `lrFX` set.
- `pictura_core::smart` — `SmartObjectRef { Embedded(bytes) | Linked(path) }`,
  content document, smart-filter stack.
- `pictura_core::text` — text engine data, glyph runs, paragraph settings
  (parity target: PS text engine, see `03-tools/type-tools.md`).
- `pictura_core::metadata` — EXIF/IPTC/XMP containers and unknown image-resource
  preservation.
- `pictura_core::psd` — parser/serializer for the sections above; raw-preserves
  resource blocks and additional-layer blocks it does not understand.
- `pictura_core::composite` — the compositor (order, blend, masks, styles); CPU
  reference, GPU implementation in `ARCH-006`.

Crossing types: `NodeId(u64)`, `ChannelId(u32)`, `PathId(u32)`, and borrowed tile
slices. The PSD layer adds `PsdScalar` (u8/u16/f32) helpers and fixed-point
path conversion.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `LayersModel` | `QAbstractItemModel` | Tree over `NodeKind::Group`/artboard children; roles for id, name, visibility, lock, blend, opacity, style badge, thumbnail |
| `ChannelsModel` | `QAbstractListModel` | Color/alpha/spot channels; visibility, thumbnail, load-as-selection action |
| `PathsModel` | `QAbstractListModel` | Named paths, working path, clipping path name |
| `PropertiesWidget` | `QWidget` | Selected-node editor (type-specific stacked pages) |
| `MetadataDialog` | `QDialog` | EXIF/IPTC/XMP pages over `pictura_core::metadata` |
| `MaskThumbnailDelegate` | `QStyledItemDelegate` | Renders raster/vector mask thumbnails for the Layers tree |

Model/view and threading constraints are owned by `ARCH-003`; the document model
is immutable to the GUI except through commands (`ARCH-005`).

## Data-model impact

- Every PSD-visible field has a counterpart in `Node`/`Document`; fields the
  engine does not interpret (unknown resource blocks, unknown additional-layer
  keys, duotone spec) are stored as opaque bytes keyed by signature/id so a
  round-trip does not lose them.
- Node removal, reorder, visibility, mask edits, style edits, adjustment
  parameter edits, metadata edits, and mode/depth changes are commands with
  undo records (see `ARCH-009`).
- Undo record shape (proposal): `Command { label, affected: Vec<NodeId>,
  before: Vec<StateDiff>, after: Vec<StateDiff>, pixel_backups: Vec<TileRef> }`
  where `StateDiff` covers scalar/structural fields and `pixel_backups` holds
  pre-edit tiles for destructive pixel ops.
- Smart objects store a nested `Document` (embedded) or a path (linked);
  editing contents opens a separate document window and its history is separate.
- 3D and video/timeline layers are Extended-only; represent them as opaque node
  kinds so files round-trip even if the engine does not render them
  (`05-layers/3d` out of core parity, see `00-overview/feasibility-and-non-goals.md`).

## Edge cases

- **Maximize Compatibility off** — no merged composite; must rebuild from layers.
- **Negative layer count** — first alpha channel is merged transparency.
- **Channel count > 24** — many alpha/spot channels; PSD limit is 56 total.
- **1-bit Bitmap mode** — no color per pixel; restricts masks, adjustment layers,
  and bit-depth conversions (many commands unavailable).
- **Indexed mode** — color table is a 768-byte resource; layer model is limited
  (Photoshop restricts layers in Indexed mode).
- **Duotone** — specification is unpublished; treat as grayscale for compositing
  and preserve the opaque duotone block.
- **Empty / 1-px documents** — zero-area layer rectangles are legal; channels may
  have zero length.
- **PSB sizes** — 300,000 px dimensions and 8-byte length fields everywhere the
  PSD format uses 4 bytes; 8-byte lengths for the documented key set.
- **RLE odd rows** — a pad byte terminates odd-length rows (inferred from the
  spec's row-padding note); RLE is PackBits as used by Macintosh ROM and TIFF.
- **Group pass-through** — `pass` changes whether children blend to the parent
  backdrop or the group buffer.
- **Vector + user mask** — channel ID `-3` ("real user mask") appears when both
  a user mask and a vector mask exist.
- **Unknown additional-layer blocks** — must be preserved verbatim.
- **Layer IDs** — the document ID seed (resource 1044) exists to avoid ID reuse
  after flatten/save/open/add; honor it on save.
- **Large documents / memory** — tile-store and stream; do not load all channel
  data as one allocation.

## Parity acceptance criteria

- Given a PSD with 8-bit RGB pixel, adjustment, fill, type, shape, group, and
  smart-object layers, opening and re-saving without edits preserves every
  unknown resource block and additional-layer block byte-for-byte.
- Given a PSB above 30,000 px, the document opens, displays, and re-saves
  without truncation; length fields use the PSB widths.
- Given a negative layer count, the merged transparency is read from the first
  alpha channel and is not shown as a user alpha channel.
- Given layer order `[A, B, C]` bottom-to-top, reordering to `[A, C, B]` changes
  the rendered composite exactly as compositing in the new order would.
- Given a group with `Pass Through`, its children's blend modes interact with the
  parent backdrop; switching the group to `Normal` first composites children into
  the group buffer, and the two composites differ as specified by the blend
  formulas within the tolerance of `11-cross-cutting/testing-strategy.md`.
- Given a layer with both a user mask and a vector mask, both are read (`-2` and
  vector-mask block), both affect compositing, and the `-3` channel appears only
  when the file contains it.
- Given a layer with density/feather mask parameters (flag bit 4), the mask is
  applied with the stored density and feather.
- Given RLE-compressed layer channels, decoding matches raw decode within
  bit-exact tolerance for 8/16-bit (PackBits is lossless).
- Given ZIP-with-prediction channels, decode matches raw within bit-exact
  tolerance.
- Given a document with alpha and spot channels, channels survive a PSD round
  trip with names (Pascal for old, resource 1045 Unicode names for current).
- Given a bitmap, grayscale, indexed, RGB, CMYK, Lab, multichannel, and duotone
  file, each opens with the correct mode and the composite renders for the modes
  the engine supports.
- Given layer styles from `lrFX`, effects are parsed into typed parameters and
  re-emitted so CS6 opens the file with equivalent effects.

## Sources

Fetched for this document:

- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — the authoritative PSD/PSB specification. Established: five-section file
  structure; header fields (signature, version 1/2, channels 1–56, dimensions
  30,000/300,000, depths 1/8/16/32, modes 0/1/2/3/4/7/8/9); color-mode data
  (indexed 768-byte table, opaque duotone); image resources and IDs; layer
  records and channel IDs (`0…`, `-1`, `-2`, `-3`); blend-mode keys; layer
  flags; layer mask/parameter records (density, feather); channel compression
  codes 0/1/2/3 and PackBits; layer info keys `Layr`/`Lr16`/`Lr32`; PSB 8-byte
  keys (`LMsk`, `Lr16`, `Lr32`, `Layr`, `Mt16`, `Mt32`, `Mtrn`, `Alph`, `FMsk`,
  `lnk2`, `FEid`, `FXid`, `PxSD`); effects layer `lrFX` and effect keys;
  adjustment-layer keys; path resource format (26-byte records, 8.24 fixed
  point); global layer mask info; merged-composite behavior.
- psd-tools 1.19 `psd_tools/psd/layer_and_mask.py` (`MaskFlags`, `MaskData`,
  `MaskParameters`) — the installed PSD oracle. Established: mask-parameter
  field flags and widths, the real-user-mask header preceding the parameters
  (keyed on the `-3` channel, psd-tools #693), and 4-byte block padding.
- `https://docs.rs/image/latest/image/` — only used to confirm the Rust imaging
  ecosystem's buffer types; not a document-model source.

Not parsed in this pass: `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`
(official CS6 Help; fetch exceeded the 5 MB limit). Artboard, smart-object,
3D, and text-engine details are therefore *(inferred)/unverified* here.

## Open questions

- **Full additional-layer-information catalogue.** The fetched spec was
  truncated before the complete list of additional-layer blocks (smart objects,
  linked files, vector masks, pattern data, metadata). Resolve by re-fetching the
  section (`PhotoshopFileFormats.htm#50577409_1049436`) or a mirror.
- **Artboard serialization.** How CS6 stores artboards in PSD (names, rectangles,
  which additional-layer key) is not sourced. Resolve from the CS6 Help PDF or
  CS6-made PSDs.
- **Smart-object serialization.** The exact keys/structures for embedded vs
  linked smart objects, smart filters, and their masks are not sourced here.
- **Type layer data.** The text-engine data format inside `TySh`-style blocks is
  not in the fetched excerpt; parity of editable text requires reverse
  engineering or accepting rasterized-text parity.
- **Blend-mode math.** Exact Adobe rounding/clamping for the 27 modes is closed.
  Define the numeric tolerance against PDF blend formulas.
- **Mask feather/density round-trip.** *Partly resolved.* The byte layout is
  sourced from psd-tools (`MaskData`/`MaskParameters`): mask flags bit 4 marks a
  parameters block whose first byte selects user density (`u8`, bit 0), user
  feather (`f64`, bit 1), vector density (`u8`, bit 2), and vector feather
  (`f64`, bit 3), in that order. The 18-byte real-user-mask header precedes the
  parameters (not after, as the spec text orders it) and is present only when the
  layer carries a `-3` channel; the block pads to 4 bytes, so a parameterless
  record is the 20-byte form. Still open: whether CS6 writes the block for default
  values. *Resolves with:* a CS6 mask PSD byte inspection.
- **Pass-through group semantics vs clipping.** Exact interaction of
  pass-through groups, clipping masks, and adjustment layers needs a controlled
  reference; resolve with test PSDs rendered by CS6.
- **Tile store parameters.** Tile size, prefetch, and scratch-disk spill policy
  are design choices, not sourced; resolve in `ARCH-006` and
  `10-workflow-io/scratch-disks-and-memory.md`.
- **32-bit layer support.** Which layer kinds can exist in a 32-bit document and
  how `Lr32` encodes them is not confirmed beyond the key's existence.
