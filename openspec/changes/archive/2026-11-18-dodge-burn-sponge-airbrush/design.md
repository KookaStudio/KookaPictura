# Design: dodge-burn-sponge-airbrush

## Airbrush build-up

`ToneBrush` already records the coverage each pixel has received so a normal
stroke tones a pixel once. Airbrush is the opposite contract: a held button
keeps adding effect. The engine therefore keeps the coverage cap only when
`airbrush` is off; with it on, every dab contributes `cover * amount` and the
`applied` plane is not consulted. The existing `Stroke::hold` path (already used
by the Brush and stamps) repeats the last dab at `kAirbrushHoldDabMs`; the
toning handler starts the same timer when Dodge/Burn Airbrush is on.

Sponge has no Airbrush option in CS6, so it never starts the timer even though
the shared `ToneOptions` carries the field.

## Grayscale

A Grayscale layer stores one colour plane. The stroke surface expands it to
RGBA as `[grey, 0, 0, alpha]`, which would make Dodge/Burn use a red-only
luminance and Sponge treat the missing planes as saturation. `Stroke::begin_tone`
sets `grayscale` from `doc.mode`, and the tone kernels then use channel 0 as the
grey value:

- Dodge/Burn move that value with the same Range-weighted Exposure model;
  Protect Tones moves it directly rather than through `shift_luminance`.
- Sponge moves it away from (Saturate) or toward (Desaturate) 0.5. Vibrance
  eases the change near 0 and 1, where there is little contrast to add or
  remove.

The write-back path already writes only the planes the layer owns, so keeping
the unused channels in the returned RGBA preserves the "changed?" test.

## Quick keys

The four CS6 painting-mode keys are window `QShortcut`s that call one
`ToolController::applyToneQuickKey` entry point, so they only act when the
active tool is a toning tool and the model stays the single source of truth.
The controller emits `toneOptionsChanged`; the toning page re-reads the Range or
Mode combo, Exposure/Flow, Protect Tones/Vibrance, and Airbrush under signal
blockers. The same signal lets `Shift+Alt+P` update the Airbrush toggle.

## 32-bpc availability

The engine already returns `UnsupportedDepth` from `begin_tone`. The UI gate
mirrors the sibling Color Replacement mode gate: one
`toolAllowedForDocument(ToolId, const PictureView*)` predicate, a
`ToolController::setActiveTool` refusal plus a `bindCanvas` Move fallback, a
`Toolbox::available` helper used by every flyout/click/key path, and a
`Toolbox::refreshToolAvailability` keyed on document depth and called from
`PicturaMainWindow::refresh`. The predicate is per document, not per tool
edition, so 8- and 16-bpc documents keep the tools.
