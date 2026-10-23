## MODIFIED Requirements

### Requirement: Filter parameter descriptor

Every filter kind the project offers SHALL be recorded once, in the filter parameter descriptor. A descriptor SHALL record the kind's menu path, or for a kind with no menu row the family path it belongs to and that it is not a menu entry; its label; its ordered parameter controls, and for each control the control kind, its slot count, its label, its range, its choices, the value each of its slots opens on, and whether the control's value is a distance in document pixels. The descriptor SHALL also record whether the kind resolves against the whole layer, which dialog edits it, whether it asks for a displacement map after OK, and its dialog presentation flags (thumbnail pane, stacked layout, preview below, live preview).

The descriptor SHALL be the app's only table of a kind's dialog controls, canonical slot count, dialog-opening defaults, and pixel-distance facts. The mapping's variable-list decoders SHALL remain the only other place a variable kind's slot grammar is recorded. The shared parameter controls for a kind SHALL show exactly the controls its descriptor records, in the descriptor's order, each edited by the control the descriptor names, within the range the descriptor gives, with the choices the descriptor lists, opening on the values the descriptor records. A kind with a dedicated dialog SHALL open that dialog; the dedicated dialog's layout is its own requirement's, and the descriptor's controls record the slots it produces. The mapping of a kind with an empty slot list SHALL build the filter from the descriptor's dialog-opening values, except that a commit uses the kind's commit normalization: Lens Blur's Preview radio is preview-only, so a commit always uses More Accurate. A kind with no descriptor SHALL not be a filter kind.

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
- **THEN** every menu row's kind, label, menu path, controls, slot counts, ranges, choices, default values, pixel-distance flags, dialog kind, map-after-OK flag and presentation flags are the ones the descriptor records for the same kind, and the descriptor with no menu row has no menu entry but still resolves through `filterCommandForKind`

#### Scenario: Positional kinds come from the descriptor [fpd_whole_layer]

- **WHEN** a kind is asked whether it resolves against the whole layer
- **THEN** the answer is the one its descriptor records, and a kind with no descriptor is not a filter kind

#### Scenario: A kind opens the dialog the descriptor names [fpd_dialog_kind]

- **WHEN** a kind that owns its own dialog is chosen from the filter menu
- **THEN** that dialog opens for it, and a kind the descriptor names no dialog for that takes parameters opens the shared parameter dialog
