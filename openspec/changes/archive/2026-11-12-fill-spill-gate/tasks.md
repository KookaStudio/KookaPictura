# Tasks

## 1. Tests named by the plan

- [x] 1.1 `pictura-app`: `a_large_fill_spills_its_tiles`, `a_small_document_fill_charges_its_refresh`.
- [x] 1.2 Qt: `tst_memory_governor::fillsSpillToScratch`, `tst_memory_governor::fillRefusalNamesFullScratchDisks`.
- [x] 1.3 Qt: `tst_document_jobs::aLargeGradientRunsAsAJob`, `tst_document_jobs::cancellingAFillChangesNothing`, `tst_document_jobs::aSmallGradientFillsAtOnce`.

## 2. App

- [x] 2.1 `memory_gate::fill_cost` and `refresh_working`; `SYNC_PIXELS` and `REPAIR_PIXELS` visible to the gate.
- [x] 2.2 The four fills gate with `admit_spilling`.
- [x] 2.3 `sample_tiles` takes the stored-composite flag; `apply`'s closure loses its view parameter.
- [x] 2.4 `apply_job`: gate on the GUI thread, then a `document_op` job for an armed, large, pixel-layer fill; `Outcome.rect`.
- [x] 2.5 Shell: the Gradient and Paint Bucket tools and Edit > Fill / Stroke run through `runDocumentCommand`; `fillMemoryRefusalMessage` names full scratch disks.
- [x] 2.6 `ROADMAP.md`: no entry tracks large-document fills; this fix ships no roadmap feature.
