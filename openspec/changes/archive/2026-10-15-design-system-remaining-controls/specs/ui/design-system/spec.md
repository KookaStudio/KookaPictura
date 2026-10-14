# Spec Delta

## ADDED Requirements

### Requirement: Remaining standard controls

Each remaining standard control SHALL have exactly one global stylesheet
definition, selected by object name where a control needs its own surface:
group box, radio button, progress bar, header view, and splitter. Each
definition SHALL cover its interactive states — rest, hover, pressed, checked
(where the control toggles), disabled, and keyboard focus (where the control
accepts focus) — and every colour SHALL come from the theme tokens. Controls
already tokenised SHALL keep their existing single definition.

#### Scenario: Each remaining control is defined once [ds_remaining_single_definition]

- **WHEN** the global stylesheet is searched for a remaining control's rules
- **THEN** each control's states are declared in one place, scoped by type or
  object name, not per call site

#### Scenario: Remaining controls resolve through tokens [ds_remaining_states]

- **WHEN** a group box, radio button, progress bar, header view, or splitter is
  shown at rest, hovered, pressed, checked, disabled, or focused
- **THEN** its state styling resolves to theme tokens and no state falls back to
  an untokenised literal

### Requirement: Custom-painted widgets read tokens

A widget that paints itself SHALL derive every theme-concern colour from the
theme tokens (directly, or through the widget palette the token layer applies)
and SHALL NOT hard-code a theme colour in its paint code. Colours that are data
MAY keep a dynamic value. The custom-painted widgets that still hard-code theme
greys — the curve editor, the spectrum bar, the histogram view, and the
navigator thumbnail — SHALL be converted; widgets that already derive from the
palette are unchanged.

#### Scenario: Custom-painted widgets follow the theme [ds_painted_tokens]

- **WHEN** the curve editor, spectrum bar, histogram view, and navigator
  thumbnail repaint after a theme change
- **THEN** every theme-concern colour they draw equals a value from the token
  accessor, and no hard-coded theme grey remains

#### Scenario: Data colours stay data [ds_painted_data]

- **WHEN** a custom-painted widget draws a colour that is document or user data
- **THEN** it keeps that dynamic value and only its frame or background comes
  from the tokens

### Requirement: Theme-like colour literals are rejected

The repository SHALL provide a check that fails when a widget source outside the
theme layer introduces a theme-like grey colour literal — a hard-coded grey
surface, text, border, or control colour. The check SHALL ship with a baseline
allow-list capturing the pre-existing grep hits so it passes on landing: genuine
data-colour sites (the canvas backdrop, marching-ants pairs, greyscale
gradients, the foreground/background tool swatches, liquify and guide presets)
and the theme-like literals still pending conversion, each recorded with a
reason. The list only shrinks as conversion proceeds; a new literal not on it
fails.

#### Scenario: The guard rejects a new theme literal [ds_literal_guard_rejects]

- **WHEN** a source file outside the theme layer introduces a theme-like grey
  literal that is not allow-listed
- **THEN** the check fails and names the offending file

#### Scenario: Allow-listed data colours pass [ds_literal_guard_allows]

- **WHEN** an allow-listed data-colour site keeps its literal
- **THEN** the check passes

#### Scenario: The baseline is green on landing [ds_literal_guard_baseline]

- **WHEN** the check runs over the tree as landed
- **THEN** it passes, because every pre-existing hit is either converted or
  recorded on the baseline allow-list, and the list may only be pruned

## MODIFIED Requirements

### Requirement: Theme token contract

The theme SHALL expose a public accessor that returns the named tokens of a
brightness level: the colour tokens the stylesheet substitutes, including every
derived shade, and the numeric metrics (panel border width, chrome border width,
group divider width, tool-slot footprint and icon maxima, shade step). The
global stylesheet builder SHALL consume the same accessor, so a token value
appears exactly once in the theme and the stylesheet, the dump, and the widgets
cannot disagree. A token consumed only by a custom-painted widget SHALL still be
a token (so it appears in the dump and is shared with paint code) even though
the global stylesheet never references it. Clamping a level SHALL use the
existing level range.

#### Scenario: Tokens cover colours and metrics [ds_tokens_shape]

- **WHEN** the token accessor is asked for any valid brightness level
- **THEN** it returns the named colour tokens (including the derived ones the
  stylesheet uses, such as the alternate table background, menu disabled text,
  panel-header border, and scrollbar thumb hover) and the numeric metrics,
  each exactly once

#### Scenario: The stylesheet and the dump agree [ds_tokens_single_source]

- **WHEN** the stylesheet for a level is built and the same level is dumped
- **THEN** every colour token the stylesheet substitutes appears in the
  stylesheet with the same value, the stylesheet substitutes no colour literal
  outside the token set, and a token consumed only by a custom-painted widget is
  exempt from the stylesheet-substitution check
