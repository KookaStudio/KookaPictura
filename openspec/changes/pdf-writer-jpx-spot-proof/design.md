# Design

## Context

See `proposal.md` for motivation and the two spec deltas for the behavior
contract. The shipped writer funnels every option through `PdfOptions` and its
canonical `.joboptions` text (`PdfOptions::normalize` / `write_joboptions`), and
the dialog edits that text field by field; a new field without a joboptions key
is lost on the next edit. The engine encodes JPEG through a `PdfEnv` callback
provided by the Qt shell, while WebP is already a pure-Rust engine codec. PSD
image resources are preserved verbatim, and `Document.channels` carries extra
channel pixels with no kind or colour. The canvas presents premultiplied RGBA8888
crops, and `pictura-color` wraps lcms2 6.

## Goals / Non-Goals

**Goals:**

- Make the three Save options real without changing what the rest of the writer
  already guarantees (atomic save, option normalization, preset semantics).
- Keep the JPEG 2000 encoder portable (no native library, no KDE plugin).
- Identify spot channels from data Photoshop already writes, without a new
  authoring model.
- Build one proof setup that serves the canvas and the PDF writer.

**Non-Goals:**

- Writing Separation/DeviceN spot plates into the page image; the option governs
  the embedded Photoshop payload.
- Creating or editing spot channels in the app.
- Importing arbitrary ICC files as proof devices; the five built-in profiles are
  the device list.
- Preserve RGB/CMYK Numbers, colour-blindness simulation, and Gamut Warning.
- Reproducing Adobe's private layered-PDF payload layout.

## Decisions

### D1. JPEG 2000 encodes in the engine with a pure-Rust crate

Add `jpeg2k = "0.11"` to `pictura-codec`. Since 0.11 it is a pure-Rust port of
OpenJPEG and its `Image::encode` returns a raw codestream, which is exactly what
`/JPXDecode` carries; `encode_jp2` would add box framing the filter does not
want. The PDF path builds a `CoreImage` of 8-bit component planes from the
interleaved raster and encodes with `EncodeParameters::tile_size(tile, tile)`,
`irreversible(true)`, and `layer_rate(ratio)`.

Alternatives rejected: native `openjpeg-sys` (CI and licence build cost), the
KDE `kimg_jp2` Qt plugin (not portable; produces JP2), and a `PdfEnv` callback
(more seam for no behavioral gain, and WebP already sets the pure-Rust
precedent). The Quality → compression-ratio mapping is inferred from the five
Image Quality levels; it can be tuned without changing the contract.

Gating stays in one place: `normalize()` allows `Jpeg2000` only at Acrobat 6+,
8-bit, and not PDF/A-1b; otherwise it coerces to ZIP. The dialog disables the
item under the same conditions (`compressionItemEnabled`), and `tileSize` is
locked unless JPEG 2000 is selected. The dialog has no document depth of its
own, so its constructor context gains a `documentIs16Bit` flag; a 16-bit
document disables JPEG 2000 unless Convert 16 Bit/Channel Image To 8
Bit/Channel is on. 16-bit output keeps its spec-mandated ZIP-only rule.

### D2. Spot channels are read from PSD DisplayInfo; no core model change

Resource 1077 DisplayInfo stores one entry per extra channel: colour space, four
components, opacity, and a mode byte (`0` alpha, `1` inverted alpha, `2` spot).
It is already preserved verbatim. `pictura-codec` gains a parse/encode pair and
`spot_channel_indices(&Document)`, so the PDF payload can partition extras
without changing `pictura_core::Channel` or every constructor and history
snapshot that copies it.

When the payload filters extras, it also rewrites resources 1045/1006 (channel
names) and 1077 to the retained channels. This fixes a latent bug: Alpha
Channels off already dropped channels while leaving their names behind.

`SpotColors` becomes a Save option: `PicturaSpotColors` is written and parsed in
the non-preset joboptions text (so dialog edits retain it) and omitted from
preset text, like Layers/Notes/Alpha/ICC. The dialog binds the checkbox, disables
it when the document has no spot channels, and keeps it across preset switches.

### D3. Proof setup is application view state

`docs/01-architecture/color-management.md:257-259` calls proof setup
document-level view state with a persisted global default. This change
deliberately makes it application-level instead: the shell has no per-document
view-state store and the View menu is application-wide. The docs will be updated
to the application-level model.

