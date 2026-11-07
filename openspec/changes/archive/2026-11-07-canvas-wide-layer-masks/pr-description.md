Raster masks on small layers and imported masks previously limited editing to
stored bounds, exposed their old exterior color when moved, and clipped selected
coverage at their edges. Masks now accept canvas-wide edits, expose black when
moved, and retain off-canvas coverage, including exact native 16-bit/float samples.
Raster mask thumbnails show opaque document-shaped grayscale coverage with black
outlines and white brackets on the active thumbnail. Rounded card edges stay covered. Mask cards follow the document aspect for every
owner kind, independently of square content cards. Plain row clicks activate
content; new adjustments are selected with their mask active. Imported masks
normalize only when an edit succeeds; cancellation and undo restore their complete
previous state.

Validation before rebasing onto current master: full verification checks passed: 8,033 tests passed, 124 skipped,
zero failed. This includes all 5,071 workspace Rust tests, application self-tests,
and the full Qt suite. Formatting, Clippy, all guards, strict OpenSpec validation,
control checks, and memory-harness checks passed. The interrupted full gate
completed its Rust checks; app self-tests, the complete Qt layer, and remaining
checks then finished sequentially with one job/process. The focused Rust run also
passed all 3,177 tests. No golden baselines or dependencies changed.

Manual reviewer checks: none. The three archived test plans map all 90 scenario rows
to automated checks.

Mask-link flag serialization is handled by the upstream correction in #1200
(PR #1244); this change does not introduce a second interpretation.

The rebase preserves upstream mask-link semantics and adapts movement fixtures.
No local builds or application tests run after rebasing; CI validates the final branch.
