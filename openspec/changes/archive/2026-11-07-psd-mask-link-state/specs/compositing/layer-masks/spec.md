# Spec Delta

## REMOVED Requirements

### Requirement: Mask link flag

**Reason**: This requirement encodes the reversed PSD bit polarity, including
scenario titles that describe linking as setting bit 0 and unlinking as
clearing it. Replace the incorrect contract explicitly rather than retaining
those contradictory scenario titles.

**Migration**: Use `Raster mask link flag polarity` below. Keep the native
PSD flags byte; change the runtime interpretation and newly created defaults.
Do not automatically rewrite old files or infer their intended link state.

## ADDED Requirements

### Requirement: Raster mask link flag polarity

The system SHALL interpret bit 0 of a raster layer mask's PSD flags byte as
unlinked when set and linked when clear. Linking SHALL clear only bit 0;
unlinking SHALL set only bit 0, preserving every other flags bit and the mask's
bounds and coverage. Setting the state it already has SHALL report no change
and leave the document unchanged. A newly created raster mask SHALL be linked
with bit 0 clear, including masks created by vector-mask rasterization and
Auto Blend. The raw flags byte SHALL retain its PSD representation through
save and read; linking SHALL NOT reinterpret the mask's stored coordinates.

#### Scenario: Linking clears only bit 0 [pml_link_clears_bit]

- **WHEN** an unlinked mask with flags `0xA1` is linked
- **THEN** its flags become `0xA0`, it reads as linked, and its bounds and coverage are unchanged

#### Scenario: Unlinking sets only bit 0 [pml_unlink_sets_bit]

- **WHEN** a linked mask with flags `0xA0` is unlinked
- **THEN** its flags become `0xA1`, it reads as unlinked, and its bounds and coverage are unchanged

#### Scenario: A new mask is linked [pml_new_mask_linked]

- **WHEN** a raster mask is created by an ordinary mask command, vector-mask rasterization, or Auto Blend
- **THEN** the created mask reads as linked and its flags have bit 0 clear

#### Scenario: Repeating the linked state is a no-op [pml_link_noop]

- **WHEN** a mask is set to the linked or unlinked state it already has
- **THEN** the operation reports no change and leaves the document unchanged
