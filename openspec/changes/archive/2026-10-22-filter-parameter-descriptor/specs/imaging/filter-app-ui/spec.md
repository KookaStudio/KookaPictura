# Spec Delta

## MODIFIED Requirements

### Requirement: Filter-kind mapping and defaults

The command SHALL map each recognized `kind` to a concrete `pictura_filters::Filter` with fixed defaults. The mapping SHALL include at least: `gaussian-blur` → `GaussianBlur { radius: 5.0 }`, `box-blur` → `BoxBlur { radius: 3 }`, `motion-blur` → `MotionBlur { angle: 0.0, distance: 15 }`, `median` → `Median { radius: 2 }`, `despeckle` → `Despeckle`, `sharpen` → `Sharpen`, `sharpen-more` → `SharpenMore`, `unsharp-mask` → `UnsharpMask { amount: 150.0, radius: 1.0, threshold: 0 }`, `add-noise` → `AddNoise { amount: 25.0, distribution: Uniform, monochromatic: false, seed: 1 }`, `dust-and-scratches` → `DustAndScratches { radius: 1, threshold: 0 }`, `extrude` → `Extrude { kind: Blocks, size: 30, depth: 30.0, level_based: true, solid_front: false, mask_incomplete: false }`, `tiles` → `Tiles { count: 10, offset: 10, fill: BackgroundColor, foreground: [0, 0, 0], background: [255, 255, 255] }`, `trace-contour` → `TraceContour { level: 128, edge: Lower }`, `wind` → `Wind { method: Wind, from_right: true }`, and `smart-sharpen` → `SmartSharpen { amount: 100.0, radius: 1.0, reduce_noise: 0.0, remove: GaussianBlur, angle: 0.0 }`. The complete set of recognised kinds, with their canonical slot order, is the filter parameter descriptor; a kind's canonical slot count is the sum of its descriptor's control slot counts. An unrecognized `kind` SHALL map to no filter and the command MUST return false without changing the document.

The command SHALL additionally accept an ordered list of floating-point parameter values for a recognized `kind`, building the same `pictura_filters::Filter` with those values in the kind's canonical slot order. The existing default mapping SHALL be equivalent to supplying the documented defaults for that kind. A kind with a fixed slot list MUST refuse a non-empty list whose length differs from its canonical slot count. The kinds with a documented variable list — `lighting-effects` (9 rig slots plus 13 per light, 1 to 16 lights) and the Blur Gallery kinds (a pin, path or spin count followed by fixed-stride records, then the shared 13-slot effects tail; `blur-gallery` additionally carries its per-effect counts, motion section and flash duration) — MUST accept exactly the lists those kinds document and refuse any other non-empty list. Every kind accepts the canonical-length list built from its descriptor's defaults.

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

#### Scenario: The canonical slot count is the descriptor's [fpd_arity]

- **WHEN** a recognised kind is called with its descriptor's default vector, then with one more zero and — when that count is greater than one — one fewer
- **THEN** the default vector maps to a filter, and the longer and shorter lists are refused without changing the document

#### Scenario: The variable-list kinds accept their documented lists [fpd_variable_lists]

- **WHEN** `lighting-effects` is mapped with a rig of 1 to 16 lights, and each Blur Gallery kind with one of its documented pin, path or spin lists (or, for `blur-gallery`, its combined list)
- **THEN** each maps to a filter, and a non-empty list that does not decode as one of those shapes is refused

### Requirement: Filter parameter dialog

The Qt `Filter` menu SHALL open a parameter dialog for every recognized kind that takes parameters. The dialog MUST show a preview thumbnail of the filtered result with its own zoom controls (magnifier icons and a percentage readout), a Preview checkbox, and one control per parameter of the kind. The thumbnail SHALL show the document section the canvas is currently viewing at that zoom, seeded from the canvas zoom, rather than the whole image scaled down. Radial Blur MUST omit the thumbnail and show a draggable Blur Center instead; the centre is two engine slots (x, y, normalized 0..1) that move the blur origin. Lens Flare MUST show a whole-image placement control for its center. Kinds with no parameters (Average, Blur, Blur More, Sharpen) MUST apply without opening a dialog.

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
- **THEN** the dialog shows Amount, Blur Method, Quality, and a draggable Blur Center, and no preview thumbnail; the centre's x and y are slots 3 and 4 of the `radial-blur` list

#### Scenario: A parameterless filter applies directly

