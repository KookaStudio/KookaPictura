# Proposal

## Why

`File > New` always opens on Default Photoshop Size (1280×800) even when the
system clipboard holds an image. CS6 preselects the Clipboard document type and
preloads the clipboard image's dimensions, and `docs/10-workflow-io/open-and-new.md`
lists that as a parity acceptance criterion ("Given a copied selection on the
clipboard, `File > New` preloads the clipboard image's dimensions and
resolution"). Today the user must switch Document Type to Clipboard by hand.

## What Changes

- `File > New` SHALL open with Document Type preselected to `Clipboard` and the
  width/height fields preloaded from the clipboard image's pixel dimensions
  whenever the system clipboard holds an image.
- With no image on the clipboard the dialog SHALL keep its current default
  (Default Photoshop Size, Clipboard type disabled).
- Every other dialog interaction (choosing another type, editing a dimension,
  presets, units, mode/depth) is unchanged.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `document/document-lifecycle`: the New document creation requirement gains
  the clipboard-derived default size.

## Impact

- `crates/pictura-app/cpp/new_document_dialog.{h,cpp}`: initial Document Type
  selection.
- `crates/pictura-app/cpp/tests/tst_new_document.cpp`: a Qt Test case covering
  the clipboard and empty-clipboard openings.
- No engine, codec, or dependency changes.
