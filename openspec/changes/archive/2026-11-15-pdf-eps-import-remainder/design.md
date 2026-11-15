# Design

## Context

See `proposal.md` for motivation. The shipped import path is:

```
ImportTarget(mode, depth)
      |
      v
gs pngalpha -> Raster RGBA8 -> Document::from_rgba
      |
      v
convert_mode(mode) -> convert_bit_depth(16)      open_jobs/imp.rs
```

`Raster` is RGBA8 by definition (`ghostscript.rs`). The page, EPS, and Images
modes all share `imported_document`, so all three currently fake CMYK and
16 bit by converting one 8-bit RGB render.

Three capabilities already exist in the codebase and shape the work:

- `PdfObjects::decrypt` implements the standard security handler and is used
  by `pdf::editing_psd`; the Images reader simply never gets the password.
- The PSD reader retains native 8/16/32-bit RGB, Grayscale, CMYK, and Lab
  samples in `source_planes` and `source_channels`, and turns a layer-less PSD
  into a display document. Ghostscript's PSD devices emit exactly that shape.
- Ghostscript 10.07 (the CI floor is 9.55) offers `psdcmyk`, `psdcmyk16`, and
  `psdrgb16`; all honour `-dTextAlphaBits`/`-dGraphicsAlphaBits`. None emits
  an alpha channel; they composite the page onto white paper.

Constraints: Ghostscript is an external process (AGPL stays out of the
binary); no new dependency; the Import PDF path has no colour-policy prompt;
documents keep an 8-bit working display with native samples in the source
stores; transparency is part of the page contract.

## Goals / Non-Goals

**Goals:**

- CMYK (8/16-bit) and 16-bit RGB/Grayscale pages, embedded images, and EPS
  carry Ghostscript's native samples, not a converted 8-bit RGB render.
- Transparency and anti-aliasing survive the native route; Crop To geometry
  and naming/resolution behave exactly as today.
- Images mode works on encrypted PDFs once the password is entered.
- Inline images are listed and open at their native size.
- 3D is recorded as out of scope; the radio stays disabled.

**Non-Goals:**

- A 3D engine, 3D layer model, or U3D/PRC parsing.
- 16-bit alpha coverage (the mask stays 8-bit; the document widens it).
- Native 16-bit Lab (the existing Lab encoder is 8-bit and widens).
- A spot-colour UI. Spot channels that Ghostscript writes beyond CMYK are
  preserved as document extra channels, not promoted to a new workflow.
- Inline images inside patterns.
- Device-side cropping or other Ghostscript geometry tricks.

## Decisions

### D1 Native colour uses Ghostscript's PSD devices plus a second alpha pass

For a native target, run Ghostscript twice at the same per-axis resolution:

```
gs -sDEVICE=psdcmyk16   (or psdcmyk / psdrgb16)  -> page.psd  (white paper)
gs -sDEVICE=pngalpha                            -> page.png  (coverage)
```

The colour pass covers the same page geometry the 8-bit path renders, then
Pictura crops. The alpha pass supplies straight coverage. The alternatives:

- `png48` / `tiff64nc` plus new decoders and hand-built documents: more code
  for the same samples, and the existing PSD reader already understands every
  target mode and depth.
- Keep converting from RGBA8: does not satisfy the requirement.
- Fixed-media device cropping: not proven for PDF with `/Rotate` and the six
  Crop To boxes; render-then-crop is already tested.

PSD devices write raw (uncompressed) image data. A full-page render is bounded
by the existing `MAX_RASTER_FILE` and image-budget checks; a crop much smaller
than the page can still produce a large intermediate. That cost is accepted
for now and surfaces as the existing "over the image budget" / Ghostscript
failure rather than a corrupted document.

### D2 Un-composite the white-paper render through the coverage mask

Where page coverage `a` is below full, the PSD colour is composited over white.
Recover straight values in the store's convention (255 = paper/white):

```
stored = 255 - ((255 - observed) * 255) / a         8-bit
stored = 65535 - ((65535 - observed) * 255) / a     16-bit
```

`a` is the 8-bit coverage in both lines. The subtracted term is clamped to the
paper value before the subtraction: the coverage is 8-bit while a 16-bit colour
keeps a sub-coverage-step remainder, so the un-clamped term can exceed the paper
value at a partially covered pixel. `a == 0` leaves the pixel white and
transparent; it is never displayed. The
formula is applied to the native colour planes only; the RGB8 working display is
then derived from the un-composited native store (`cmyk_to_rgb` for CMYK,
narrowing for RGB16) instead of being un-composited independently. Lab and
Greyscale targets derive from the un-composited native RGB16 samples through the
existing conversion code. Alpha remains 8-bit and is widened by the normal depth
store, the same precision the current conversion path has.

