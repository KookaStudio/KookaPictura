# Spec Delta

## MODIFIED Requirements

### Requirement: The engine owns one adjustment catalogue

The engine SHALL publish one catalogue of adjustment and fill-layer kinds in CS6
menu order. Each entry SHALL carry:

- its kind id, as a typed engine value that every adjustment-or-fill decision in
  the workspace is written against;
- its CS6 title;
- its Image-menu command: a dialog under Image > Adjustments, a direct command
  under Image > Adjustments, a direct command at the Image menu root, or none;
- whether Layer > New Adjustment Layer (or New Fill Layer) offers it, and whether
  that leaf carries an ellipsis;
- its Adjustments panel tile icon id, if it has a tile;
- its Adjustments panel menu label, if it has a menu row;
- whether the GPU compositor can apply it, and by which route;
- its dialog-default and new-layer-default block builders, when applicable;
- its direct-adjustment constructor, when applicable.

The typed kind enum and its `ALL` list SHALL be generated from one declaration.
The engine SHALL expose the dialog block, new-layer block, and direct adjustment
through their catalogue row. Automated coverage SHALL compare the complete
catalogue with generated `ALL` in both directions.

The catalogue SHALL cover:

- the sixteen dialog adjustments;
- the six direct commands: Invert, Desaturate and Equalize under Image >
  Adjustments plus Auto Tone, Auto Contrast and Auto Color at the Image root;
- the three fill layers, Solid Color, Gradient and Pattern.

Shadows/Highlights SHALL be a dialog only, with no adjustment-layer entry. Invert
SHALL be both a direct command and an adjustment layer without an ellipsis. Every
Image-menu and layer-menu entry SHALL name a command that exists in the command
catalogue.

Each catalogued adjustment kind SHALL have exactly one typed kind value, and the
catalogue SHALL contain exactly one entry for it. A kind that carries an
adjustment layer SHALL have the same typed kind value whether the block came from
a dialog default or from a new layer.

HDR Toning, Match Color and Replace Color are image commands with their own
dialogs and no adjustment layer. They SHALL stay outside the catalogue.

#### Scenario: Catalogue covers every kind once [adjust_catalogue_complete]

- **WHEN** the catalogue is enumerated
- **THEN** every kind id appears exactly once. The dialog entries are the sixteen
  Image > Adjustments dialogs in CS6 order, with Color Lookup after Channel Mixer.
  Each dialog kind has a default adjustment block.

#### Scenario: Layer entries exclude Shadows/Highlights [adjust_catalogue_layer_rows]

- **WHEN** the New Adjustment Layer entries are listed in catalogue order
- **THEN** they are the sixteen CS6 layer adjustments with Color Lookup and Invert
  and without Shadows/Highlights, and only Invert lacks an ellipsis

#### Scenario: Catalogue entries resolve to commands [adjust_catalogue_commands]

- **WHEN** each entry's Image-menu path and layer-menu path is looked up in the
  command catalogue
- **THEN** every path resolves to a command id. The Auto entries resolve at the
  Image root.

## ADDED Requirements

### Requirement: Each adjustment kind fact has one source

For every generated kind the catalogue SHALL contain exactly one row. Each row
SHALL state these facts exactly once:

- the block a dialog opens on (`dialog_default`);
- the block a new adjustment layer carries (`layer_default`);
- the adjustment a direct menu command applies (`direct`);
- the GPU route: an inline shader id, a lookup-table path, or no GPU path.

The shell SHALL NOT restate a dialog default, a new-layer default, a direct
command, or a GPU id in a table of its own. Where a fact genuinely differs
between two kinds of use, such as a dialog opening on neutral parameters while a
new layer carries a visible change, both values SHALL be declared and
distinguishable, not merged into one.

