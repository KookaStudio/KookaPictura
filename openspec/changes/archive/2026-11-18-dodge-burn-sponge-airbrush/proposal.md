# Proposal: dodge-burn-sponge-airbrush

## Why

Issue #548 (parity gap TOOL-24) records four CS6 behaviours the toning tools do
not yet have: Sponge is a no-op on a Grayscale document, Dodge/Burn have no
Airbrush toggle, the painting-mode quick keys are missing, and the three tools
remain selectable on 32-bpc documents even though the engine refuses to paint
there. The long-form contract is `docs/03-tools/dodge-burn-sponge.md`.

## What Changes

- `pictura_paint::tone`: `ToneOptions` gains `airbrush` (Dodge/Burn build-up)
  and `grayscale` (set by `Stroke::begin_tone` from the document mode). The
  per-dab engine applies `cover * amount` on every Airbrush dab instead of the
  once-per-stroke coverage cap, and reads a Grayscale layer's single colour
  plane as the grey value for Dodge, Burn, and Sponge. Sponge in Grayscale
  moves grey levels away from (Saturate) or toward (Desaturate) middle grey.
- `cxxqt_object/paint_tools`: `begin_tone` carries the Airbrush flag.
- `tool_retouch.cpp`: the toning handler owns a held-dab timer, started for
  Dodge/Burn with Airbrush on and stopped on release, tool switch, or cancel.
- `options_bar_retouch.cpp`: Dodge/Burn get the Airbrush toggle; the bar follows
  model changes made by the shortcut or a quick key.
- `ToolController`: `Shift+Alt+P` toggles Dodge/Burn Airbrush; the four tone
  quick keys set Range (Dodge/Burn) or Mode (Sponge); a document-aware
  `toolAllowedForDocument` gate mirrors the sibling mode-gate pattern.
- `Toolbox` and `refresh()`: the tone flyout rows disable on a 32-bpc document
  and an active tone tool falls back to Move.
- `tst_retouch_tools`: held build-up, the Airbrush shortcut, the quick keys,
  Grayscale contrast, and the 32-bpc disable/fallback.

## Capabilities

### New Capabilities

- `tools/dodge-burn-sponge-options`: the shared Airbrush, quick-key, Grayscale,
  and 32-bpc availability requirements of the three toning tools.

### Modified Capabilities

- `tools/sponge-tool`: qualify "a grey pixel is left unchanged" to RGB
  documents; a Grayscale document moves grey levels around middle grey.

## Impact

- `crates/pictura-paint` (`tone.rs`, `stroke.rs`), `crates/pictura-app`
  (bridge, tool handler, options bar, toolbox, shortcuts), tests.
- No new dependency.

## Provenance

CS6 behaviour from the public Help reference already recorded in
`docs/03-tools/dodge-burn-sponge.md`; the implementation remains behavioural
parity only, with the exact Airbrush rate, Grayscale contrast curve, and Range
weighting marked inferred.
