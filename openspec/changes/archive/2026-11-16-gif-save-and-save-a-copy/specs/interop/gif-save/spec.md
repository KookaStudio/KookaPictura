# Spec Delta

## Purpose

Saving the active document as CompuServe GIF from a snapshot, including the
Indexed Color hand-off a non-Indexed document needs before its pixels can be
represented in a GIF palette.

## ADDED Requirements

### Requirement: GIF is a save destination

`File > Save As` and `File > Save a Copy` SHALL offer CompuServe GIF in the
format list with `.gif` as its default suffix. A destination whose extension is
`.gif` in any case SHALL produce a GIF file, whether the path was chosen through
the GIF filter or typed with the All Formats filter. `File > Save` on a document
whose recorded path ends in `.gif` SHALL write a GIF at that path; it MUST NOT
write PSD bytes or another container under a `.gif` name. A single-layer
document imported from a GIF SHALL have GIF preselected as its Save As format.
A destination whose pixel width or height exceeds 65,535 SHALL be refused
without writing. GIF writes SHALL be atomic: a refused encode (oversized
dimensions) or a failed write SHALL leave any previous file intact and SHALL
report failure without a partial file.

#### Scenario: Save As offers GIF [gif-save-filter]

- **WHEN** Save As is opened for an open document
- **THEN** the format list contains CompuServe GIF, and choosing it for a name without a suffix yields a `.gif` destination

#### Scenario: Save rewrites a recorded GIF path [gif-save-recorded-path]

- **WHEN** a single-layer document's recorded path ends in `.gif` and the user runs Save
- **THEN** the file begins with the GIF89a signature and decodes to the document size

#### Scenario: A typed GIF suffix wins over the native default [gif-save-typed-suffix]

- **WHEN** the user types `cat.gif` with the All Formats filter and confirms Save As
- **THEN** a GIF89a file is written at `cat.gif`, not a PSD payload

#### Scenario: A GIF import preselects GIF [gif-save-import-default]

- **WHEN** a single-layer GIF is opened and Save As is invoked
- **THEN** the GIF format is preselected

#### Scenario: A failed GIF write preserves the previous file [gif-save-atomic]

- **WHEN** the GIF destination cannot be written and a file already exists there
- **THEN** the save reports failure, the existing file's bytes are unchanged, and no partial file remains

#### Scenario: An oversized GIF destination is refused [gif-dimension-limit]

- **WHEN** a document wider or taller than 65,535 pixels is saved to `.gif`
- **THEN** the save is refused, no file is written, and the document is unchanged

### Requirement: Non-Indexed documents pass through the Indexed Color dialog

Saving to a `.gif` destination from a document that is not already in Indexed
Color mode SHALL show the Indexed Color dialog before anything is written, with
its Palette, Colors, Dither, Amount, and Preview controls. The accepted settings
SHALL govern the written palette and SHALL become the remembered GIF settings
for the session. Cancelling the dialog SHALL abort the save: no file is written
and the document's pixels, mode, depth, path, modified state, and history are
unchanged. A document already in Indexed Color mode SHALL NOT show the dialog.
The preview MAY show the conversion while the dialog is open; closing the dialog
by accept or cancel SHALL restore the pre-preview document and SHALL add no
history state. The Preview control SHALL be disabled when the conversion cannot
preview. When the remembered palette is Exact and the document has more than 256
colors, the dialog SHALL select an available palette and the accepted settings
SHALL govern the file.

#### Scenario: An RGB document opens the Indexed Color dialog [gif-rgb-dialog]

- **WHEN** Save As is confirmed for a `.gif` destination from an RGB document
- **THEN** the Indexed Color dialog is shown before anything is written

#### Scenario: Cancelling writes nothing [gif-save-cancel]

- **WHEN** the Indexed Color dialog is cancelled during a GIF save
- **THEN** no file is written and the document is unchanged

#### Scenario: An indexed document saves without the dialog [gif-indexed-no-dialog]

- **WHEN** a document already in Indexed Color mode is saved to `.gif`
- **THEN** no Indexed Color dialog is shown

#### Scenario: Preview restores the document [gif-preview-restores]

- **WHEN** Preview is toggled in the Indexed Color dialog during a GIF save and the dialog is then cancelled or accepted
- **THEN** the pre-preview document is restored and no history state is added

#### Scenario: Accepted settings seed the next GIF dialog [gif-settings-remembered]

- **WHEN** GIF palette settings are accepted and the Indexed Color dialog is opened for another GIF save
- **THEN** its controls hold the accepted settings

#### Scenario: A remembered Exact palette falls back [gif-exact-fallback]

- **WHEN** the remembered GIF palette is Exact and a document with more than 256 colors opens the GIF dialog
- **THEN** the dialog selects an available palette, and the accepted settings govern the file

### Requirement: Saving GIF writes a snapshot

Saving GIF SHALL NOT change the document's mode, bit depth, pixels, or history;
it is a file write, not an undoable document operation. When the document
carries an Indexed source whose retained palette and index plane still expand to
the current composite, the written file SHALL use them without re-quantizing.
When an edit has made the retained planes stale, the application SHALL quantize
the current composite instead. Otherwise the application SHALL quantize the
document's 8-bit sRGB composite with the accepted palette, color count, dither,
and amount. A 16-bit or 32-bit document SHALL be written as 8-bpc GIF while
remaining at its original depth; a CMYK or Lab document SHALL be written from
its 8-bit sRGB composite while remaining in its mode. A document whose layers,
groups, adjustments, or type layers cannot be held by GIF SHALL warn before
writing and then write the flattened composite. The file SHALL be a single-frame
GIF89a whose row order is Normal.

#### Scenario: An unchanged indexed source keeps its palette [gif-indexed-palette]

- **WHEN** a PSD carrying an unchanged Indexed source is saved as GIF
- **THEN** the written file uses that source's palette and pixel indices

#### Scenario: An edited indexed source uses the current composite [gif-edited-indexed]

- **WHEN** a document is edited after it was converted to Indexed Color and is then saved as GIF
- **THEN** the written file uses the edited composite rather than the stale retained indices

#### Scenario: An RGB snapshot is quantized [gif-quantized]

- **WHEN** an RGB document with more than 256 colors is saved as GIF
- **THEN** the written file decodes to the document size with at most 256 palette colors

#### Scenario: A 16-bit document stays 16-bit [gif-16bit-snapshot]

- **WHEN** a 16-bit RGB document is saved as GIF
- **THEN** the file is 8-bpc GIF and the document remains 16-bit with unchanged pixels and history

#### Scenario: A 32-bit document stays 32-bit [gif-32bit-snapshot]

- **WHEN** a 32-bit RGB document is saved as GIF
- **THEN** the file is 8-bpc GIF and the document remains 32-bit with unchanged pixels and history

#### Scenario: A CMYK document saves its sRGB snapshot [gif-cmyk-snapshot]

- **WHEN** a CMYK document is saved as GIF
- **THEN** the file is 8-bpc GIF from the sRGB composite and the document remains CMYK with unchanged pixels and history

#### Scenario: Features GIF cannot hold warn first [gif-layered-warning]

- **WHEN** a layered document is saved to a `.gif` destination
- **THEN** a warning states that the flattened composite will be written, and on confirmation a GIF of the visible composite is written
