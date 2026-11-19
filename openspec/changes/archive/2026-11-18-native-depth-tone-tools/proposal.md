# Proposal: native-depth-tone-tools

## Why

Issue #1421 (follow-up to #548) records that the Dodge, Burn, and Sponge tools
are available on 16-bpc documents but do not edit the pixels the compositor
reads. The working model is 8-bit, so `Stroke::begin_tone` writes only the
layer's 8-bit channels; a 16-bpc layer's retained native store is authoritative
for display and save, so the stroke has no visible effect. The bit-depth spec
expects these tools at 16 bpc, while 32 bpc stays excluded in CS6.

## What Changes

- `pictura_paint::tone`: add a native 16-bit tone brush that reads and writes
  the target layer's retained native colour planes at 16 bits, applies the same
  Dodge/Burn/Sponge model, mirrors the result into the 8-bit channels, and
  applies selection coverage at the native depth. A Grayscale document uses its
  single native plane.
- `Stroke::begin_tone`: allow 16-bpc documents with an authoritative native
  store, and refuse a retained 32-bit source store as well as a 32-bit working
  document.
- `Stroke`'s saved-pixel snapshot: keep the native planes a tone dab changes so
  a cancelled stroke restores them.
- `tone` tests: 16-bpc Dodge, Burn, Sponge, Grayscale, native precision, cancel,
  and retained-32-bit refusal.
- `tst_retouch_tools`: a 16-bpc Dodge drag lightens the displayed canvas and
  records one history state.

## Capabilities

### New Capabilities

- `tools/native-depth-tone`: 16-bpc Dodge, Burn, and Sponge strokes edit the
  retained native samples at the document depth; 32 bpc stays unavailable.

### Modified Capabilities

<!-- None: the existing tone tool, options, and fill capabilities keep their
     contracts; this adds the native-depth path those specs leave open. -->

## Impact

- `crates/pictura-paint` (`tone.rs`, `tone_native.rs`, `stroke.rs`,
  `stroke/grow.rs`), `crates/pictura-app/cpp/tests/tst_retouch_tools.cpp`.
- No new dependency.

## Provenance

The 16-bpc availability follows the public CS6 Help exclusion list already
recorded in `docs/03-tools/dodge-burn-sponge.md`; the native write path mirrors
the repository's existing `fill_native` design. Adobe's exact tone curve remains
closed, so the operator stays behavioural parity only.
