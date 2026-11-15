# Proposal

## Why

The Photoshop PDF writer shipped with three Save options that are listed but do
not work: JPEG 2000 is disabled in the dialog and silently coerced to JPEG in
the options, Spot Colors is forced off and cannot even survive a preset
round-trip, and Use Proof Setup is an inert checkbox because no proof setup
exists. The dialog therefore advertises capabilities it lacks, and the
`interop/pdf-export` contract currently *requires* that broken state.

## What Changes

- **JPEG 2000 compression** becomes selectable for 8-bit continuous-tone images
  at PDF 1.5 (Acrobat 6) and later, encoded as a JPEG 2000 codestream with
  `/JPXDecode`. Below that level, with a 16-bit image, or under PDF/A-1b the
  option coerces to ZIP instead of silently becoming JPEG. The Tile Size field
  is enabled when JPEG 2000 is selected.
- **Spot Colors** becomes a real Save option: the PSD DisplayInfo resource
  distinguishes spot channels from alpha channels, and the embedded Photoshop
  payload keeps or drops each kind independently. Filtering the payload also
  rewrites the channel-name and display-info resources so names stay aligned.
- **Use Proof Setup** becomes functional: a new View > Proof Setup feature
  (Working CMYK, Working Gray, Custom) with Proof Colors (`Ctrl+Y`), an lcms2
  soft-proof preview of the canvas, session persistence, and a title-bar
  indicator, plus the Save Adobe PDF option that converts the page image to the
  active proof profile. Convert To Destination on the Output page overrides the
  Save As proof option, as Photoshop warns.
- Two capability contracts change: the JPEG 2000 and Spot Colors requirements
  no longer mandate the disabled state, and a new proof-setup capability
  specifies the View feature and its PDF use.

## Capabilities

### New Capabilities

- `color/proof-setup`: View > Proof Setup and Proof Colors selection, the
  lcms2 proof-preview transform, the Custom dialog, the canvas proof display,
  session persistence, and the title-bar proof indicator.

### Modified Capabilities

- `interop/pdf-export`: JPEG 2000 is selectable (not disabled) and gated by
  compatibility, bit depth, and standard; Spot Colors preserves or drops spot
  channels instead of being disabled; the Save Adobe PDF dialog lists the Save
  options including Use Proof Setup; and Use Proof Setup converts and tags the
  page with the active proof profile.

## Impact

- `crates/pictura-codec`: new `jpeg2k` dependency (pure Rust), PDF image
  encoding, options and `.joboptions` keys, PSD image-resource 1077 parsing and
  writing, embedded-payload channel filtering.
- `crates/pictura-color`: proof-preview and proof-output transforms over lcms2.
- `crates/pictura-app`: view proof state and bridge, canvas proof display,
  proof menu handlers and Custom dialog, session keys, PDF dialog bindings,
  command catalogue and generated command ids.
- Tests: codec unit tests, colour unit tests, `tst_save_pdf`, a new
  `tst_proof_setup`, and poppler/psd-tools oracle checks.
- Docs: `ROADMAP.md`, the parity-gap register, the JPEG 2000 dependency note,
  `docs/01-architecture/color-management.md` (application-level proof setup),
  and `docs/dev/STATE.md`. No new domain or layout change.
