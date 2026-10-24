## MODIFIED Requirements

### Requirement: Section previews touch only their section

A filter dialog's section preview SHALL filter the visible section plus each
filter's apron, clamped to the layer. It SHALL save the pre-preview pixels of
that section alone and write the live layer's planes in place, without copying
a whole plane. The next preview or a cancel SHALL put back only that section.
The refreshed region SHALL be the union of the section written and the section
put back. A commit SHALL first restore the section, then filter the whole
layer, so the preview never compounds into the result. A preview the engine
refuses SHALL restore the pre-preview pixels and SHALL recomposite the document
before returning, so the canvas and composite equal the state before the
preview and no preview session remains open. A cancel after a refusal SHALL be
a no-op.

#### Scenario: A re-preview elsewhere restores the first section

- **WHEN** a Gaussian Blur is previewed over one section and then over another
- **THEN** the layer equals a fresh preview of the second section alone, and the reported refresh region covers both sections

#### Scenario: The live planes are written in place

- **WHEN** two section previews run and are cancelled on a layer whose planes no one else shares
- **THEN** each plane keeps its allocation and the layer is bit-identical to before

#### Scenario: A commit after a section preview is exact

- **WHEN** a section preview is followed by a commit of the same filter
- **THEN** the layer equals a fresh commit of that filter

#### Scenario: A refused parameter change restores the canvas [fdp_refused_restore]

- **WHEN** a parameter change makes the filter kernel refuse, and the dialog is then cancelled
- **THEN** the layer and composite equal the pixels before the preview, no preview session remains open, and cancelling changes nothing further

### Requirement: Zoomed-out previews use a reduced copy

Below 100% canvas zoom, when the preview would cover more than 4 MP of
document, the filter dialog SHALL preview on a reduced copy of the region
instead of the document. The region is the visible area, or the whole document
for a filter whose descriptor marks it whole-layer, including Radial Blur,
Clouds, Difference Clouds, and Fibers. The copy SHALL be box-averaged to about
the canvas's device pixels, at most 4 MP, never scaled up, built from the
pre-preview pixels, and kept while the region, scale and document are
unchanged. Each parameter change SHALL first draw a draft on the canvas over
that region: the copy filtered with every parameter the descriptor marks as a
document-pixel distance scaled by the copy's scale, rounded on an integer
control and held to its range. Every other parameter SHALL keep its value. The
pane SHALL show its own section filtered at full resolution, off the document.
The document SHALL NOT change until OK.

#### Scenario: A large zoomed-out document previews on the copy

- **WHEN** Add Noise is previewed on a 3000×2400 document shown at 10% zoom
- **THEN** the canvas shows the preview, the document is unchanged, and cancelling removes the canvas preview and leaves the document and history as they were

#### Scenario: Pixel parameters scale with the copy

- **WHEN** a dialog's values are scaled for a copy at a quarter of the document's size
- **THEN** a 5-pixel Gaussian radius becomes 1.25, an Unsharp Mask amount stays 150%, and a 2-pixel Median radius rounds to its 1-pixel minimum

#### Scenario: An unsuffixed pixel control scales with the copy [fdp_unsuffixed_scale]

- **WHEN** Smart Blur Radius, Mosaic Cell Size, Crystallize Cell Size, Pointillize Cell Size, or Color Halftone Max Radius is previewed on a copy at a quarter of the document's size
- **THEN** each value is scaled by 0.25, rounded on an integer control and held to its range, exactly as a control whose unit says pixels

#### Scenario: The pane shows its section exactly

- **WHEN** the pane's section is rendered while the canvas shows the copy
- **THEN** it equals that section of a section preview of the same filter, and the document is unchanged

#### Scenario: A layer mask target keeps the exact preview

- **WHEN** the dialog previews while the active target is a layer mask
- **THEN** no reduced copy is made and the mask is previewed exactly

#### Scenario: Whole-layer kernels preview the whole layer [fdp_whole_layer]

- **WHEN** Radial Blur, Clouds, Difference Clouds, or Fibers is previewed with a cropped viewport
- **THEN** the preview filters the whole layer, and the visible region matches the commit rather than a crop-sized kernel
