# imaging/filter-dialog-preview Specification

## Purpose
Filter dialog section previews compute only the section they show.

## Requirements

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

### Requirement: The draft gives way to the exact picture

While the canvas shows a zoomed-out draft, a worker thread SHALL filter the
full-resolution layer with the dialog's values and reduce the result to the
draft's region and scale. When the worker finishes, that exact picture SHALL
replace the draft on the canvas. A newer parameter change SHALL supersede an
older request: only the latest result is shown, and one worker runs at a
time. While the exact picture is being computed, the dialog SHALL show an
animated busy bar. A commit of the same filter SHALL reuse the worker's
filtered layer instead of filtering again.

#### Scenario: The exact picture replaces the draft

- **WHEN** a zoomed-out preview's worker finishes
- **THEN** the canvas shows the reduced full-resolution result and the busy bar hides

#### Scenario: Only the latest request is shown

- **WHEN** a second parameter change arrives while the first is being computed
- **THEN** the picture shown is the one for the second change

#### Scenario: A commit reuses the exact preview

- **WHEN** OK is pressed after the exact picture for the same values is ready
- **THEN** the commit installs the layer the worker filtered

### Requirement: Filter dialogs read no full-resolution image

The filter dialog, Lens Flare and Lighting Effects SHALL NOT build a
full-resolution image of the document. The pane SHALL read only its section from
the view pyramid's level 0. A placement pad or a proxy SHALL read the smallest
pyramid level at least its own size.

#### Scenario: The pane reads its section

- **WHEN** the pane updates for a parameter change, a zoom step or a resize
- **THEN** it reads a crop the size of its section, not the document

### Requirement: Parameter changes coalesce

A parameter change SHALL schedule the canvas preview for the next turn of the
event loop instead of running it at once. Changes that arrive before it runs,
such as input queued behind a slow preview, SHALL produce a single preview of
the latest values. Clearing Preview or closing the dialog SHALL drop a pending
preview.

#### Scenario: An edit previews on the next turn

- **WHEN** a spin box value changes with Preview checked
- **THEN** the canvas preview appears once the event loop runs, and clearing Preview restores the document at once

### Requirement: Filter commits run off the UI thread

Applying a filter from the menu, Last Filter or the Filter Gallery SHALL
filter the layer, composite the changed region and capture history on a worker
thread, then swap the results in on the UI thread. User input SHALL wait while
it runs, paints and timers SHALL continue, and a window with an animated busy
bar SHALL appear when the commit outlasts a quarter second. A zoomed-out
dialog's canvas preview SHALL stay up until the commit lands. The result SHALL
equal the synchronous commit. A refused commit SHALL leave the document and
history unchanged.

#### Scenario: A background commit equals the synchronous one

- **WHEN** a Gaussian Blur is committed on a worker
- **THEN** the layer and the stored composite equal the synchronous commit's, and history gains one state

#### Scenario: A locked layer is refused before the worker starts

- **WHEN** a commit is started on a pixel-locked layer
- **THEN** it is refused with a reason, no worker starts, and the document and history are unchanged

### Requirement: History names the filter

A filter committed from the menu or Last Filter SHALL be recorded as
"Filter: <menu name>". A Filter Gallery commit SHALL be recorded as
"Filter Gallery: " followed by the visible effects' names in order, separated
by commas.

#### Scenario: A menu filter is named

- **WHEN** Gaussian Blur is applied from its dialog
- **THEN** the new history state reads "Filter: Gaussian Blur"

#### Scenario: A gallery stack is named

- **WHEN** the gallery commits Grain then Texturizer
- **THEN** the new history state reads "Filter Gallery: Grain, Texturizer"

### Requirement: Large whole-layer previews run on the worker at full zoom

At 100% canvas zoom or above, when the preview covers more than 4 MP of
document and the filter's descriptor marks it whole-layer, the filter dialog
SHALL run the preview on the exact-preview worker instead of filtering the live
layer on the GUI thread. The GUI thread SHALL keep the last displayed picture
and keep processing events; the worker SHALL filter the whole layer with the
dialog's values and reduce the result to the preview region and scale. The
preview region and scale SHALL be capped to the same 4 MP budget used by the
reduced copy, and the exact picture SHALL be installed on the canvas when it
lands. A newer parameter change SHALL supersede an older request, with one
worker running at a time. Cancel SHALL drop the request and leave the layer,
composite, and history unchanged. A commit of the same values SHALL reuse the
worker's filtered layer instead of filtering again. A layer-mask target SHALL
keep the mask-sized synchronous preview path even at 100% and above.

#### Scenario: The event loop stays live during a large whole-layer preview [fdp_full_zoom_worker]

- **WHEN** a whole-layer filter preview over more than 4 MP is held in flight by the test seam at 100% zoom
- **THEN** a repeating timer keeps firing with no gap over 50 ms, the canvas keeps repainting, and the canvas shows the exact picture once the worker lands

#### Scenario: Cancel leaves the document unchanged [fdp_full_zoom_cancel]

- **WHEN** a large whole-layer preview is cancelled before or after its worker finishes
- **THEN** the layer, the composite, and the history equal their state before the preview, and the canvas preview is removed

#### Scenario: A commit reuses the worker result [fdp_full_zoom_commit]

- **WHEN** OK is pressed after the exact picture for the same values is ready
- **THEN** the commit installs the layer the worker filtered and records one history state

#### Scenario: A layer-mask target keeps the synchronous preview [fdp_full_zoom_mask]

- **WHEN** a whole-layer filter preview over more than 4 MP at 100% zoom targets a layer mask
- **THEN** the dialog previews the mask synchronously on the mask-sized plane and Cancel restores the mask exactly
