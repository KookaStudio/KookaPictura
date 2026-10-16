# Spec Delta

## MODIFIED Requirements

### Requirement: Filter-kind mapping and defaults

The command SHALL map each recognized `kind` to a concrete `pictura_filters::Filter` with fixed defaults. The mapping SHALL include at least: `gaussian-blur` → `GaussianBlur { radius: 5.0 }`, `box-blur` → `BoxBlur { radius: 3 }`, `motion-blur` → `MotionBlur { angle: 0.0, distance: 15 }`, `median` → `Median { radius: 2 }`, `despeckle` → `Despeckle`, `sharpen` → `Sharpen`, `sharpen-more` → `SharpenMore`, `unsharp-mask` → `UnsharpMask { amount: 150.0, radius: 1.0, threshold: 0 }`, `add-noise` → `AddNoise { amount: 25.0, distribution: Uniform, monochromatic: false, seed: 1 }`, `dust-and-scratches` → `DustAndScratches { radius: 1, threshold: 0 }`, `extrude` → `Extrude { kind: Blocks, size: 30, depth: 30.0, level_based: true, solid_front: false, mask_incomplete: false }`, `tiles` → `Tiles { count: 10, offset: 10, fill: BackgroundColor, foreground: [0, 0, 0], background: [255, 255, 255] }`, `trace-contour` → `TraceContour { level: 128, edge: Lower }`, `wind` → `Wind { method: Wind, from_right: true }`, and `smart-sharpen` → `SmartSharpen { amount: 100.0, radius: 1.0, reduce_noise: 0.0, remove: GaussianBlur, angle: 0.0 }`. The complete set of recognised kinds, with their slot order, is the `FILTER_ARITIES` table in `filter_map.rs`. An unrecognized `kind` SHALL map to no filter and the command MUST return false without changing the document.

The command SHALL additionally accept an ordered list of floating-point parameter values for a recognized `kind`, building the same `pictura_filters::Filter` with those values in the kind's fixed slot order. The existing default mapping SHALL be equivalent to supplying the documented defaults for that kind. A parameter list whose length does not match the kind's slot count MUST map to no filter, and the command MUST return false without changing the document.

#### Scenario: Each known kind maps to its default filter

- **WHEN** the command is called with each supported kind and no parameters
- **THEN** the corresponding `Filter` with the listed defaults is applied

#### Scenario: A parameter list overrides the defaults

- **WHEN** the command is called with `gaussian-blur` and a slot list of one value
- **THEN** `GaussianBlur` is built with that radius instead of the default `5.0`

#### Scenario: A wrong-arity parameter list is refused

- **WHEN** the command is called with a known kind and a parameter list of the wrong length
- **THEN** it returns false and no layer is modified

#### Scenario: Unknown kind is refused

- **WHEN** the command is called with an unrecognized kind
- **THEN** it returns false and no layer is modified

## ADDED Requirements

### Requirement: Filter parameter dialog

The Qt `Filter` menu SHALL open a parameter dialog for every recognized kind that takes parameters. The dialog MUST show a preview thumbnail of the filtered result with its own zoom controls (magnifier icons and a percentage readout), a Preview checkbox, and one control per parameter of the kind. The thumbnail SHALL show the document section the canvas is currently viewing at that zoom, seeded from the canvas zoom, rather than the whole image scaled down. Radial Blur MUST omit the thumbnail and show a draggable Blur Center instead, and Lens Flare MUST show a whole-image placement control for its center. Radial Blur's Blur Center is a display-only control: the engine's radial blur has no center parameter, so it records no engine slot. Kinds with no parameters (Average, Blur, Blur More, Sharpen) MUST apply without opening a dialog.

#### Scenario: A parameterised filter opens its dialog

- **WHEN** the user chooses `Filter > Blur > Gaussian Blur…`
- **THEN** a dialog opens with a Radius control and a preview thumbnail

#### Scenario: The preview zoom is independent of the document

- **WHEN** the user clicks the dialog's zoom-in or zoom-out control
- **THEN** the percentage readout steps and only the thumbnail scales; the document pixels and the document zoom are unchanged

#### Scenario: The preview shows the current canvas section