- **WHEN** the user chooses `Filter > Sharpen > Sharpen`
- **THEN** the filter applies immediately with no dialog

### Requirement: Filter menu wiring

Every `Filter`-menu entry whose kind has a descriptor MUST be enabled and MUST dispatch its parameter dialog, or apply directly when it has no parameters. A dialog-opening entry's menu label MUST end with an ellipsis (`…`); a parameterless entry MUST not. Entries whose kind has no engine kernel MUST remain disabled. `Custom` is a normal parameterized kind and opens its dialog on the pass-through kernel. A filter command MUST be disabled when the active layer is not an unlocked normal pixel layer.

#### Scenario: An implemented entry is enabled and dispatches

- **WHEN** a document has an unlocked normal pixel layer active and the user opens the Filter menu
- **THEN** implemented entries are enabled and choosing one runs its filter or dialog

#### Scenario: Dialog entries carry an ellipsis

- **WHEN** the Filter menu is shown
- **THEN** every entry that opens a parameter dialog ends with `…`, and every parameterless entry does not

#### Scenario: An unimplemented entry stays disabled

- **WHEN** the user opens the Filter menu
- **THEN** entries with no engine kernel, such as `Filter > Video > De-Interlace`, are disabled

#### Scenario: No pixel layer disables the filters

- **WHEN** the active layer is missing or is not an unlocked normal pixel layer
- **THEN** the filter commands are disabled

## ADDED Requirements

### Requirement: Filter parameter descriptor

Every filter kind the project offers SHALL be recorded once, in the filter parameter descriptor. A descriptor SHALL record the kind's menu path, or for a kind with no menu row the family path it belongs to and that it is not a menu entry; its label; its ordered parameter controls, and for each control the control kind, its slot count, its label, its range, its choices, and the value each of its slots opens on. The descriptor SHALL also record whether the kind resolves against the whole layer, which dialog edits it, whether it asks for a displacement map after OK, and its dialog presentation flags (thumbnail pane, stacked layout, preview below, live preview).

The descriptor SHALL be the app's only table of a kind's dialog controls, canonical slot count and dialog-opening defaults. The mapping's variable-list decoders SHALL remain the only other place a variable kind's slot grammar is recorded. The shared parameter controls for a kind SHALL show exactly the controls its descriptor records, in the descriptor's order, each edited by the control the descriptor names, within the range the descriptor gives, with the choices the descriptor lists, opening on the values the descriptor records. A kind with a dedicated dialog SHALL open that dialog; the dedicated dialog's layout is its own requirement's, and the descriptor's controls record the slots it produces. The mapping of a kind with an empty slot list SHALL build the filter from the descriptor's dialog-opening values, except that a commit uses the kind's commit normalization: Lens Blur's Preview radio is preview-only, so a commit always uses More Accurate. A kind with no descriptor SHALL not be a filter kind.

#### Scenario: The shared dialog opens on the descriptor's values [fpd_dialog_defaults]

- **WHEN** each kind edited by the shared parameter controls is opened
- **THEN** its controls, their order, their ranges, their choices and their opening values are the ones its descriptor records

#### Scenario: The mapping and the dialog agree on the defaults [fpd_mapping_defaults]

- **WHEN** each filter kind is mapped with an empty slot list and again with the descriptor's default vector
- **THEN** the two filters are equal, and a kind whose engine supplies a default produces that default

#### Scenario: The commit defaults are explicit [fpd_commit_defaults]

- **WHEN** Lens Blur is mapped with an empty slot list and then committed from its opening values
- **THEN** the mapped filter opens on Faster while the committed and stored slots carry More Accurate

#### Scenario: The menu rows are the descriptor's [fpd_cpp_rows]

- **WHEN** the filter menu is built
- **THEN** every menu row's kind, label, menu path, controls, slot counts, ranges, choices, default values, dialog kind, map-after-OK flag and presentation flags are the ones the descriptor records for the same kind, and the descriptor with no menu row has no menu entry but still resolves through `filterCommandForKind`

#### Scenario: Positional kinds come from the descriptor [fpd_whole_layer]

- **WHEN** a kind is asked whether it resolves against the whole layer
- **THEN** the answer is the one its descriptor records, and a kind with no descriptor is not a filter kind

#### Scenario: A kind opens the dialog the descriptor names [fpd_dialog_kind]

- **WHEN** a kind that owns its own dialog is chosen from the filter menu
- **THEN** that dialog opens for it, and a kind the descriptor names no dialog for that takes parameters opens the shared parameter dialog