The setup is therefore global: a device code (Working CMYK, Working Gray, sRGB,
Adobe RGB, ProPhoto), intent, BPC, two simulation flags, and the Proof Colors
toggle. It lives in `PictureViewRust` so the canvas can read it while
presenting, and in `SessionState` so it survives restarts; new and opened views
inherit it through the existing `applyColorSettings` hook. It never enters the
document, history, or PSD.

The Custom dialog edits a copy; OK applies to every view, Cancel changes
nothing.

### D4. The canvas uses an lcms2 proofing transform

`pictura-color` exposes a proof-preview function over interleaved RGBA8 pixels
using `Transform::new_proofing(input=sRGB, output=sRGB, proofing=device)` with
`SOFT_PROOFING | NO_CACHE`; it transforms the RGB channels and copies alpha
unchanged. The canvas crop is premultiplied RGBA8888, so the app unpremultiplies
before calling the proof function and premultiplies again afterwards. The
transform is cached for the active setup and rebuilt when the setup changes.

Proofing applies only on the composite branch: the Quick Mask and solo-mask
crops in `impl_core/display.rs` return before the composite and stay unproofed.
The transform runs per visible crop; if profiling shows a stall, the upgrade is
to proof inside the pyramid cache.

### D5. Simulation mapping is an approximation

lcms2 has one proofing transform, not Photoshop's two simulation switches.
Map Simulate Paper Color to absolute-colorimetric proofing and Simulate Black Ink
to black-point compensation off; both off uses relative colorimetric with BPC.
The mapping is inferred and marked with a `ponytail:` note; tests pin that each
switch changes output, not Adobe's exact numbers.

### D6. PDF proof conversion is runtime data, not a preset field

`PdfOptions` gains only the `useProofSetup` Save-option flag. The device ICC
bytes, intent, and BPC travel in `PdfEnv` (like the clock and entropy), because
they are machine state and would bloat the canonical option text. When the flag
is on and Color Conversion is No Conversion, `build_raster` and `text_fill`
convert working → proof device with the setup's intent/BPC. The proof conversion
writes 8-bit samples and forces Convert 16 Bit/Channel Image To 8 Bit/Channel
on, so a 16-bit document loses its 16-bit output under Use Proof Setup. The
image is tagged ICCBased with the proof profile regardless of the Profile
Inclusion Policy; live text uses the same converted fill. Convert To Destination
wins (Adobe warns that the Save Adobe PDF dialog overrides the Save As color
option); the option locks and the Summary names the override.

### D7. Menu wiring follows the existing catalogue path

The four proof commands become `implemented: true`; Working CMYK, Working Gray,
and Proof Colors are `checkable: true`, while Custom opens a dialog and has no
checked state. The colour-blindness entries and Gamut Warning stay
unimplemented. Const names are added to `const_names.rs` and `command_ids.h` is
regenerated, with `catalogue.tsv` updated. Handlers and checked providers live
in a new `frame_proof.cpp` called from `frame_commands.cpp`, keeping
`menu_registrar.cpp` under its size cap.

## Risks / Trade-offs

- **`jpeg2k` 0.11 is young** → pin the version, cover it with a real encode plus
  poppler render test, and keep the encoder behind one module so it can be
  swapped without touching the writer.
- **Component planes are `i32`** → JPEG 2000 export of a very large document
  costs about 4x the sample memory; the existing image-size validation still
  applies, and the preview/commit paths are unchanged.
- **Proof per crop** → O(visible pixels) per paint; the cached transform and the
  mix-with-duplex upgrade path keep the cost bounded.
- **DisplayInfo assumptions** (13-byte entries, mode 2 = spot) → parse defensively,
  fall back to "no spots", and check with psd-tools plus a synthetic PSD.
- **Resource alignment** → the payload rewrite is covered by tests for spot-on,
  spot-off, and alpha-off separately.
- **Save options vs presets** → `spotColors` and `useProofSetup` join the kept
  fields; the preset writer omits them, matching the existing Save-option rule.
- **Inferred numbers** (JPEG 2000 quality ratios, proof simulation mapping) →
  marked `ponytail:` and isolated to one mapping each; tunable without a spec
  change.

## Migration Plan

No file-format or stored-option migration. Existing sessions load the new keys
with their defaults (Working CMYK, Proof Colors off), and presets keep ignoring
Save options. Rollback is a revert: the source PSD is untouched, and older
builds ignore the new session keys.

## Open Questions

- The exact JPEG 2000 quality ratios and the title-bar suffix wording are
  inferred and can be adjusted without changing the specs or the task order.
