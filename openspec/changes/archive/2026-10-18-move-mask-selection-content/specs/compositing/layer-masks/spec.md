# Spec Delta

## MODIFIED Requirements

### Requirement: Foreground and background follow the edit target

The application SHALL keep one foreground/background pair for editing layer
pixels and one for editing a mask. When the active edit target moves from a
layer's pixels to its mask, the colour state SHALL switch to the mask pair,
initially white foreground and black background as CS6 uses while a mask is
active; when it moves back, the colour state SHALL switch to the image pair.
Each pair SHALL keep the colours last set while its target was active. Resetting
the colours (`D` or the toolbox reset) SHALL restore the active target's
defaults: white foreground / black background for a mask, black foreground /
white background for layer pixels.

#### Scenario: Toggling the thumbnails swaps the pairs

- **WHEN** the image colours are set, the mask thumbnail is clicked, the mask foreground is changed, and the layer thumbnail is clicked
- **THEN** the mask shows white/black on activation, the image pair returns on the layer click, and clicking the mask again shows the changed mask foreground

#### Scenario: Resetting colours follows the edit target [lpr_mask_reset_colors]

- **WHEN** a mask is the edit target and `D` or the toolbox reset is used
- **THEN** the mask pair becomes white foreground / black background, and switching back to the layer restores the image pair
