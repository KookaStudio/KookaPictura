# Spec Delta

## Purpose

The engine-owned catalogue that makes one filter row the source of the kind's engine facts, parameter schema and ranges, slot construction, GPU eligibility, menu entry, and gallery membership, so the shell and the script catalogue cannot drift from the engine.

## ADDED Requirements

### Requirement: The engine owns one complete filter catalogue

`pictura-filters` SHALL publish exactly one catalogue row for every filter kind, in the order the Filter menu's former table used. Each row SHALL carry the kind's identifier and display name, the engine that applies it, its alpha policy, its preview apron policy, its CPU/GPU policy, its whole-layer preview policy, its parameter schema (control, range, default, choices, suffix, decimals, and pixel-distance flag), its menu path and label, its menu visibility, its dialog and preview policy, its gallery rank, and its slot-to-filter constructor.

The filter kind list and variant names SHALL derive from the macro rows that name the spec consts; each spec row SHALL carry the bridge id and display name. The engines, alpha queries, apron queries, and CPU/GPU queries SHALL derive from the same spec rows. A filter kind added without a row MUST fail to compile, and a row whose kind does not match its position in the kind list MUST fail the catalogue coverage test.

#### Scenario: Every kind has exactly one row [fc_catalogue_complete]

- **WHEN** the catalogue is enumerated
- **THEN** it holds one row per filter kind, no kind appears twice, and each row's kind matches the kind it is reached from

#### Scenario: A kind without a row does not compile [fc_catalogue_missing_row]

- **WHEN** a filter kind is added without a catalogue row
- **THEN** the filter crate does not compile

#### Scenario: The row's facts round-trip [fc_facts_round_trip]

- **WHEN** a row's kind is asked for its variant name, engine, alpha policy, apron policy, and CPU/GPU policy
- **THEN** the variant name is the one recorded by the macro row and each fact is the value recorded on that spec row, and the facts keep the values the filter type reports today

### Requirement: Parameter defaults are complete and inside their declared range

Every row's canonical default slot list SHALL cover all of its control slots, and every default value SHALL lie inside its control's declared range under the control's own semantics. The defaults SHALL build the same filter as an empty slot list.

#### Scenario: Every default list builds its kind [fc_defaults_build]

- **WHEN** each row's canonical defaults are used to construct its filter
- **THEN** a filter of that kind is produced, and an empty slot list produces the same filter

#### Scenario: Every default is in range [fc_defaults_in_range]

- **WHEN** each row's defaults are checked against its parameter schema
- **THEN** every scalar default, colour component, placement component, matrix cell, and curve point lies inside the range its control declares

### Requirement: A parameter range has one source

The catalogue row SHALL be the only source of a filter parameter's declared range. The slot decoder and the ported engine's scalar validation SHALL read the same range from the row; no mapping clamp or dispatch literal MAY restate it. Structural constraints that are not a control range (dependent Wave limits, Shear point bounds, texture option bounds) SHALL remain explicit checks and SHALL accept every value the row's defaults produce.

#### Scenario: Add Noise has one amount range [fc_add_noise_range]

- **WHEN** the Add Noise row and the engine validate an amount
- **THEN** both use `0.1..400`, an amount of `0.1` is accepted, and an amount of `0.0` is refused

#### Scenario: Surface Blur threshold follows the documented range [fc_surface_range]

- **WHEN** the Surface Blur row and the engine validate a threshold
- **THEN** both use `1..255`, a threshold of `1` is accepted, and a threshold of `0` is refused

#### Scenario: An out-of-range slot is refused [fc_range_refused]

- **WHEN** a recognised kind is built from a slot list whose value lies outside that slot's row range
- **THEN** no filter is produced or the engine refuses it, and the document is unchanged

### Requirement: The catalogue row owns slot construction

The catalogue SHALL construct every filter kind from an ordered slot list. Fixed-shape kinds SHALL be built by their row's constructor without a separate kind arm in the shell. Variable-shape kinds (Blur Gallery pins, Lighting lights, Shear curves, Displace maps) SHALL keep their documented record grammars. Resource-backed kinds SHALL resolve their resource through the app's resource seam; the shell SHALL provide only that resolution and the FFI mirror.

