# `pictura-core` layer/channel/mask model (M1-A)

Authoritative type contract for the M1-B PSD codec. All multi-byte integers on
disk are **big-endian**; all bounds are **signed** and may sit outside the
canvas. Nothing here does blend math — M1 only round-trips the mode enum.

## Types

### `BlendMode`
The 27 layer blend modes Photoshop CS6 exposes, plus the group-only `'pass'`
(Pass Through) option. Pass Through is **not** a layer mode: it is excluded from
`LAYER_MODES` and must only be set on a `Layer` with `is_group == true`.

```rust
BlendMode::to_psd_key(&self) -> [u8; 4]
BlendMode::from_psd_key([u8; 4]) -> Option<BlendMode>   // None = unknown
BlendMode::LAYER_MODES: [BlendMode; 27]                 // spec table order, no Pass Through
```

| # | Variant | PSD key | Photoshop name |
|---|---------|---------|----------------|
| 1 | `Normal` | `norm` | Normal |
| 2 | `Dissolve` | `diss` | Dissolve |
| 3 | `Darken` | `dark` | Darken |
| 4 | `Multiply` | `mul ` | Multiply |
| 5 | `ColorBurn` | `idiv` | Color Burn |
| 6 | `LinearBurn` | `lbrn` | Linear Burn |
| 7 | `DarkerColor` | `dkCl` | Darker Color |
| 8 | `Lighten` | `lite` | Lighten |
| 9 | `Screen` | `scrn` | Screen |
| 10 | `ColorDodge` | `div ` | Color Dodge |
| 11 | `LinearDodge` | `lddg` | Linear Dodge (Add) |
| 12 | `LighterColor` | `lgCl` | Lighter Color |
| 13 | `Overlay` | `over` | Overlay |
| 14 | `SoftLight` | `sLit` | Soft Light |
| 15 | `HardLight` | `hLit` | Hard Light |
| 16 | `VividLight` | `vLit` | Vivid Light |
| 17 | `LinearLight` | `lLit` | Linear Light |
| 18 | `PinLight` | `pLit` | Pin Light |
| 19 | `HardMix` | `hMix` | Hard Mix |
| 20 | `Difference` | `diff` | Difference |
| 21 | `Exclusion` | `smud` | Exclusion |
| 22 | `Subtract` | `fsub` | Subtract |
| 23 | `Divide` | `fdiv` | Divide |
| 24 | `Hue` | `hue ` | Hue |
| 25 | `Saturation` | `sat ` | Saturation |
| 26 | `Color` | `colr` | Color |
| 27 | `Luminosity` | `lum ` | Luminosity |
| — | `PassThrough` | `pass` | Pass Through (groups only) |

In PSD, a group's blend key may appear in **two** places: the folder record's
`blend mode key` (as for any layer) and the `'lsct'` section-divider tagged
block (`kind`, then optional `'8BIM'` + 4-byte key). psd-tools/Photoshop put
`pass` in the `'lsct'` block and `norm` in the folder record; the codec reads
both and prefers the `'lsct'` key when present.

Keys are exactly 4 bytes, space-padded where the name is short (`mul `,
`div `, `hue `, `sat `, `lum `). The key is stored in the layer record's
`blend mode signature`/`blend mode key` (4 bytes each).

### `PsdRect`
```rust
pub struct PsdRect { pub top: i32, pub left: i32, pub bottom: i32, pub right: i32 }
PsdRect::width()  -> i32                    // right - left, saturating
PsdRect::height() -> i32                    // bottom - top, saturating
PsdRect::is_empty() -> bool                 // width <= 0 || height <= 0
PsdRect::intersect(other) -> Option<PsdRect> // overlap; None when disjoint
PsdRect::union(other) -> PsdRect            // bounding box; empty is not identity
PsdRect::clip_to(width, height) -> Option<PsdRect> // canvas clamp; None on a miss
PsdRect::grow(pad) -> PsdRect               // saturating; a negative pad shrinks
```
PSD `Rectangle` in the layer record (top/left/bottom/right). Edges are signed;
a layer clipped at the canvas edge still reports its real (possibly negative)
bounds. Do **not** clamp to the canvas; codec computes data length from
`width * height` and offsets by the layer's `left`/`top`.

