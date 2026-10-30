# Proposal: the picker's CMYK evaluation follows the Color Settings conversion options

## Why

Three specs disagree about the options behind the picker's CMYK path. Two —
`color/color-settings` and `color/color-management` — say the Color Settings
rendering intent and black-point compensation SHALL apply to the picker's
working-CMYK evaluation. `color/color-model-conversions` fixes the picker at
relative colorimetric with black-point compensation.

The code follows the third: `picker_options()` always returns the defaults, and
`set_color_settings` rebuilds the picker pair only from the profile. So changing
the intent or black-point compensation in Color Settings has no effect on the
picker's C/M/Y/K readouts, CMYK field edits, gamut alert, or closest printable
colour, or on the Info panel's shared CMYK readout (#1054).

## What changes

- The picker pair is built from the installed rendering intent and black-point
  compensation, and is rebuilt when either changes. Dither is never applied to
  the picker, which evaluates a single colour rather than a plane.
- `color/color-model-conversions` drops the fixed-intent wording and defers to
  the installed options, reconciling it with the other two specs.
- `color/color-settings` and `color/color-management` name the intent and
  black-point compensation for the picker and exclude dither.

## Approximation

Whether CS6's own picker follows the Color Settings intent is not confirmed
against a capture; this ships the spec-consistent behaviour and records it as an
approximation, not verified parity.

## Capabilities

- Modified: `color/color-model-conversions`, `color/color-settings`,
  `color/color-management`