### D3 Build the document from the native PSD read, not by converting RGBA8

Read the colour PSD with `read_psd_with_options` under `Policy::Off`, matching
the import path's current profile-free behaviour (the dialog has no
colour-policy choice). The read yields:

```
composite        RGB8 display planes
source_mode      Cmyk                (CMYK targets)
source_depth     Sixteen             (16-bit targets)
source_planes    native CMYK/RGB samples at the target depth
```

The codec returns the cropped native planes, the coverage, and warnings; it does
not build a document. The app (`open_jobs`) builds the document, `Layer 1`, and
the source stores, because `add_raster_layer_from_rgba` lives in
`pictura-render` and the codec cannot depend on it. The native branch produces a
`NativeRender` carried through the bridge in `Imported { Rgba(Rendered),
Native(NativeRender) }`; `finish_rendered` branches on the variant.

Then:

1. Decode the pngalpha mask and un-composite the native store; derive the RGB8
   display from the un-composited native store (`cmyk_to_rgb` for CMYK,
   narrowing for RGB16) rather than un-compositing the display independently.
2. Crop the display RGBA8 with the existing raster crop rule and crop the
   native samples with a plane copy; this avoids relying on `crop_document`,
   which does not crop `doc.composite`.
3. In the app, create `Layer 1` from the cropped display RGBA
   (`add_raster_layer_from_rgba`) and attach `source_channels` copied from the
   cropped native planes plus the alpha plane at the store's depth.
4. Set `source_mode`/`source_depth`/`source_planes` from the cropped native
   store, set the resolution and name, and refresh the stored composite with
   `DocumentComposite::new(doc).refresh(pictura_render::CompositeScope::Full,
   gpu)` (or clear `merged_composite_present` first): `store_opened_composite`
   skips a document that already has a merged composite.
5. For Grayscale targets, run `convert_mode(Grayscale)` after the stores are
   attached so the luma is computed at 16-bit. For Lab, run `convert_mode(Lab)`
   from the native RGB16 document (the resulting Lab store is 8-bit, widened by
   the usual depth path).

Trigger the native route when the target is CMYK (either depth) or depth is 16.
RGB8, Grayscale8, and Lab8 keep the existing single-pass pngalpha route. This
keeps the common case fast and confines the two-pass cost to the modes the
issue names.

### D4 Images mode reuses the same native mechanics per image

`PdfImages::render` already writes a one-page wrapped PDF at the image's pixel
size and renders it with pngalpha. A native variant writes the same wrapper and
runs the native device plus the alpha pass, then builds the document through
the same helper; `pdf_import_open_image` returns the same `Imported::Native`
result as page opens. Thumbnails stay RGBA8. Images mode ignores output size,
so no crop is involved; anti-aliasing stays off as it is today.

### D5 Encrypted Images mode passes the existing password

`PdfImages::read` gains the password:

```
parse -> if trailer has /Encrypt { decrypt(password) }
      -> Unlock::Open ? list_images : None
```

`list_images` keeps its encrypted guard as a final check. The probe already
receives the password; `pdf_import_image_thumbnails` gains it in the bridge and
at its call in `crates/pictura-app/cpp/import_pdf_dialog.cpp:245`, and
`pdf_import_open_image` gains it in the bridge and at its call in
`import_save_import.cpp`. `PdfImage.object` becomes `Option<u32>` for inline
occurrences, so every `PdfImages::read`/`PdfImage.object` call site is updated:
`crates/pictura-app/src/cxxqt_object/pdf_export/import/imp.rs:75,124`,
`crates/pictura-app/src/cxxqt_object/open_jobs/imp.rs:515`,
`crates/pictura-codec/tests/hostile_inputs.rs:617`, and
`crates/pictura-codec/src/pdf/import/tests.rs:301,314,333,342`, including the
`encrypted_pdf_disables_images` test. The reader returns `None` for a wrong or
unsupported password, so Images never enables on a file the password does not
unlock; the page path already prompts again and the dialog is unchanged.

### D6 Inline images are found by scanning decoded content streams

