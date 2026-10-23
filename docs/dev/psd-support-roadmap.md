# PSD support roadmap

Working note (`docs/dev/`). The behavioral contract lives in
`docs/01-architecture/file-formats.md` and the capability specs
`openspec/specs/codec/psd-codec/`, `openspec/specs/codec/psd-layer-io/`. This note tracks the
distance between the shipped codec and "fully supports PSD" and the order we
close it in.

## Where we are

`pictura-codec` reads and writes the following (as of the archived changes named
below):

- **Depth.** The editing model is 8-bit. 16/32-bit files read and normalize to
  8-bit, and the source depth is preserved on save for Grayscale/RGB (archived
  `depth-preserve`) and for Lab/CMYK (`depth-color-mode-write-back`): an
  unchanged plane re-emits the native samples, an edited one widens.
  Depth-1 Bitmap reads and, when flat and unchanged, writes back.
- **Color modes.** Gray/RGB/Lab/CMYK/Indexed/Bitmap read and normalize to the
  RGB working space. An 8-bit Lab, CMYK, Indexed, or flat unchanged Bitmap
  document saves back in its source mode (`color-mode-write-back`,
  `cmyk-write-back`, `indexed-write-back`, `bitmap-write-back`), as does a
  16/32-bit Lab/CMYK document (`depth-color-mode-write-back`).
  Multichannel opens for 1 or 3 channels (gray / profile-free CMY); other channel counts are refused. Duotone opens as grayscale with the duotone spec preserved.
- **Compression** 0 (raw) / 1 (RLE) / 2 (ZIP) / 3 (ZIP-with-prediction) on read
  and write; the source kind is preserved on save and all four are depth-aware.
- **Layers.** Pixel layers, groups (`lsct`), raster masks (`-2`),
  `luni`/`lspf`/`lclr`/`iOpa`, and advanced blending (`knko`/`clbl`/`infx`
  plus a typed Blend If view of the blending-ranges body — modeled, not yet
  composited). Unmodeled per-layer tagged blocks, the `-3`
  real-user-mask channel, global layer mask, and mask params are
  preserved opaquely (P2).
- **Rendering.** The whitelisted adjustment keys decode and render (including
  version-3 `phfl`); layer effects (`lfx2` and legacy `lrFX`),
  solid/gradient/pattern fills, `vmsk` vector masks, and `vscg` vector fill
  content rasterize and author.
- **Smart objects / Camera Raw.** Embedded smart objects are modeled,
  authored, rendered, and edited; the CC Camera Raw Filter settings
  (`SoLd.filterFX[].Fltr`) read and write (P2.5). The Pictura Raw core (`pictura-raw-core`)
  adds the filter's 11 PV2012 Basic controls as a typed round-trip
  (`PicturaRawSettings`, `decode_pictura_raw_settings`/`encode_pictura_raw_fltr`), authoring into a
  converted object's `SoLd` (`attach_pictura_raw_filter` + the writer), a CPU render
  pipeline (`render_pictura_raw`), and a bake-into-proxy op (`apply_pictura_raw`). The
  app command `Filter > Pictura Raw…` (`pictura-raw-ui`) now converts a raster
  layer and assigns Pictura Raw as its smart filter in one undo step, with an 11-control
  Basic dialog and dialog prefill from the stored settings. The document-level
  `FXid`/`FEid`/`FMsk` render caches are not authored — the baked proxy carries
  the pixels.
- **Metadata / ICC.** Image resources parse; an embedded non-sRGB profile is
  honored per the incoming-profile policy; EXIF/IPTC/XMP read, IPTC and managed
  XMP edit, templates export/apply; Assign/Convert Profile; the saved resource
  `1039` matches the output color mode.
- **Document extra channels** (saved selections) in the image-data section.

Still open: live text render / Type tool (the `TySh` kind detection, typed model,
and EngineData font/size/colour decode ship — `type-layer-kind`,
`tysh-model-roundtrip`, `type-engine-data`; the deterministic layout and the
glyph-rasterizer seam ship (`text-render-seam`) and a bundled pure-Rust
Liberation Sans backend now materializes a type layer (`text-rasterize-bundled`);
the app now rasterizes a type layer through `Layer > Rasterize > Layer`
(`type-rasterize-command`) and a proxy-less type layer now renders live in the
CPU compositor (`type-live-composite`) and `Rasterize All Layers` covers type
(`rasterize-all-type`) and `Layer > Rasterize > Type` is a real command
(`rasterize-type-command`) and the Qt `QFont` backend renders that command
(`text-qt-backend`); text warp
and a Qt live-composite path do
not), Multichannel
channel counts other than 1 or 3, a true `u16`/`f32` sample model (the
sample-typed store ships, `bit-depth-sample-model`, and the tonal and
color-preserving adjustment families apply natively,
`native-depth-adjustments`/`native-depth-color-adjustments`, and the CPU
compositor applies adjustment layers at native precision, emits a source-depth
composite, and reads high-depth RGB/Grayscale pixel layers natively
(`native-depth-composite`/`native-depth-layer-content`), and `Auto`/`ColorLookup`
apply natively (`native-depth-remaining-adjustments`), and high-depth layers gate
on their native `-2` mask (`native-depth-masks`), and a dirty save re-emits the
native composite (`native-depth-save`), and the Exposure & Gamma 32-bit HDR
tone map (`hdr-conversion`)), sidecars, a manual Photoshop round-trip, and nested-shallow/clipping
knockout targets (`knockout-composite` applies the documented punch-through at
the document root, `knockout-groups` extends it into pass-through groups, and
`knockout-isolated-groups` into isolated groups (stopping at the group's own
backdrop), inferred, all diffed against psd-tools' compositor —
`knockout-oracle`; the model ships).
Blend If now gates the CPU compositor (`blend-if-render`).
Anything else is `PsdError::Unsupported` on read or preserved opaquely where P2
captured it.

