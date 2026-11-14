# Spec Delta

## RENAMED Requirements

- FROM: `### Requirement: Source-mode documents save from three color planes`
- TO: `### Requirement: Save trims the display alpha to three color planes`

## MODIFIED Requirements

### Requirement: Save trims the display alpha to three color planes

`save_view(doc)` SHALL return the document with its composite trimmed to three
color planes when the document has an RGB working mode, keeps a four-plane
(RGBA display) composite, and either carries a source mode or carries one or
more document-level extra channels. The trim SHALL apply at every output bit
depth. It SHALL borrow any other document unchanged. The application's PSD/PSB
save SHALL write through it.

**NOTE:** While the document carries extra channels, the written merged image
has no separate transparency plane; the PSD negative-layer-count convention for
merged transparency is not modeled.

#### Scenario: An opened or converted CMYK document re-saves as CMYK

- **WHEN** a CMYK document whose composite the app stored as RGBA is saved
- **THEN** the written file is header mode CMYK, not RGB with an alpha channel

#### Scenario: A four-channel RGB source keeps its channel count [dc_save_alpha_count]

- **WHEN** an RGB document whose source carried three color planes and one extra channel is saved after its composite became four-plane RGBA
- **THEN** the output header declares four channels: three color planes followed by the extra channel

#### Scenario: The extra channel is not duplicated [dc_save_alpha_unique]

- **WHEN** the display composite alpha plane and the extra channel hold different bytes and the document is saved
- **THEN** the written planes are the three color planes and the extra channel, with no duplicated alpha plane

#### Scenario: A depth-16 extra channel keeps its count and depth [dc_save_alpha_depth16]

- **WHEN** a depth-16 RGB document with a four-plane composite, retained native planes, and one extra channel is saved
- **THEN** the output header declares four channels at depth 16, the extra plane is retained, and no alpha plane is duplicated
