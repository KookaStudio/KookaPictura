# Magnetic Pen

## Purpose

Trace image edges as editable paths using the Freeform Pen's Magnetic option.

## Requirements

### Requirement: Magnetic Pen options

The Freeform Pen SHALL offer a Magnetic checkbox and controls for Width (1–256 px), Contrast (1–100%), Frequency (0–100), and Pen Pressure, defaulting to 10 px, 20%, 60, and off respectively. Brackets SHALL adjust Width by one pixel within its range. Pressure SHALL reduce the detection width when enabled and tablet pressure is available.

#### Scenario: Controls and width shortcuts [magpen_options]
- **WHEN** Magnetic is enabled and Width is adjusted with the controls and brackets
- **THEN** the values remain in range and the width changes by one pixel per bracket

#### Scenario: Tablet pressure [magpen_pressure]
- **WHEN** Pen Pressure is enabled and the stylus pressure increases during tracing
- **THEN** detection width decreases; without tablet pressure it uses the configured width

### Requirement: Edge-following path session

The first click SHALL begin a magnetic trace. Pointer movement SHALL preview a trace following image edges within Width and Contrast, with automatic fastening controlled by Frequency; clicks SHALL add fastening points that remain editable anchors in the committed path. Delete SHALL remove the latest fastening point and its segment. Escape or Undo during a trace SHALL cancel it without changing document history.

#### Scenario: Trace and remove fastening [magpen_trace]
- **WHEN** the user traces a high-contrast edge, clicks to fasten, and presses Delete
- **THEN** the trace follows the edge and the last fastened segment is removed; remaining manual fastenings become anchors

#### Scenario: Automatic fastening [magpen_frequency]
- **WHEN** the same edge is traced with Frequency zero and then high
- **THEN** high Frequency creates automatic editable fastening anchors while zero creates none

#### Scenario: Document depth [magpen_depth]
- **WHEN** Magnetic Pen and Magnetic Lasso are used on a 32-bit document
- **THEN** Magnetic Pen traces and Magnetic Lasso remains unavailable

#### Scenario: Switching context [magpen_switch]
- **WHEN** the user switches tools or documents during a trace
- **THEN** its transient field and preview are released and the next document can start a trace

#### Scenario: Cancel a trace [magpen_cancel]
- **WHEN** the user cancels a live trace using Escape or Undo
- **THEN** the preview disappears and the path and history remain unchanged

### Requirement: Complete magnetic paths

Enter SHALL finish an open path. Double-click SHALL close using a magnetic segment; Alt-double-click SHALL close using a straight segment. Alt-drag SHALL add freehand segments and Alt-click SHALL add straight segments. The finished path SHALL use the selected Shape, Path, or Pixels output mode and record a single logical edit. Shape and Pixels output SHALL use the entire active Work Path, consistent with the Pen tool. When a saved path or vector mask is active, the trace SHALL edit that path without outputting the Work Path. Pixels output SHALL support Quick Mask.

#### Scenario: Open path completion [magpen_open]
- **WHEN** the user traces and presses Enter in Path mode
- **THEN** the resulting saved path is open and follows the preview

#### Scenario: Magnetic and straight closing [magpen_close]
- **WHEN** the user completes comparable traces with double-click and Alt-double-click
- **THEN** both paths close and only the first closing segment follows edges

#### Scenario: Temporary freehand and straight segments [magpen_alt]
- **WHEN** the user Alt-drags and Alt-clicks while tracing
- **THEN** the respective segments follow the drag and connect by a straight line

#### Scenario: Output modes [magpen_modes]
- **WHEN** the user completes a trace in Shape, Path, and Pixels modes
- **THEN** each mode lands the corresponding shape, saved path, or painted stroke as one logical edit

#### Scenario: Pixels output in Quick Mask [magpen_quick_mask]
- **WHEN** the user completes a magnetic trace in Pixels mode while Quick Mask is active
- **THEN** it paints the mask as one edit and Undo restores the mask and prior path

#### Scenario: Active saved path or vector mask [magpen_active_paths]
- **WHEN** the user completes a magnetic trace with a saved path or vector mask active in Shape or Pixels mode
- **THEN** the active path gains the trace as one edit without producing shape or pixel output from the Work Path
