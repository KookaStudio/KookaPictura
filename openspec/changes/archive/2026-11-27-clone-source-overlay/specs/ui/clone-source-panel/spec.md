## MODIFIED Requirements

### Requirement: Clone Source panel

`Window > Panels > Clone Source` SHALL show or hide a Clone Source panel with five source slots, each keeping its own Alt-clicked source, measured offset, and transform; Clone Stamp and Healing Brush SHALL use the active slot. The panel SHALL show the offset as destination minus source (editable once the slot has a source), W and H (with Maintain Aspect Ratio on by default), Rotate, Flip Horizontal, Flip Vertical, and Reset Transform (100 % / 100 % / 0°, no flips). The frame controls SHALL remain shown disabled as not implemented.

#### Scenario: Slots keep their own sources

- **WHEN** the user Alt-clicks in slot 2 and strokes in empty slot 1
- **THEN** the stroke is refused without a history state

#### Scenario: Flip and offset through the panel

- **WHEN** Flip Horizontal is pressed and a stroke starts 35 px right of the source
- **THEN** the cloned pixels mirror about the stroke start and Offset X reads 35

## ADDED Requirements

### Requirement: Clone source overlay controls

The panel SHALL offer Show Overlay, Auto Hide, Clipped, Opacity from 0 to 100%, Normal/Darken/Lighten/Difference mode, and Invert. Each slot SHALL retain its own overlay settings for the session, initially off for all checkboxes, 100% opacity and Normal mode. Changes SHALL update the preview without changing document pixels or history.

#### Scenario: Overlay controls and slot settings

- **WHEN** the user changes each overlay control, switches slots and returns
- **THEN** the controls show the first slot's retained values and the document and history are unchanged

### Requirement: Clone source overlay preview

When Show Overlay is enabled, Clone Stamp and Healing Brush SHALL preview the active valid source at the pointer on the canvas using the same source mapping, offset and transform as their sampling. A missing or invalid source, an inactive tool, or a pointer outside the canvas SHALL show no preview. The preview SHALL not become document pixels. Source transparency SHALL be respected. Before the initial stroke establishes an offset, the source anchor SHALL align with the pointer; thereafter the aligned setting SHALL determine mapping as it does for painting.

#### Scenario: Preview follows each tool and source mapping

- **WHEN** a source is sampled and the pointer moves with Clone Stamp or Healing Brush, including after an aligned stroke and with a source transform
- **THEN** the preview follows the corresponding sampled pixels and changes neither pixels nor history

#### Scenario: Non-aligned mapping across strokes

- **WHEN** the user switches alignment and starts a subsequent non-aligned stroke with Clone Stamp or Healing Brush
- **THEN** the preview reanchors at the source before the stroke and follows that stroke’s mapping while painting, rather than using a previous offset

#### Scenario: Healing Brush samples a transformed source

- **WHEN** the active slot has a source transform and the user paints with Healing Brush
- **THEN** the healing operation samples through that transform and the preview uses the same mapping

#### Scenario: Missing source and tool changes

- **WHEN** the active slot lacks a valid source, the pointer leaves the canvas, or a different tool is active
- **THEN** no source preview is drawn

#### Scenario: Auto Hide during painting

- **WHEN** Auto Hide is enabled and a stroke is pressed then released or cancelled
- **THEN** the preview is hidden during the stroke and returns afterwards while otherwise eligible

#### Scenario: Clip to the brush footprint

- **WHEN** Clipped is enabled with a small brush
- **THEN** the preview appears only within the current brush footprint; disabling Clipped restores the visible source area

### Requirement: Overlay appearance

The preview SHALL invert source RGB before blending when Invert is on, preserving source alpha. It SHALL use standard channel formulas for Normal, Darken, Lighten and Difference and composite the result at the selected opacity. Opacity zero SHALL hide the preview without discarding settings. These formulas are an approximation, not verified CS6 parity.

#### Scenario: Opacity and blend modes

- **WHEN** each blend mode and opacity including zero is applied to known source and destination colors
- **THEN** the preview reflects the specified mode and opacity and zero opacity leaves the canvas appearance unchanged

#### Scenario: Transparent source appearance

- **WHEN** the source contains fully transparent and partially transparent pixels, including with Invert and reduced opacity
- **THEN** the preview preserves source alpha, showing the backdrop through transparent pixels and compositing partial coverage correctly

#### Scenario: Gray alignment preview

- **WHEN** identical opaque source and destination pixels are shown with Normal, 50% opacity, Invert on, and Clipped off
- **THEN** they appear neutral gray within rounding tolerance