#### Scenario: Fixed-shape kinds build from their row [fc_fixed_build]

- **WHEN** a fixed-shape kind is built from its canonical defaults or an explicit slot list
- **THEN** the row's constructor produces the filter with those values, and a list of the wrong length is refused

#### Scenario: Variable-shape kinds accept their documented lists [fc_variable_build]

- **WHEN** each variable-shape kind is built from its documented list shapes
- **THEN** it produces the documented filter, and a list that does not decode as one of those shapes is refused

#### Scenario: Resource-backed kinds resolve through the app seam [fc_resource_build]

- **WHEN** a resource-backed kind is built with a resource id from the app's stores
- **THEN** the row's constructor resolves the kept resource and produces the filter, and an id the store no longer keeps is refused

### Requirement: GPU eligibility comes from the catalogue row

Each row SHALL declare whether its kernel is unsupported on the GPU, accelerated, or CPU-preferred. The GPU plan SHALL use that declaration as its eligibility decision instead of a parallel list. Every accelerated kind SHALL have a GPU parity case, and every GPU parity case SHALL name an accelerated kind.

#### Scenario: Accelerated rows and parity cases agree [fc_gpu_parity_list]

- **WHEN** the GPU parity fixture and the catalogue are compared
- **THEN** the fixture covers exactly the accelerated and CPU-preferred kinds that declare a GPU kernel, and no other kind

#### Scenario: The plan agrees with the row [fc_gpu_plan]

- **WHEN** an accelerated kind's representative filter is planned
- **THEN** the plan yields a GPU kernel, and an unsupported kind yields no plan

### Requirement: Menu and gallery surfaces come from the catalogue

The Filter menu's rows and the Filter Gallery's membership and order SHALL come from the catalogue through the existing FFI bridge. The shell SHALL NOT keep a hand-written filter menu or gallery table. The gallery SHALL group by the row's menu family and order by its gallery rank, preserving the CS6 category order and counts.

#### Scenario: Menu rows mirror the catalogue [fc_menu_rows]

- **WHEN** the shell enumerates its filter menu rows
- **THEN** each menu-visible catalogue row appears once with its path and label, and no non-menu row appears

#### Scenario: Gallery categories follow the catalogue ranks [fc_gallery_order]

- **WHEN** the gallery groups its filters
- **THEN** it shows Artistic, Brush Strokes, Distort, Sketch, Stylize, and Texture in CS6 order with their existing counts, and the rows inside each category follow the catalogue ranks

### Requirement: Engine-only kinds stay out of filter surfaces

A kind that the engine applies but that is not a Filter-menu command SHALL have a catalogue row with no menu path, no menu flag, no parameters, and no gallery rank. It SHALL satisfy catalogue completeness while remaining absent from the Filter menu, the gallery, and the scriptable filter command set.

#### Scenario: HDR Toning is engine-only [fc_hdr_toning_engine_only]

- **WHEN** the catalogue is enumerated and the filter command surfaces are built
- **THEN** HDR Toning has exactly one row and does not appear as a filter menu entry, a gallery filter, or a supported scriptable filter command

### Requirement: Catalogue consumers are checked against the catalogue

The shell's FFI mirror, the GPU parity fixture, and the script catalogue membership test SHALL read the engine catalogue directly. The script test SHALL resolve each menu-visible filter row to a command definition without reading descriptor source text or depending on the app crate.

#### Scenario: Every menu row resolves a script command [fc_script_commands]

- **WHEN** the script catalogue membership test runs
- **THEN** every menu-visible filter row resolves to a command definition, and the three documented unwired leaves still have none

#### Scenario: The FFI mirror covers every row [fc_ffi_mirror]

- **WHEN** the shell requests its filter descriptors
- **THEN** it receives one row per catalogue row in catalogue order, with the same paths, labels, parameter schemas, dialog policy, and gallery ranking
