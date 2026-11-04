# Design

## One budget predicate

`image_adjust/new_document/imp.rs` held `fits_budget` privately, so only the
New Document bridge could call it. The predicate moves to
`cxxqt_object/helpers.rs` as `document_fits(width, height, bits, budget)`, with
`document_budget_allows` and `resize_budget_allows` on top. The existing
`document_budget_allows` bridge delegates to it, and the unit tests move with
the function.

The predicate keeps the established policy: at most 30 000 px a side, at most
2 GiB for one `width * height * 4 * depth` allocation, and at most the
document budget. The default document budget is three quarters of available
memory, but never below one 2 GiB allocation, so it cannot see a low-memory
host. A resize allocates the target immediately, so `allows_resize` bounds the
document budget by three quarters of the memory available now instead, with no
floor. New Document keeps the default; the resize bridge, Image Size, and the
control server's size commands use the resize bound.

## Where the gate runs

- `image_size_apply` computes the document's depth bits first, then checks only
  when a resample is requested and the target differs from the current size. A
  resolution-only change (Resample off) allocates no pixels and is not refused.
- `PictureView::resize_image` checks before `resize_document` for the same
  reason.
- The Image Size handler preflights with the same C++ helper before calling
  `applyImageSize`, so it can show a warning; the executor preflights silently.
  The Rust gate stays authoritative for a direct bridge call.

A refusal happens before any mutation, so the document, its history and its
modified flag are untouched; the existing "nothing changed" contract of both
functions already covers the caller.

## The warning

`imageSizeFitsBudget(view, width, height)` calls `document_resize_budget_allows`
with the document's depth bits. `imageSizeBudgetMessage(width, height)` returns
the warning text, which names the target and the memory shortfall. The handler
shows it through `showImageSizeBudgetWarning` and returns without recording a
command. A file-local driver in `image_size_dialog.cpp`, set by
`setImageSizeDialogDriverForTest`, answers the dialog and the warning so a Qt
test can check both without a modal box.

## Ceiling

The gate bounds the target's own `width * height * 4 * depth` plane by three
quarters of the memory available now; it does not estimate the full working set
(layers + composite + level-0 frame + pyramid) or the old document that history
retains. That is enough for the reported failure: with 456 MB available, the
17000² plane's 1.16 GB is refused instead of allocated. A machine with enough
free memory for the plane but not for the whole working set can still swap; a
working-set estimate and scratch paging are tracked separately (#943).