A kind the shader applies directly SHALL declare its numeric shader id. The WGSL
marker SHALL be derived from that id, identify exactly one adjustment branch in
the shader's adjustment dispatch, and be used by exactly one catalogue entry.
The lookup-table shape selector's ids SHALL be included in the same uniqueness
and WGSL checks. A declared GPU route states that the kind has a GPU
implementation; it does NOT promise that every parameter set uses it. A
parameter shape the shader does not model SHALL still fall back to the CPU.

The mapping from a decoded adjustment block to its kind SHALL have one
implementation, used by both the shell's layer rows and the preset store.

The control server's adjustment kind list SHALL be the catalogue's dialog
entries, sixteen kinds including Color Lookup and Shadows/Highlights, not a
hand-written subset. Every listed kind SHALL have a non-empty parameter page.

#### Scenario: Every dialog default is the catalogue's [adjust_facts_dialog_default]

- **WHEN** each catalogue entry that opens an Image > Adjustments dialog is
  asked for its dialog default
- **THEN** the returned block decodes under that entry's title, and every
  parameter of its editor starts at the layout's declared default

#### Scenario: Every new layer carries the catalogue's layer default [adjust_facts_layer_default]

- **WHEN** a new adjustment layer is created for each catalogue entry that offers
  one, using the catalogue's layer default
- **THEN** the created layer's block decodes to that entry's typed kind, and the
  layer's display name is that entry's title

#### Scenario: The CMYK layer default carries the ink identity [adjust_facts_layer_default_cmyk]

- **WHEN** the Channel Mixer new-layer default is built for a CMYK document
- **THEN** the block carries the CMYK ink identity, with 100-percent rows and
  zero ink constants, and differs from the RGB identity a non-CMYK document gets

#### Scenario: Every direct command yields an adjustment [adjust_facts_direct]

- **WHEN** each catalogue entry that declares a direct command is asked for its
  adjustment
- **THEN** it yields an adjustment whose typed kind is that entry's kind, and a
  kind without a direct command yields none

#### Scenario: Every GPU id appears once in the shader [adjust_facts_gpu_id]

- **WHEN** each catalogue entry that declares a direct shader route is compared
  with the adjustment dispatch in the shader source, together with the ids
  returned by the lookup-table shape selector
- **THEN** each id's derived `kind == {id}u` marker appears exactly once inside
  that dispatch, and no id is declared twice

#### Scenario: Decoded blocks resolve to one kind [adjust_facts_kind_map]

- **WHEN** a block for each catalogued adjustment kind is decoded
- **THEN** the shell's layer-row kind and the preset store's kind both report
  that kind, and both use the same mapping

#### Scenario: The control server lists every dialog kind [adjust_catalogue_control_kinds]

- **WHEN** the control server's adjustment kinds are listed
- **THEN** they are the catalogue's sixteen dialog entries by name, including
  Color Lookup and Shadows/Highlights, and every listed kind has a non-empty
  parameter page

#### Scenario: GPU parity follows the catalogue rows [adjust_facts_gpu_parity_rows]

- **WHEN** the GPU adjustment parity case is built
- **THEN** it iterates the catalogue's GPU-route entries instead of a hand list,
  requires a layer default for every routed row, composites each one through
  both backends, and asserts that the checked count equals the routed-row count

#### Scenario: A GPU-capable kind still falls back on unsupported parameters [adjust_facts_gpu_param_shape]

- **WHEN** a kind that declares a GPU route is composited with a parameter set
  the shader does not model
- **THEN** the GPU compositor declines that layer and the CPU path produces the
  result, rather than silently applying the adjustment unchanged

#### Scenario: Documents keep the declared defaults distinct [adjust_facts_defaults_distinct]

- **WHEN** every kind's dialog default and new-layer default are built with the
  same default black/white foreground and background, and inspected
- **THEN** both blocks are declared, and Brightness/Contrast, Hue/Saturation and
  Photo Filter differ there. Gradient Map differs only when the
  foreground/background are not that default pair, because its dialog default is
  built from them. Every differing pair is assertable separately, so neither
  value is merged into the other
