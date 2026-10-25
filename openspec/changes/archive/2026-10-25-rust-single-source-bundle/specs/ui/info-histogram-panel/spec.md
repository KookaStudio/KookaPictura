# Spec Delta

## MODIFIED Requirements

### Requirement: CMYK colour readout

The Info panel SHALL show the CMYK components of the colour under the cursor in
the top-right block by default. The components SHALL be derived from the sampled
pixel through the shared CMYK working-profile conversion, with no document
call beyond the sample. Each component SHALL be reported as a 0–100 percentage.
Off-canvas positions SHALL blank the colour blocks.

#### Scenario: Cursor over a white pixel [info_cmyk_white]

- **WHEN** the cursor is over a white pixel of a document
- **THEN** the top-right block shows `C : 0  M : 0  Y : 0  K : 0`

#### Scenario: Cursor leaves the canvas [info_cmyk_offcanvas]

- **WHEN** the cursor moves off the canvas
- **THEN** the colour blocks are blank rather than holding their last values

#### Scenario: CMYK follows the working profile [info_cmyk_profile]

- **WHEN** the cursor is over a black pixel and the block shows CMYK
- **THEN** the components equal the shared conversion's CMYK for black, not
  `C : 0  M : 0  Y : 0  K : 100`