### `Channel`
```rust
pub struct Channel { pub id: i16, pub data: Vec<u8> }
```
Planar, row-major, one sample per pixel. IDs:

| id | meaning |
|----|---------|
| `0,1,2…` | color channels (gray=0; RGB=0,1,2; CMYK=0..3) |
| `-1` | transparency mask (alpha) |
| `-2` | user layer mask |
| `-3` | real user mask |

`data.len()` must equal `rect.width() * rect.height()` for the layer, or the
mask rect for `-2`. Compression (raw vs PackBits RLE) is a per-channel header
concern owned by M1-B; this type holds the decompressed bytes.

### `LayerMask`
```rust
pub struct LayerMask {
    pub rect: PsdRect,        // mask bounds
    pub default_color: u8,    // 0 or 255
    pub disabled: bool,       // from the mask flags
    pub flags: u8,            // raw mask flags byte, preserved
    pub data: Option<Vec<u8>>,// None until the -2 channel is decoded
}
```
Maps to the `Layer mask / adjustment layer data` block plus its `-2` channel.

### `AdjustmentData` (M4-B)
```rust
pub struct AdjustmentData {
    pub key: [u8; 4],   // PSD adjustment key, e.g. nvrt, brit, levl
    pub data: Vec<u8>,  // payload, verbatim
}
```
The raw additional-layer-info block of an adjustment layer, carried **opaquely**:
`pictura-core` never interprets `data` and does not depend on `pictura-adjust`.
`pictura-render` decodes the subset it understands; unknown keys are a no-op.

### `Layer`
```rust
pub struct Layer {
    pub name: String,
    pub rect: PsdRect,
    pub blend: BlendMode,
    pub opacity: u8,          // 0..=255
    pub clipping: bool,
    pub visible: bool,
    pub mask: Option<LayerMask>,
    pub adjustment: Option<AdjustmentData>, // M4-B: None for pixel/group layers
    pub channels: Vec<Channel>,
    pub children: Vec<Layer>, // groups only
    pub is_group: bool,
}
Layer::is_group() -> bool
```
- Pixel layer: `is_group == false`, `children` empty, `channels` holds color
  (`0..`) and transparency (`-1`), `adjustment` is `None`.
- Group: `is_group == true`, `children` non-empty, `channels` typically empty.
- Adjustment layer: `adjustment` is `Some` and `channels` is empty (PSD stores
  no color pixels for an adjustment); its mask still rides channel `-2`.
- `visible` is `!(flags & 0x02)`; `clipping` comes from the layer record's
  **dedicated clipping byte** (read after opacity), not a flags bit. (psd-tools
  sets the `0x08` flag on ordinary layers, so treating `flags & 0x08` as clipping
  would mislabel every layer.)
- `opacity` is the raw 0..=255 layer opacity.

### `Document`
Unchanged M0 fields (`width`, `height`, `mode`, `depth`, `composite`) plus:
```rust
pub layers: Vec<Layer>   // bottom-first: layers[0] is the bottom layer
```
`Document::new` leaves `layers` empty. **Bottom-first ordering matches the PSD
`'Layr'` on-disk order**; do not reverse it when reading or writing.

## Codec notes (for M1-B)

- `'Layr'` channel image data is planar per layer, bottom-to-top.
- Group layers are marked with `'lsct'`; open/closed is enough for M1.
- Unicode names come from the `'luni'` additional-layer key; fall back to the
  legacy Pascal name when absent.
- Adding `layers` to `Document` is a source-compatible field addition; the
  existing struct literal in `pictura-codec` must gain `layers: vec![]`.
