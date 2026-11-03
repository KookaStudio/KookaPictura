# Proposal

## Why

Clone Stamp, Pattern Stamp, History Brush and Art History Brush paint only while
the pointer moves (#799). The Help and `docs/07-color-painting/airbrush-and-flow.md`
make hold-to-build an Airbrush behaviour: with Airbrush off, a still pointer adds
no paint. These tools have no Airbrush option, so the behaviour cannot be reached.
The Art History Brush has no Airbrush option in its bar and keeps painting while
held, which its source already records as a known gap.

## What Changes

- **Airbrush toggle.** Clone Stamp, Pattern Stamp and History Brush each gain an
  off-by-default Airbrush toggle in the options bar. While it is on, a held button
  keeps laying dabs at the pointer.
- **Art History Brush.** It always keeps painting while the button is held.
- **Held-dab path.** The engine gains a dab at the stroke's last sample, because
  the spacing walker places nothing for a still pointer. A bounded timer drives it
  at 20 dabs per second; the CS6 rate is undocumented, so this is an approximation.
- **Stroke lifecycle.** A tool switch during a stamp stroke commits it, as the
  Brush does, and a session cancel abandons it. Either stops the held dabs.

## Capabilities

### Modified Capabilities

- `tools/clone-stamp-tool`: adds the Airbrush held-button requirement.
- `tools/pattern-stamp-tool`: adds the Airbrush held-button requirement.
- `tools/history-brush-tool`: adds the Airbrush held-button requirement.
- `tools/art-history-brush-tool`: adds the held-button requirement.

## Impact

- `crates/pictura-paint`: `DabPlacer::repeat_last`, `Stroke::hold`.
- `crates/pictura-app`: `paint_hold`, `paint_api::Stroke::hold`, the stamp handler,
  `StampOptions`, the stamp options bars.
- The Brush's Airbrush option and `Shift+Alt+P` stay out of scope (#1120).
