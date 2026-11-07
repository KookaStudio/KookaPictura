# Proposal

## Why

The Channel Mixer is RGB-only in the app. `channel_mixer_dialog.h` carries a
`ponytail:` that admits it has no Monochrome checkbox, and the engine's
`ChannelMixerParams` holds a 3×3 RGB matrix with no CMYK rows, so a CMYK
document can only show RGB outputs. The docs/code parity register records this
as `ADJ-1` (`docs/dev/doc-code-parity-gaps.md`), filed as #571.

The behavioural contract (`docs/04-image-ops/adjustments/channel-mixer.md`)
requires the Monochrome mode with a Gray output and, for a CMYK document, the
Cyan/Magenta/Yellow/Black outputs fed by C/M/Y/K sources. The spec marks the
CMYK row *(inferred)* because no CS6 capture exists; this change implements the
inferred contract and records the inference rather than claiming verified
parity.

## What Changes

- **Monochrome UI.** The Image > Adjustments dialog and the Properties panel gain
  a Monochrome checkbox in RGB documents. Enabling it switches the Output
  Channel to `Gray`; enabling from the identity rows seeds the 40/40/20
  luminance mix. Disabling it keeps the Gray mix as the Red row and restores the
  Green/Blue identity rows (the handtint baseline).
- **CMYK engine.** `ChannelMixerParams` gains a 4×4 ink matrix and four ink
  constants. A non-monochrome mixer applied to a CMYK document converts the
  working RGB backdrop to unit CMYK with the document's embedded CMYK profile
  when the document is 8-bit, otherwise the profile-free GCR, applies the ink
  matrix, and converts back. A negative constant adds ink; a positive constant
  removes it.
- **CMYK persistence.** The 44-byte `mixr` block already has room: each 10-byte
  row is five shorts, and the two bytes ag-psd skips as reserved carry the K
  source in a CMYK row. The RGB form is unchanged. The CMYK layout is inferred
  (no independent decoder models it) and covered by round-trip and
  hand-computed tests.
- **Mode-aware UI.** The dialog and the Properties panel show Cyan/Magenta/
  Yellow/Black outputs and four source rows for a CMYK document, and keep the
  RGB rows and the Monochrome checkbox for an RGB document.
- **Native depth.** A 16/32-bit CMYK document uses the f32 GCR path, so the ink
  mix is not quantized to 8 bits.

## Capabilities

### Modified Capabilities

- `imaging/image-adjustments`: the Channel Mixer kernel gains the CMYK matrix,
  constants, and validation.
- `imaging/adjustment-layer-rendering`: `mixr` decode/encode gains the CMYK
  interpretation, and the compositor and destructive paths apply it in ink
  space.
- `imaging/native-depth-adjustments`: CMYK Channel Mixer stays in the f32 unit
  domain at 16/32-bit.
- `ui/properties-panel-editing`: the descriptor page gains the Monochrome check
  and the CMYK output groups.
- `ui/adjustment-catalogue`: the Channel Mixer dialog gains the Monochrome
  checkbox and the CMYK output/source rows.

## Impact

- `crates/pictura-adjust/src/types.rs`, `color.rs`, `apply.rs`, `native.rs`:
  the ink matrix, the CMYK kernel, and tests.
- `crates/pictura-render/src/channel_mixer.rs`: the mode-independent four-row
  decode and the ink encoder.
- `crates/pictura-render/src/channel_mixer_cmyk.rs` (new): RGB↔unit-CMYK
  conversion, the profile-aware path, and the GCR fallback.
- `crates/pictura-render/src/composite_native.rs`, `filter.rs`,
  `adjustment_params.rs`, `adjustment_defaults.rs`: mode-aware application,
  descriptors, and defaults.
- `crates/pictura-app/src/cxxqt_object/*` and `cpp/channel_mixer_dialog.*`,
  `cpp/panels/properties_panel.cpp`, `cpp/adjustment_dialog.*`: the bridge and
  the UI.
- `ROADMAP.md`, `docs/dev/STATE.md`, `docs/dev/doc-code-parity-gaps.md`: tick
  #571, close `ADJ-1`, and record the CMYK inference.
- Non-goals: CMYK channel mixing for Lab/Grayscale/Indexed/Bitmap/Multichannel
  documents (the spec says unavailable), converting an existing RGB mixer when
  a document changes mode, and a CS6-verified CMYK `mixr` layout (no capture or
  independent decoder exists).
