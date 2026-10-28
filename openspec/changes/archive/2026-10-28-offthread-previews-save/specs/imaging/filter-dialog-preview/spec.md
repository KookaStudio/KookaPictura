# Spec Delta

## ADDED Requirements

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