The gap table below tracks the distance to "fully supports PSD"; the phases after
it record the order it was closed in.

## Gap list

| # | Gap | Evidence | Impact |
|---|---|---|---|
| G1 | ~~ZIP / ZIP-with-prediction unsupported~~ ZIP (2) and ZIP-with-prediction (3) read (P1) and written (`psd-zip-write`) | `read.rs`, `write.rs` | Closed |
| G2 | Color modes beyond Gray/RGB (Bitmap, Indexed, CMYK, Multichannel, Duotone, Lab) | `read.rs` mode match, `write.rs` mode match | Shipped for Bitmap/Indexed/CMYK/Lab/Multichannel/Duotone (1- or 3-channel Multichannel) with source-mode write-back where the mode maps (`color-mode-write-back`, `cmyk-write-back`, `indexed-write-back`, `bitmap-write-back`, `depth-color-mode-write-back`, `multichannel-duotone-read`); Multichannel channel counts other than 1 or 3 stay Unsupported |
| G3 | Color-mode data (Indexed palette, Duotone spec) dropped | `read.rs` skip, `write.rs` zero | Partly shipped: the Indexed palette is interpreted on read and retained (`Document.source_palette`) so an unchanged Indexed document writes it back; the Duotone spec stays preserve-only and is re-emitted on write-back (`multichannel-duotone-read`) |
| G4 | Bit depth 1/16/32 unsupported (`PixelBuffer` is `Vec<u8>`) | `read.rs` depth check, `write.rs` | Partly shipped: 16/32 read and normalize to 8-bit for editing (`>>8` / `clamp(trunc(f*256))`), and an open→save now preserves the source depth for Grayscale/RGB **and** a CMYK/Lab source (archived `depth-preserve`, `depth-color-mode-write-back`): unchanged planes re-emit exact source-depth samples, edited ones are widened; the sample-typed store ships (`bit-depth-sample-model`), the tonal and color-preserving adjustment families apply natively (`native-depth-adjustments`, `native-depth-color-adjustments`), and the CPU compositor applies adjustments at native precision with a source-depth output (`native-depth-composite`) and reads high-depth RGB/Grayscale pixel layers natively (`native-depth-layer-content`), and `Auto`/`ColorLookup` apply natively (`native-depth-remaining-adjustments`),
and high-depth layers gate on their native `-2` mask (`native-depth-masks`), and
a dirty save re-emits the native composite (`native-depth-save`), and a moved,
transformed, warped, resized, oriented, canvas-resized, or cropped layer now
keeps a re-anchored or resampled native store so its channel is not re-widened
(`native-depth-edit-preserve`; unmodeled spot/extra raw streams, merge/flatten,
and a true working-depth model stay open). The CS6 Exposure & Gamma 32→16/8 HDR
Conversion now ships (`hdr-conversion`, `hdr-conversion-methods`: the retained `f32` composite is tone-mapped by any of the four CS6 methods and quantized to a real 16/8 store); the three closed kernels are inferred approximations, and the Local Adaptation Toning Curve/Corner stays open |
| G5 | Image resources are parsed; an embedded non-sRGB ICC profile is honoured per an incoming-profile policy (Preserve default / Convert / Off); EXIF/IPTC decode, XMP parse + edit with IIM sync, XMP template export/apply with three merge modes, a File Info dialog, IPTC core-field editing, user Assign/Convert Profile commands, and Color Settings ship; sidecars remain open | `read.rs` `read_psd_with` keep, `write.rs` re-emit; `image_resources.rs` parses; `icc.rs` converts/assigns/policy; `metadata.rs`/`exif.rs`/`iptc.rs`/`xmp.rs` decode, edit, template | Wide-gamut files render correctly and are not force-converted; metadata readable/editable/templatable; profiles assignable/convertible/policy-driven |
| G6 | Unknown additional-layer-info keys (effects `lfx2`/`lrFX`, smart objects, text, vector masks, gradient/pattern fills, blend-if, knockout) | `read.rs` `_ => {}`, `write.rs` subset | Shipped: effects, fills, vector masks, adjustment descriptors, and smart objects decode and render (P2.5/P3); text is modeled (`TypeTool` view, `tysh-model-roundtrip`); knockout is typed and round-trips (`knko-blend-if-model`) and now punches through the CPU compositor at the document root, inside pass-through groups, and inside isolated groups (against the group's own backdrop), independently diffed against psd-tools' own compositor (`knockout-composite`, `knockout-groups`, `knockout-isolated-groups`, `knockout-oracle`, inferred mechanism); Blend If/blending-ranges gate the CPU compositor (`blend-if-render`, GPU declines a non-default layer); other unmodeled keys stay opaque (P2) |
| G7 | `-3` real-user-mask channel, mask params, blend ranges, global layer mask | `read.rs`, `write.rs` | Closed (opaque → partly modeled): captured and re-emitted verbatim on open→save (P2); blending-ranges now also parse into a typed `BlendIf` view (`knko-blend-if-model`) and gate the CPU compositor (`blend-if-render`); knockout punch-through is now applied at the document root (`knockout-composite`) and inside a pass-through group (`knockout-groups`), nested-shallow/clipping targets still open |
| G8 | ~~Adjustment descriptor payloads~~ all whitelisted keys decode and render, including version-3 `phfl` (XYZ, `phfl-v3-xyz-decode`) | `composite.rs` doc | Closed |
| G9 | ~~PSB write missing~~ PSB write shipped: version-2 container, dimensions to 300 000; tagged-block big-key width + pad framing fixed | `write.rs` | Closed |
| G10 | ~~Unknown blend key aborts the whole file~~ an unknown blend key degrades to Normal instead of aborting (P1) | `read.rs` `from_psd_key(...).ok_or` | Closed |
| G11 | ~~Absent merged composite ("Maximize Compatibility" off) unhandled~~ an absent merged composite is tolerated on read and writes no image-data section (P1) | `read.rs` reads compression unconditionally | Closed |
| G12 | ~~Write always raw~~ RLE and ZIP/ZIP-prediction write shipped; the document's recorded source compression is preserved on save | `write.rs`, `compression` model | RLE/raw/ZIP/ZIP-prediction composite/layer channels/mask; per-channel mixed kinds normalize per category |
| G13 | Smart objects are preserved opaquely but not modeled: no embedded-source node, so a smart object cannot be resolved, rendered, or re-edited | P2 holds `SoLd`/`SoLE`/`plLd` and `lnkD`/`lnk2`/`lnk3` bytes; nothing consumes them | Closed: an embedded smart object is modeled (source bytes, filename, filetype, config descriptor, linking `uuid`) and rendered (P2.5, `smart-object-source-render`) |
| G14 | No writer for a valid smart-object pair: the `SoLd`/`SoLE` config descriptor, its `lnkD`/`lnk2`/`lnk3` source record, and the matching `uuid` that links them | `write_psd` re-emits preserved bytes but cannot author a new smart object | Closed: `SoLd`/`SoLE` and the matching `lnk*` record are authored (P2.5) |
| G15 | Camera Raw settings are not read or written. Two storage models: `crs:` XMP for a raw opened as a Smart Object, and the `SoLd.filterFX[].Fltr` descriptor for a Camera Raw Filter smart filter | settings are preserved opaquely only; no edit round-trip | Shipped: the CC Camera Raw Filter settings (`filterFX`/`Fltr`, `filterID` 2683) read and write (P2.5); `crs:` XMP is lifted to a typed `CrsSettings` view and edited in place (`crs-xmp-edit`), proven on a synthetic packet (no real ACR fixture) |
| G16 | No **manual** Adobe round-trip check; the automated `psd-tools` oracle ships | interop is proven against `psd-tools`, not Photoshop | Partly closed: a `psd-tools` round-trip oracle ships (P2.5); a manual Photoshop reopen and a CS6/16-bit fixture are deferred |
| G17 | Smart filters are unmodeled: `SoLd.filterFX` (Camera Raw Filter, `filterID` 2683), document `FEid`/`FXid`, and the filter mask `FMsk` | P2 preserves the bytes; nothing parses `Fltr` | Closed: `SoLd.filterFX`, document `FEid`/`FXid`, and the `FMsk` filter mask parse as settings (P2.5) |

## Phases

**P1 — Interop: open any RGB/Gray PSD.** *(shipped)*
Read ZIP (2) and ZIP-with-prediction (3) for composite and layer channels
(G1); degrade an unknown blend key to Normal instead of aborting (G10); tolerate
an absent merged composite (G11). Adds `flate2` (miniz_oxide backend, pure Rust).
No write-format change (the byte-layout golden is unchanged); write compression
moves to P3. psd-tools cannot author ZIP, so the oracle uses it as a *decoder*
of a hand-built ZIP/ZIP-with-prediction file and compares bytes with `read_psd`.

**P2 — Lossless round-trip.** *(shipped)*
Opaque-preserve what the engine doesn't model so open→save is faithful:
image resources and color-mode data (G3/G5), unknown per-layer tagged blocks,
global layer mask, mask params, blend ranges, the `-3` channel (G6/G7). The
model carries the raw bytes and the codec captures and re-emits them; engine
documents keep empty storage so their bytes are unchanged. A psd-tools-authored
fixture round-trips whole-`Document`-equal and psd-tools still opens our output.
Shipped as `2026-09-19-psd-opaque-preservation`; the document-level tagged blocks
(the smart-object source records) and per-layer config descriptors it preserves
are what P2.5 consumes.

**P2.5 — Smart objects, Camera Raw Filter, and Adobe round-trip.** *(shipped)*
Own what P2 preserves. Model an embedded smart object (source bytes, filename,
filetype, config descriptor, and the `uuid` linking layer to source) on top of
the preserved blocks (G13), and author a valid `SoLd`/`SoLE` pair and its
`lnkD`/`lnk2`/`lnk3` record (G14). For Camera Raw, support the CC smart-filter
model: `SoLd.filterFX[].Fltr` with `filterID` 2683, plus the document `FEid` and
`FMsk` blocks, read and written as settings (G15/G17). Smart objects are
CS6→current CC (tolerant read plus byte-preserving write, proven only on the CC
2021 fixture). The Camera Raw settings model targets the earliest CC Camera Raw
Filter (ACR 8 / PV2012); `crs:` XMP is now lifted to a typed view and edited in
place (archived `2026-09-24-crs-xmp-edit`). The round-trip is proven
against the supplied Photoshop fixtures (G16) by the `psd-tools` oracle; a
manual Photoshop reopen and a CS6/earliest-CC fixture are deferred follow-ups.
Shipped as the archived change `2026-09-19-psd-smart-object-roundtrip`.

**P3 — Render preserved data.** *(in progress)*
Decode the remaining adjustment descriptors and real fill descriptors (G8);
gradient fill layers; layer effects (`lfx2` Drop Shadow, Outer Glow, Inner
Shadow, Inner Glow, Stroke, Color/Gradient/Pattern Overlay, Satin); text (the
remaining kind — vector masks are shipped, archived `vector-mask-render`, and
`vscg` vector fill content is shipped, archived `vector-fill-content`).
**Smart-object source rendering is shipped** (archived
`2026-09-19-smart-object-source-render`): an `Embedded` smart object with no
raster proxy is rendered by decoding its payload and sampling it into the layer
rect (stored merged composite preferred, layers fallback; nearest-neighbour;
`Trnf`/warp deferred). **Adjustment payload decoding is partly shipped**: `expA`
(Exposure), `vibA` (Vibrance), and `blwh` (Black & White) decode to
`pictura-adjust` ops (archived `2026-09-19-adjustment-payload-decode`), and
`phfl` (Photo Filter, version 2) now decodes and encodes too (archived
`2026-09-19-photo-filter-adjustment-decode`) with an encoder and an Adjustments
panel `Photo Filter` entry. Gradient Map (`grdm`) now decodes to
`Adjustment::GradientMap` and encodes too (archived
`2026-09-19-gradient-map-adjustment-decode`) with an encoder and an Adjustments
panel `Gradient Map` entry. The real solid-color fill (`SoCo`) descriptor now
decodes, encodes, and rasterizes (archived
`2026-09-19-solid-color-fill-descriptor`); the real gradient fill (`GdFl`)
descriptor now decodes to `Adjustment::GradientFill`, composites generatively for
all five kinds, is fill content for rasterize, and authors through
`encode_gradient_fill` and `Layer > New Fill Layer > Gradient…` (archived
`2026-09-19-gradient-fill-layer`). Color Balance (`blnc`) now decodes to nine
`i16` shifts plus a luminosity byte and encodes too, with an Adjustments panel
entry (archived `2026-09-19-color-balance-adjustment-decode`). Channel Mixer
(`mixr`) now decodes to `Adjustment::ChannelMixer` and encodes too, with an
Adjustments panel entry; the layout is grounded on **ag-psd** because psd-tools
reads only the red row (archived `channel-mixer-adjustment-decode`). Curves
(`curv`) now decodes composite and per-channel curves into `Adjustment::Curves`
and encodes too, grounded on **ag-psd** with a psd-tools partial check (archived
`curves-adjustment-decode`). Pattern fill
(`PtFl`) now decodes to `Adjustment::PatternFill`, taking its pixels from the
document `Patt`/`Pat2`/`Pat3` pattern library (`pictura-codec::decode_patterns`)
and compositing as tiled content, and is fill content for rasterize (archived
`2026-09-19-pattern-fill-layer`). Layer effects (`lfx2`) now decode the
object-based **Drop Shadow** (`DrSh`), **Outer Glow** (`OrGl`), **Inner Shadow**
(`IrSh`), **Inner Glow** (`IrGl`), and a **Stroke** (`FrFX`),
compositing the shadows and glows behind the layer content, the inner shadow and
inner glow above it, and the stroke as a band at the content edge above it, on
the CPU, with the GPU falling back to CPU (archived
`2026-09-19-layer-effects-drop-shadow`,
`2026-09-19-layer-effects-outer-glow`,
`2026-09-19-layer-effects-inner-shadow`,
`2026-09-19-layer-effects-inner-glow`, and
`2026-09-20-layer-effects-stroke`); the object-based **Color Overlay**
(`SoFi`), **Gradient Overlay** (`GrFl`), and **Pattern Overlay** (`patternFill`)
now decode and composite above the layer content, gated by the content coverage
with the effect blend/opacity, the gradient reusing the shared gradient
geometry and the pattern the document pattern library (archived
`2026-09-20-layer-effects-overlays`); the object-based **Satin** (`ChFX`) now
decodes and composites an interior directional band from the blurred content
matte, optionally inverted, gated by the content coverage, and composited above
it (archived `2026-09-20-layer-effects-satin`); **Bevel & Emboss** (`ebbl`) now
decodes and, for the Inner + Smooth (`InrB`/`SfBL`) slice, composites a lit height
field confined to the content above it, other styles/techniques decoding to a
no-op (archived `2026-09-20-layer-effects-bevel`). The legacy `lrFX` block is
now decoded into the same typed effect model and rendered through the shipped
renderers, with a single `lfx2`-over-`lrFX` resolver (archived
`layer-effects-legacy-lrfx`), so the layer-effects family covers both the
object-based `lfx2` and the legacy `lrFX` encodings. The **Stroke** (`FrFX`) now
also fills from a **gradient** (`PntT` `GrFl`, `Grad`) or **pattern** (`PntT`
`Ptrn`, `Ptrn`) source over the same content-edge band, the last deferred `lfx2`
stroke piece (archived `2026-09-20-layer-effects-stroke-fills`). That change also fixed a cross-cutting bug: every `lfx2`
effect blend mode now decodes the `BlnM` descriptor vocabulary instead of the
layer-key one, and the effect goldens were regenerated. Selective Color (`selc`)
now decodes to `Adjustment::SelectiveColor` and encodes too, with an Adjustments
panel entry; the ten-plate layout (reserved plate 0 plus nine named ranges) is
grounded three ways (libpsd, ag-psd, psd-tools framing) and the kernel follows
libpsd's integer CMYK pipeline (archived `selective-color-adjustment-decode`).
**Color Lookup (`clrL`) is now shipped** (archived
`2026-09-22-color-lookup-adjustment-decode`): the block decodes to
`Adjustment::ColorLookup`, an embedded `.CUBE` `3DLUT` is sampled trilinearly,
and `encode_color_lookup`/`identity_cube` author a block; abstract-profile,
device-link, and non-`.CUBE` payloads are no-ops (marked ceiling, no Adobe pixel
parity). That closes the whitelisted adjustment-key set.
**Version-3 `phfl` is now shipped** (change `phfl-v3-xyz-decode`): three
big-endian `u32` CIE XYZ values are read as 16.16 fixed-point relative to D50
and converted with the same profile-free matrix as Lab document read; the
encoder stays version 2, so open→save re-emits v2 with the decoded colour.
Ceiling: the scale and white point are unproven without a CS6 v3 fixture.
**Curves (`curv`) is now shipped**: the original deferral reason — a
single-composite model versus Photoshop's per-channel curves, and an ungrounded
channel-bitmap order — is addressed by the per-channel `CurvesParams` model,
with the per-channel-then-composite order marked an assumption (not
Photoshop-verified). Remaining P3: live text render from EngineData (kind + `TypeTool` model ship, and EngineData font/size/colour decode ships — `type-engine-data`, proven by a real synthetic text-layer fixture against psd-tools; the deterministic layout and POD glyph-rasterizer seam also ship — `text-render-seam`; a bundled pure-Rust Liberation Sans backend materializes a type layer into pixels — `text-rasterize-bundled`, `Layer > Rasterize > Layer` reaches it — `type-rasterize-command`, and a proxy-less type layer renders live in the CPU compositor — `type-live-composite`, `Rasterize All Layers` covers type — `rasterize-all-type`, and `Layer > Rasterize > Type` is a real command — `rasterize-type-command` — which the Qt `QFont` backend renders — `text-qt-backend`; the bundled backend now shapes with `rustybuzz` (pure-Rust HarfBuzz) so kerning/GPOS applies and carries the shaper's glyph offsets (`text-shaping-rustybuzz`, `text-shaping-offsets`) and rasterizes subpixel-accurately with `swash` (`text-subpixel-positioning`, `text-backend-drop-fontdue`); a layer's skew/distort/perspective now ships (`free-transform-quad`, the projective `transform_layer_quad` plus the three `Edit > Transform` modes), but text warp and a Qt live-composite path do not);
RLE and ZIP write shipped (`psd-rle-write`, `psd-zip-write`).
**RLE write is shipped** (archived
`2026-09-19-psd-rle-write`): the merged composite (color + document extra
channels), layer color channels, and the raster mask are PackBits-encoded;
preserved verbatim channels stay byte-for-byte. **ZIP write is shipped**
(archived `2026-09-22-psd-zip-write`): `Document` records the composite and
layer-channel compression observed on read (default RLE) and `write_psd` emits
that kind — RLE, raw, ZIP (zlib), or ZIP-with-prediction (reversible per-row
delta then zlib) — for the composite, extra channels, layer color channels, and
raster mask; a constructed document is byte-unchanged, and an absent merged
composite now writes no image-data section. Ceiling: mixed per-channel kinds
within a category normalize to the first seen.

**P4 — Color modes and depth.** *(partly shipped)*
Indexed/Bitmap/CMYK/Lab now read and normalize to RGB (G2/G3, archived
`color-mode-read`), with a status-bar conversion notice in the app and a
documented lossy-in-mode save for the modes that still write the working mode.
**An 8-bit Lab, CMYK, or unchanged Indexed document now saves back in its source
mode** (archived `2026-09-23-color-mode-write-back`, `2026-09-23-cmyk-write-back`,
and `2026-09-23-indexed-write-back`): `read_psd`
retains the pre-normalization Lab or CMYK color planes (composite and every layer
color channel, recursing into groups) and `write_psd` re-emits them exactly when
unchanged — avoiding the up-to-19-LSB drift an 8-bit Lab re-encode would cause —
or converts the working RGB with a profile-free inverse when edited (Lab
quantizes; the CMYK `rgb_to_cmyk` is an exact right-inverse). CMYK output adds a
fourth color channel (the composite plane loop, the document-extra offset, and
each layer's synthesized `(C,M,Y,K)` channels). The Lab change also fixed a
pre-existing read bug: the color-mode normalization now recurses into layer
groups. For Indexed, the read retains the 768-byte palette
(`Document.source_palette`, `color_mode_data` stays cleared) and each index plane,
and the writer emits mode Indexed with the palette and one index channel only
while every retained plane still expands to the working RGB; the mode is
document-wide, so any edit or an added color layer falls back to RGB (no
RGB-to-palette quantization is invented). For Bitmap, the read retains the raw
packed composite plane and the writer re-emits mode 0 / depth 1 at the source
compression while the document is flat and unchanged; an edited, layered, or
extra-channel Bitmap falls back to RGB (no RGB-to-1-bit threshold is invented,
and layered/extra-channel Bitmap output is out of scope). A 16/32-bit CMYK or Lab
source now also saves back in its source mode **and** source depth (archived
`depth-color-mode-write-back`): the read retains the native Lab/CMYK color planes
(composite and layer) and an unchanged plane re-emits the native samples
byte-identically, while an edited plane re-encodes 8-bit and widens. 16/32-bit
depth now
reads and normalizes to 8-bit on load too (G4, archived `depth-read`):
`source_depth` records the original, every channel (color, alpha/mask,
spot/extra, and document extras) is narrowed, and the app shows a conversion
notice. **The source depth is now preserved on save** (archived
`2026-09-23-depth-preserve`): for a 16/32-bit Grayscale/RGB document `read_psd`
retains the decoded source-depth samples of the composite, extra, and layer
channels, and `write_psd` writes the header at the source depth — re-encoding an
unchanged plane exactly at the recorded compression, and widening an edited (or
moved) plane's 8-bit bytes (`v*257` at 16, scaled to `[0,1]` at 32). All four
compression kinds are depth-aware; the retained copy is dropped on a
scale/rotate/flip. A converted mode now preserves its depth too when the read
retained samples (a 16/32-bit Lab/CMYK source; archived
`depth-color-mode-write-back`), so only a converted mode with no retained native
store — an 8-bit Lab/CMYK document, which writes 8-bit in its source mode — and
the Indexed/Bitmap paths save 8-bit as before. The remaining depth work is a
true `u16`/`f32` sample model that preserves depth through *editing*: the
sample-typed store foundation now ships (`bit-depth-sample-model` — `PixelBuffer<T
= u8>` is generic and the retained native samples are typed `u8`/`u16`/`f32` with
byte-exact encode/decode), and the tonal adjustment family now applies at native
depth (`native-depth-adjustments`: `apply_native` runs Invert/Desaturate/Levels/
Curves/BrightnessContrast/Exposure/Posterize/Threshold/GradientMap on a `Samples`
store, with the 8-bit path byte-identical and a depth-16 edit keeping its low
bits through a save/re-read), and the color-preserving family joins it
(`native-depth-color-adjustments`: HueSaturation/Vibrance/ColorBalance/BlackWhite/
PhotoFilter/ChannelMixer/SelectiveColor), and the CPU compositor applies
adjustment layers at native precision and can emit a source-depth composite
(`native-depth-composite`: no more 8-bit round-trip for a 16/32-bit document,
`composite_native` returns `U16`/`F32`) and a high-depth RGB/Grayscale pixel layer
reads its native samples (`native-depth-layer-content`) and `Auto`/`ColorLookup`
apply natively (`native-depth-remaining-adjustments`) and high-depth layers gate on
their native `-2` mask (`native-depth-masks`), and a dirty save re-emits the native
composite (`native-depth-save`), and a moved/transformed/warped/resized/oriented/
canvas-edited layer keeps a re-anchored or resampled native store
(`native-depth-edit-preserve`; unmodeled raw streams, merge/flatten, and a true
working-depth model stay open).
A real 32-bit HDR tone map now ships as the CS6 HDR Conversion 32→16/8 path
(`hdr-conversion`, `hdr-conversion-methods`): `Image > Mode > 16/8 Bits/Channel`
tone-maps the retained `f32` composite with Local Adaptation, Equalize Histogram,
Exposure and Gamma, or Highlight Compression into a 16- or 8-bit store. The three
closed kernels are inferred approximations, so per-layer tone mapping and the
Local Adaptation Toning Curve/Corner remain open, along with
Multichannel/Duotone (no
natural RGB mapping / needs the spot-ink spec), and text (`TySh`).
Color-mode
write-back is complete for the modes with an exact representation (8-bit
Lab/CMYK/Indexed, flat Bitmap, and 16/32-bit Lab/CMYK).

**P5 — PSB write / large documents.** *(shipped)* (G9)
`write_psd` emits a version-2 container when the source document was a PSB
(`Document.is_psb`) or either dimension exceeds 30 000, and `write_psb` forces
one; dimensions are accepted to 300 000. The PSB container widens the
layer-and-mask section length, the layer-info length, and each per-channel data
length to `u64`, and the RLE count entries to `u32`. The tagged-block framing it
exposed is now correct: a PSB big key (the psd-tools `_BIG_KEYS` set) carries an
8-byte length; a per-layer block declares an even length with the pad inside it,
while a document-level block declares its exact length and is padded externally
to 4, and a preserved document-level block is re-framed to the output container's
width. `iOpa` is written as a 4-byte `B3x` value. Shipped as the archived change
`2026-09-19-psb-write`; proven by the psd-tools oracle (authored-smart-object
PSB, PSD→PSB reframe, odd/non-4-multiple block framing). Remaining ceilings: an
`8B64` signature (document- and layer-level) is normalized to `8BIM` on write
(a per-layer `8B64` block now reads instead of erroring, `psd-tagged-block-8b64`),
and the app cannot yet open
a >30 000 PSB (import probe budget).

**P6 — Metadata & ICC integration.** File Info + assign/convert on open/save
(overlaps P2/P4). The resource parser shipped (archived
`2026-09-22-psd-image-resources`): `pictura_codec::decode_image_resources`
returns typed `(id, name, data)` records and exposes the ICC/EXIF/XMP/IPTC ids,
with the raw section still re-emitted byte-for-byte. The embedded **ICC profile
is now applied on read** (archived `2026-09-22-psd-icc-convert`): an RGB file
with a non-sRGB profile is converted to the sRGB working space (composite and
layer color channels, relative colorimetric), the original bytes are recorded in
`Document.source_icc`, and resource 1039 is dropped so the save is not
mis-tagged; a Grayscale or untransformable document is left untouched. A
read-only **File Info** surface is now shipped too (archived
`2026-09-22-psd-file-info`): `File > File Info…` shows decoded EXIF (1058/1059)
and IPTC-IIM (1028) fields plus the raw XMP packet (1060), via
`pictura_codec::{parse_exif, parse_iptc, read_metadata}`; the fixture is proven
against `exiftool`. **IPTC editing and write-back** now ship (archived
`2026-09-22-psd-iptc-write`): the dialog's IPTC page edits the six core fields
and OK writes them into resource 1028 as one undo state
(`frame_image_resource`, `Iptc::set`/`remove`, `encode_iptc`,
`set_iptc_fields`), preserving every other resource byte-for-byte and leaving a
section that does not decode losslessly untouched; a save persists the edit
(proven via `exiftool`), and clearing a field removes its record. Ceilings: XMP
is raw text and is not edited (no field extraction, no IIM↔XMP sync), a
multi-value EXIF tag decodes as raw bytes, only the six core IPTC fields are
editable, and IIM values are written as UTF-8. **Assign Profile and Convert to
Profile** now ship (archived `2026-09-22-assign-convert-profile`): a document
carries an optional working-profile ICC, assign retags without touching pixels,
convert transforms the composite and every layer (including group children) and
retags, the canvas converts the final composite to sRGB for display, and a save
tags resource 1039; each is one undo step. Ceilings: RGB 8-bit only, the three
built-in profiles only (no installed-profile discovery), fixed
relative-colorimetric intent with no black-point compensation, dither, or
flatten. **XMP is now parsed and edited** (archived
`2026-09-22-xmp-metadata`): resource 1060 is decoded into a fixed typed property
set shown in a read-only Description category, and editing the six IPTC-Core
fields writes both the XMP packet and the IIM record so the two channels agree.
The packet is patched by byte-span — unmanaged bytes, unknown namespaces, and
the wrapper survive verbatim — and a packet that cannot be safely rewritten is
left untouched. Ceilings: only the nine managed properties are modelled (IPTC
Extension and arbitrary RDF are preserved but not editable), the raw packet is
read-only, EXIF is not editable (camera data is read-only per `WF-010`), and a
non-primary `x-default` alternative collapses to the edited scalar. **Metadata
templates** now ship (archived `2026-09-22-metadata-templates`): File Info can
export the managed properties as a standalone `.xmp` and apply one with Append
(fill empty only), Replace (overwrite, clearing template-omitted fields), or
Keep Original (overwrite only template-defined fields). Apply patches in place,
merges lists as full `rdf:Seq`/`rdf:Bag`, keeps XMP and IIM in sync, and is one
undo step; camera data and unknown properties survive. **Color Settings now
ships** (archived `2026-09-22-color-settings`): `pictura_color::Policy { Off,
Preserve, Convert }` and `pictura_codec::read_psd_with` let the user choose
whether an incoming non-sRGB embedded profile is preserved (pixels unchanged,
profile tagged, display-converted via `document_icc`), converted to sRGB (the
old behaviour), or ignored; the policy is a persisted application preference
(default Preserve) and `Edit > Color Settings…` is a real dialog. `read_psd`
keeps Convert for existing callers. Ceilings: sRGB working space only, RGB
policy only, no `.csf`, no mismatch/missing dialogs. **The saved profile now
always matches the output color mode** (archived
`2026-09-23-icc-output-mode-consistency`): a framable resource `1039` whose ICC
data-space signature does not match the output header color mode is dropped at
the writer, closing the path where a 16/32-bit CMYK or Lab source (normalized to
RGB, its CMYK/Lab profile preserved) saved RGB bytes still tagged CMYK/Lab.
Still open: sidecars.

## Smart objects and Camera Raw interop

PSD is the one format that can round-trip Camera Raw edits, through embedded
smart objects. Two storage models show up, and the reference files confirm the
container and the CC model.

**Smart object container.** The layer carries a config descriptor in `SoLd`
(legacy `PlLd`) holding placement, transform, warp, and a `uuid`. The
document-level `lnkD`/`lnk2`/`lnk3` tag holds a list of source records; the one
whose `uuid` matches the layer is the embedded source. For embedded content its
`kind` is DATA and its payload is the file bytes. Photoshop also writes an empty
`lnkE`. Confirmed from `assets/test_with_smart_object01.psd`: record version 7,
`filetype` `8BPB`, `creator` `8BIM`, payload an embedded PSB, `child_id` a single
NUL, `mod_time` 0.0, `lock_state` 0.

**Camera Raw settings, two models.**

- A raw opened as a Smart Object stores `crs:` XMP in the embedded raw payload.
  This is the CS6 path; `CrsSettings` lifts the eleven PV2012 Basic scalars and
  `set_crs_property` edits one in place (archived `crs-xmp-edit`), but no real
  ACR fixture exists to prove it.
- A Camera Raw Filter applied as a smart filter stores its settings inside the
  layer's `SoLd` descriptor at `filterFX.filterFXList[].Fltr`, with `filterID`
  2683. This is the synthetic path and `assets/test_with_smart_object02.psd`
  confirms it. `Fltr` uses short keys that map to the `FILT-100` tabs: `Temp`,
  `Tint`, `WBal`, `Sett`, and PV2012 Basic `Ex12`/`Cr12`/`Hi12`/`Sh12`/`Wh12`/
  `Bk12`/`Cl12`/`Vibr`; HSL `RHue`/`RSat`/`GHue`/`GSat`/`BHue`/`BSat` and
  `HA_*`/`SA_*`/`LA_*` per eight ranges; split toning `STSH`/`STSS`/`STHH`/`STHS`/
  `STB`; tone curve `PC_*` parametric and `Crv`/`CrvR`/`CrvG`/`CrvB`; detail
  `Shrp`/`ShpR`/`ShpD`/`ShpM` and `LNR`/`CNR`; lens `LPEn`/`MDis`/`VigA`/`Per*`/
  `DfP*`; effects `GRNA`/`GRNS`/`GRNF` and `PCV*`; calibration `CamP`/`CP_D`/
  `PrVe`. `Dhze` (Dehaze) and `Upri`/`GuUr` (Upright XMP) are CC-only.

**Scope.** Smart objects are CS6→current CC: tolerant read plus byte-preserving
write, proven only on the synthetic fixture. The Camera Raw settings model targets
the earliest CC Camera Raw Filter (ACR 8 / PV2012); `crs:` XMP is typed and
editable (`crs-xmp-edit`), preserve-only for keys outside the fixed set.
The app exposes `Layer > Smart Objects > Convert to Smart Object` (archived
`2026-09-19-convert-to-smart-object`): it keeps the raster proxy and authors the
embedded `SoLd`/`lnk2`. `Layer > Rasterize > Smart Object` (archived
`2026-09-19-rasterize-smart-object`) materializes the content into the layer
channels and drops the preserved `SoLd`/`SoLE`/`plLd` block and its document
`lnk*` record (`pictura-codec::remove_linked_source`). `File > Place…` (archived
`2026-09-19-place-smart-object`) inserts a PSD/PSB as a channel-less top
smart-object layer that renders from its embedded source. `Layer > Smart
Objects > Replace Contents…` (archived `2026-09-19-replace-smart-object-contents`)
swaps the embedded source while preserving the layer's transform and re-authors
a fresh link on save. `File > Open As Smart Object…` (archived
`2026-09-19-open-as-smart-object`) opens a PSD/PSB as a new untitled document
with one embedded smart-object layer. `Layer > Smart Objects > Export
Contents…` (archived `2026-09-19-export-smart-object-contents`) writes the stored
payload byte-for-byte and records no state. `Layer > Smart Objects > Edit
Contents…` (archived `2026-09-19-edit-smart-object-contents`) opens the embedded
source in an untitled editor tab and re-embeds it on save. Linked objects are
deferred.

Double-clicking the layer reopens ACR from the stored source and settings. CS6
supports embedded objects only; linked objects (`lnkE`, external paths) are CC
2014, so a raw smart object is self-contained and large.

Open items: a 16-bit fixture, produced later, and whether Photoshop regenerates
or trusts our merged composite.

## Non-PSD image import

`File > Open` and `File > Place…` accept common raster images
(PNG/JPEG/GIF/BMP/TIFF/WebP) beside the native PSD/PSB path; Qt decodes at the
app boundary and the engine builds its own document/layer structures from the
pixels (shipped as `2026-09-19-image-import`). OS file drag-and-drop is shipped
as `2026-09-19-file-drop-routing`: a drop on the document canvas places each
image into the current document, while a drop on the tab strip, menu bar, or
options bar opens each file as its own tab, reusing the same PSD-native vs
Qt-decode routing. Free Transform shipped as `2026-09-19-free-transform-mode`:
a successful place (menu command or canvas drop) selects the new layer and enters
an interactive move/scale/rotate session that commits one state on Enter and
cancels bit-identically on Escape. Skew, distort, and perspective now ship as
`2026-09-25-free-transform-quad` (the projective `transform_layer_quad` and the
three `Edit > Transform` modes, gesture semantics inferred); Warp now ships as
`transform-warp-presets` (the 15 CS6 preset meshes ported from
SethRobinson/Patchy and a one-shot Warp dialog through the shipped mesh op, but
not the interactive cage); Puppet Warp and Content-Aware Scale remain deferred
follow-ups.

## Reference fixtures

Supplied, currently in `assets/`, to move under
`crates/pictura-codec/tests/fixtures/` when the oracle lands:

- `test_with_smart_object01.psd`: 512x512 RGB 8-bit, one embedded PSB smart
  object with no filter. Confirms the `SoLd`/`lnk2` container schema.
- `test_with_smart_object02.psd`: the same, plus a Camera Raw Filter smart
  filter (`filterFX`, `filterID` 2683) and the document `FEid`/`FMsk` blocks.
  Synthetic (no Adobe assets). Confirms the `Fltr` settings model.

Both are self-produced; record provenance when they move.

To produce later: a 16-bit variant of either, which needs roadmap G4 depth
support.

They feed a round-trip oracle: read with `read_psd`, write with `write_psd`,
re-read, and compare the smart-object and smart-filter blocks with `psd-tools`.
The final check is manual: open our saved file in Photoshop CC and confirm the
smart object and the Camera Raw Filter are present and editable.

## Sequencing rationale

P1 first: it converts the most real files from "refuses to open" to "opens
(possibly flattened)" and is a small, self-contained codec change. P2 second:
today a save silently strips profiles, metadata, effects, smart objects, and
text, which is a data-loss hazard worse than a refusal. P2.5 third: preserving a
smart object is only half the contract; owning it, including the CC Camera Raw
Filter settings it carries, is what Adobe interop is judged on and the only
Camera Raw support we can validate against files we can produce. Rendering
parity (P3+) is the long tail and can proceed feature by feature.