Decode each page's `/Contents` (a stream or an array of streams) with the
existing Flate-only `decode_stream`; only Flate-encoded or unfiltered content
streams are scanned, so a stream with any other filter yields no inline images
but does not stop the other content streams or the XObject listing. Only page
content streams are scanned: inline images inside Form XObject content streams
are not listed.

Scanning follows the PDF inline-image rules:

- `BI` starts the dictionary; parse abbreviated keys into full image keys
  (`W`→`Width`, `H`→`Height`, `BPC`→`BitsPerComponent`, `CS`→`ColorSpace`,
  `D`→`Decode`, `DP`→`DecodeParms`, `F`→`Filter`, `IM`→`ImageMask`,
  `I`→`Interpolate`, `Mask`→`Mask`; a `/CS` value `I` expands to `/Indexed`).
- `ID` is followed by one whitespace byte, then the data. For `AHx`/`A85`
  consume the terminator and the following `EI`; otherwise find `EI` preceded
  by whitespace and followed by whitespace, a delimiter, or end of stream.
- Expand filter abbreviations (`Fl`, `AHx`, `A85`, `DCT`, `CCF`, `RL`, `LZW`)
  and colour-space shorthand (`G`, `RGB`, `CMYK`) when synthesizing the stream.

Each inline image becomes an `Object::Stream` in a one-page wrapper, so
Ghostscript decodes its filter and colour space exactly as for an XObject.
`PdfImage` keeps `page`, `width`, and `height`, with `object: Option<u32>` so
an inline occurrence has no fake object number; `PdfImages` keeps a parallel
source list for rendering. Listing order is per page: XObjects (object identity
deduplicated as today), then inline images in content order, capped by
`MAX_IMAGES`. Inline occurrences are not deduplicated.

### D7 3D remains disabled and leaves #627

The project has no 3D engine; ROADMAP marks 3D tools/layers as not planned.
The radio keeps its CS6 layout position and disabled state. The issue title,
the #627 roadmap row, and parity row WF-3 drop the 3D item; no code changes for
3D.

## Risks / Trade-offs

- **GS PSD compatibility** → the first task is a Ghostscript-backed spike that
  writes `psdrgb16`/`psdcmyk16` and reads them with `read_psd_with_options`.
  If the reader rejects a structural quirk, extend the reader before building
  the app path.
- **Raw PSD size** → full-page intermediates are large (A4 at 300 dpi is about
  70 MB for CMYK16); existing file/decode budgets refuse what will not fit.
  A future fixed-media crop could remove the full-page cost.
- **Two Ghostscript runs** → native imports take roughly twice as long. Only
  CMYK/16-bit targets pay it; RGB8/Grayscale8 keep one run.
- **Un-compositing precision at very low alpha** → the result is clamped; such
  pixels are nearly invisible, and the mask is 8-bit regardless.
- **Spot colours** → a `psdcmyk` PSD may carry extra channels; native import
  keeps only the process colour planes and drops the extra/spot channels (out
  of scope). The working RGB display already shows their alternate tint. No
  new UI is added.
- **Lab 16-bit** → remains an 8-bit Lab store widened to 16; the spec's
  precision guarantee excludes Lab.
- **Inline `EI` false positives/negatives** → follow the spec's delimiter rules,
  terminate ASCII filters at their marker, and treat malformed data as "no
  more inline images" rather than an error. A malformed `BI`/`ID` stops the
  page's scan, so repeated malformed pairs cannot make the scan quadratic.
- **Inline scan on every probe** → `pdf_import_probe` decodes page content
  streams to report the inline-image count before the dialog opens; a large
  PDF pays that cost on every generic-PDF open. It is bounded by the existing
  inflate and image budgets.
- **Encrypted image render copies decrypted streams into a scratch PDF** → the
  scratch directory is already owner-only, and the existing inflate/file
  budgets apply.
- **No native fallback** → a native render failure (an unsupported device, a
  PSD the reader refuses, or a colour/coverage size mismatch) is an explicit
  error; it never silently falls back to the 8-bit path, so the precision
  contract is not broken behind the user's back. The import fails instead.

## Migration Plan

No persisted format, preference, or environment changes. Existing RGB8 and
Grayscale8 imports keep their current code path; native imports produce the
same document shape (working RGB8 display plus source stores), so saving,
history, and reopen are unchanged. Rollback is deleting the change; no data is
migrated.

## Open Questions

None. The issue's 3D item is resolved by removing it from scope; the remaining
choices above are implementation decisions.