- **WHEN** a dialog opens while the canvas is viewing a particular section at a particular zoom
- **THEN** the thumbnail shows that document section at the seeded zoom, not the whole image shrunk to fit

#### Scenario: Radial Blur shows the Blur Center instead of a thumbnail

- **WHEN** the user opens `Filter > Blur > Radial Blur…`
- **THEN** the dialog shows Amount, Blur Method, Quality, and a draggable Blur Center, and no preview thumbnail

#### Scenario: A parameterless filter applies directly

- **WHEN** the user chooses `Filter > Sharpen > Sharpen`
- **THEN** the filter applies immediately with no dialog

### Requirement: Live non-committing filter preview

While a filter dialog is open, a parameter change SHALL render the filtered result on the canvas without recording a history state. Each update MUST be computed from the pre-filter pixels, not from the previous preview. A preview SHALL be bounded to the visible document section expanded by the filter's support, so previewing a large document costs at most a viewport; choosing OK MUST commit exactly one history state for the filter by filtering the whole layer; choosing Cancel MUST restore the pre-filter pixels bit-identically and record no history state.

#### Scenario: Preview does not record history

- **WHEN** a parameter is changed in the dialog
- **THEN** the canvas shows the filtered result and the history list is unchanged

#### Scenario: The live preview is bounded to the visible section

- **WHEN** a parameter changes on a large document
- **THEN** only the visible section plus the filter's support apron is filtered, and the commit still filters the whole layer

#### Scenario: OK commits one state

- **WHEN** the dialog is accepted
- **THEN** exactly one filter history state is recorded and the canvas keeps the result

#### Scenario: Cancel restores the pre-filter pixels

- **WHEN** the dialog is cancelled after previewing
- **THEN** the pre-filter pixels are restored bit-identically and no history state was added

### Requirement: Filter menu wiring

Every `Filter`-menu entry whose kind has an engine kernel MUST be enabled and MUST dispatch its parameter dialog, or apply directly when it has no parameters. A dialog-opening entry's menu label MUST end with an ellipsis (`…`); a parameterless entry MUST not. Entries whose kind has no engine kernel MUST remain disabled. `Custom` requires a caller-supplied 5×5 kernel and therefore has no fixed default; it stays a disabled stub. A filter command MUST be disabled when the active layer is not an unlocked normal pixel layer.

#### Scenario: An implemented entry is enabled and dispatches

- **WHEN** a document has an unlocked normal pixel layer active and the user opens the Filter menu
- **THEN** implemented entries are enabled and choosing one runs its filter or dialog

#### Scenario: Dialog entries carry an ellipsis

- **WHEN** the Filter menu is shown
- **THEN** every entry that opens a parameter dialog ends with `…`, and every parameterless entry does not

#### Scenario: An unimplemented entry stays disabled

- **WHEN** the user opens the Filter menu
- **THEN** entries with no engine kernel, such as Filter Gallery and Reduce Noise, are disabled

#### Scenario: No pixel layer disables the filters

- **WHEN** the active layer is missing or is not an unlocked normal pixel layer
- **THEN** the filter commands are disabled

### Requirement: Last Filter and Last Filter Settings

The `Filter` menu SHALL provide `Last Filter` (`Ctrl+F`) and `Last Filter Settings` (`Alt+Ctrl+F`). After a filter is committed, `Last Filter` MUST re-apply the same kind and parameter values with no dialog, and `Last Filter Settings` MUST reopen that filter's dialog prefilled with the last values. Both commands MUST remain disabled until a filter has been committed, and the `Last Filter` label MUST name the last filter.

#### Scenario: Last Filter re-applies without a dialog

- **WHEN** Gaussian Blur has been committed and the user invokes `Last Filter`
- **THEN** Gaussian Blur applies with the same radius and no dialog opens

#### Scenario: Last Filter Settings reopens prefilled

- **WHEN** the user invokes `Last Filter Settings` after a filter
- **THEN** that filter's dialog opens with the previous parameter values

#### Scenario: Both are disabled before any filter runs

- **WHEN** no filter has been committed in the session
- **THEN** `Last Filter` and `Last Filter Settings` are disabled
