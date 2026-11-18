# tools/dodge-burn-sponge-options Specification

## Purpose
The shared options of the Dodge, Burn, and Sponge tools: Dodge/Burn Airbrush
build-up, the painting-mode quick keys, Grayscale tonal behavior, and the
32-bpc availability gate.

## Requirements

### Requirement: Dodge/Burn Airbrush build-up

The Dodge and Burn options bars SHALL carry an Airbrush toggle, off by default,
and `Shift+Alt+P` SHALL toggle it for the active Dodge or Burn tool. While a
Dodge or Burn stroke is open with Airbrush on, the tool SHALL keep applying its
effect at the pointer at a bounded time-driven rate even when the pointer is
still, so a held button builds the effect up. With Airbrush off, a still pointer
SHALL add nothing after the initial dab. Held dabs SHALL stop when the stroke
ends, whether by release, tool switch, or session cancel, and SHALL NOT add a
second history state.

#### Scenario: Airbrush off adds nothing while held [tone_airbrush_off]

- **WHEN** a Dodge stroke with Airbrush off is held still
- **THEN** the pixels under the pointer stop changing after the initial dab

#### Scenario: Airbrush on builds while held [tone_airbrush_on]

- **WHEN** a Dodge stroke with Airbrush on is held still and then released
- **THEN** the pixels under the pointer keep lightening and release records one history state

#### Scenario: The Airbrush shortcut follows the active tone tool [tone_airbrush_shortcut]

- **WHEN** `Shift+Alt+P` is pressed with Dodge or Burn active
- **THEN** that tool's Airbrush flag flips and its options-bar toggle follows

### Requirement: Tone painting-mode quick keys

While Dodge or Burn is active, `Shift+Alt+S`, `Shift+Alt+M`, and `Shift+Alt+H`
SHALL select Shadows, Midtones, and Highlights. While Sponge is active,
`Shift+Alt+S` and `Shift+Alt+D` SHALL select Saturate and Desaturate. The
options bar SHALL follow the model when a quick key changes it, and a tone quick
key pressed with another tool active SHALL change nothing.

#### Scenario: Dodge/Burn range quick keys [tone_range_quick_keys]

- **WHEN** `Shift+Alt+S`, `Shift+Alt+M`, or `Shift+Alt+H` is pressed with Dodge or Burn active
- **THEN** the tool's Range is Shadows, Midtones, or Highlights and the options bar shows it

#### Scenario: Sponge mode quick keys [tone_sponge_quick_keys]

- **WHEN** `Shift+Alt+S` or `Shift+Alt+D` is pressed with Sponge active
- **THEN** the Sponge Mode is Saturate or Desaturate and the options bar shows it

#### Scenario: A quick key does not cross tools [tone_quick_key_ignored]

- **WHEN** a tone quick key is pressed with a tool that is not Dodge, Burn, or Sponge active
- **THEN** no tone option changes

### Requirement: Grayscale tonal behavior

In a Grayscale document, Dodge and Burn SHALL move the single grey channel with
the same Range-weighted Exposure model, and Sponge SHALL move grey levels away
from (Saturate) or toward (Desaturate) middle grey, increasing or decreasing
contrast. The operator SHALL NOT treat the missing colour planes as black
channels or introduce a colour cast.

#### Scenario: Grayscale Sponge changes contrast [tone_grayscale_sponge]

- **WHEN** the Sponge drags across a Grayscale document in Saturate mode
- **THEN** dark greys get darker and light greys get lighter, moving away from middle grey

#### Scenario: Grayscale Dodge and Burn move the grey plane [tone_grayscale_dodge_burn]

- **WHEN** Dodge and Burn drag across a Grayscale document
- **THEN** the grey value rises for Dodge and falls for Burn without a colour cast

### Requirement: 32-bpc tone tool availability

The Dodge, Burn, and Sponge tools SHALL be unavailable on 32-bpc documents. The
toolbox flyout rows SHALL be disabled, `setActiveTool` SHALL refuse them, and an
active tone tool SHALL switch to Move when a document switch or depth conversion
makes it unavailable. The engine SHALL refuse to start a tone stroke on a
32-bpc document even when called directly. The 32-bpc gate SHALL be independent
of the engine's existing 8-bit tone path; 8-bpc documents SHALL keep the tools
available.

#### Scenario: A 32-bpc document disables the tone tools [tone_32bpc_disabled]

- **WHEN** the active document is 32 bpc
- **THEN** the Dodge, Burn, and Sponge flyout rows are disabled and activating one leaves Move active

#### Scenario: A tone tool falls back to Move [tone_32bpc_fallback]

- **WHEN** Dodge, Burn, or Sponge is active and the document becomes 32 bpc
- **THEN** the active tool becomes Move

#### Scenario: The engine refuses a 32-bpc tone stroke [tone_32bpc_refused]

- **WHEN** a tone stroke is begun on a 32-bpc document
- **THEN** it is refused and no history state is recorded
