## ADDED Requirements

### Requirement: 16-bpc tone writes the retained native samples

On a 16-bpc RGB or Grayscale document whose target layer retains native samples
that the compositor reads, a Dodge, Burn, or Sponge stroke SHALL read and write
the layer's retained native store at 16 bits, not only its 8-bit working mirror.
After the stroke, the layer's 8-bit colour channel SHALL be the narrowing of the
native plane, so the working copy and the retained store agree. A Grayscale
document SHALL use its single native colour plane and SHALL NOT write a colour
plane it does not have.

The tone result SHALL be computed at 16 bits: a stroke on a native sample that
is not 8-bit aligned SHALL NOT equal the 8-bit widening of the same stroke on
the narrowed sample. Selection coverage SHALL be applied at the native depth.
Cancelling the stroke SHALL restore the native samples it changed.

#### Scenario: 16-bpc Dodge lightens the native samples [tone16_dodge]

- **WHEN** a Dodge stroke runs on a 16-bpc RGB layer whose native red sample is not 8-bit aligned
- **THEN** the native red sample rises and the layer's 8-bit red channel is its narrowing

#### Scenario: 16-bpc Burn darkens the native samples [tone16_burn]

- **WHEN** a Burn stroke runs on a 16-bpc RGB layer
- **THEN** the native samples fall and the 8-bit channels are their narrowing

#### Scenario: 16-bpc Sponge moves native saturation [tone16_sponge]

- **WHEN** a Sponge Saturate stroke runs on a 16-bpc RGB layer
- **THEN** the native colour spread rises and the 8-bit channels are its narrowing

#### Scenario: 16-bpc Grayscale tone uses the single plane [tone16_gray]

- **WHEN** Dodge, Burn, or Sponge runs on a 16-bpc Grayscale layer
- **THEN** the single native grey plane moves and no colour plane is written

#### Scenario: The native result is not the 8-bit widening [tone16_precision]

- **WHEN** the same stroke runs on a 16-bpc layer and on an 8-bit layer seeded with the narrowed samples
- **THEN** the 16-bpc native result differs from the 8-bit result widened to 16 bits

#### Scenario: Cancelling restores the native samples [tone16_cancel]

- **WHEN** a 16-bpc tone stroke is cancelled after painting
- **THEN** the layer's native samples are the ones the stroke began on

### Requirement: 32-bpc tone stays unavailable

A tone stroke SHALL be refused on a 32-bpc document, including one whose 32-bit
depth is a retained source store on an 8-bit working document. The refusal SHALL
apply even when `Stroke::begin_tone` is called directly, and no stroke or
history state SHALL be created.

#### Scenario: A retained 32-bit store is refused [tone16_32bpc_refused]

- **WHEN** a tone stroke is begun on a document that retains a 32-bit source store
- **THEN** it returns `UnsupportedDepth` and changes no pixel

### Requirement: The 8-bit tone path is unchanged

On an 8-bit document, and on a 16-bpc document with no authoritative native
store, the tone tools SHALL keep the existing 8-bit working path and produce
byte-identical results to before this change.

#### Scenario: The 8-bit results are unmoved [tone16_8bit_unchanged]

- **WHEN** the existing tone engine tests run
- **THEN** they pass with no change to the expected bytes
