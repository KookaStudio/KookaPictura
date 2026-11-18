# Design

## Context

`frame_place.cpp` dispatches PSD/PSB placement separately from raster placement. The codec already renders selected PDF pages through the Ghostscript import backend. The Smart Object writer stores an embedded source record separately from the layer's pixel channels. Imported external records are preserved and classified, and the reader currently skips the external descriptor.

The existing Edit Contents coordinator runs an internal editor for PSD/PSB. Its session stores a row path. External edits can outlive layer reordering and document switches, so they need a stable source identity and a commit-time check. The clipboard currently mirrors a Qt image and retains Pictura's own positioned clip.

## Goals / Non-Goals

**Goals**

- Share one PDF/AI source preparation path between Place, external edits, vector paste, and vector replacements selected during link repair.
- Keep source bytes, display pixels, layer properties, and history consistent across failed renders, Undo/Redo, and PSD/PSB writes.
- Treat repair of imported external records as a post-CS6 extension with an explicit user-selected file.

**Non-goals**

- Legacy PostScript AI or EPS as editable Smart Object sources. EPS keeps its raster import path.
- Place Linked, Convert to Linked, link packaging, automatic refresh, and cross-document updates.
- Native Files panel and file browser preferences. [#624](https://github.com/KookaStudio/KookaPictura/issues/624)

## Decisions

### Preserve source bytes and prepare display pixels off the UI thread

Sniff PDF-compatible content and reuse the existing PDF probe and page/box renderer. Stage the full source, the page and crop selection, and the transparent raster proxy together. Use the normal Place fit and Free Transform lifecycle. Use preview pixels during interactive transforms and prepare new pixels from the source when placement or a scale change commits. Cache or persist the resulting proxy through existing layer channels. Display and save can consume those channels without invoking Ghostscript during each render. A vector operation whose render fails leaves the previous transaction state intact.

Persist page and crop metadata with the Smart Object, using the placed descriptor where it represents the selection and a private block only where needed. Preserve the incoming filename and file type. Keep source bytes in the embedded record, including after exporting and after a PSD/PSB save and reopen. Storing a rasterized PSD as the source would discard the original format and prevent the external round-trip.

### Apply an external edit against the source that launched it

Use a private temporary directory and a format-appropriate extension, then hand off the local file through the desktop association. Offer Apply and Discard on a modeless session so the document remains usable. Process exit and file notifications cannot reliably tell Pictura that a desktop application has finished saving. Apply reads the current file once, validates the selected page, and stages pixels for every affected instance before committing.

Bind the session to the originating document, source identity, and source revision. Use the embedded record UUID for imported shared sources and ensure newly placed sources have an identity before they can be duplicated. Duplication that shares a source retains this identity, and New Smart Object via Copy assigns a distinct one. Persist identities through PSD/PSB writes and reads; for authored vectors the writer groups records by this identity, never by filename or payload equality. Distinct identities must remain distinct even when bytes and filenames match. Revalidate when Apply commits so a stale session cannot update a different object. Preserve per-instance geometry, masks, effects, and filter state. Leave the internal PSD/PSB editor on its existing Save path.

### Select Paste As destinations from the available payloads

Enable normal Edit > Paste when `application/pdf` is present even without an internal clip or system image, and snapshot the PDF before the existing image import can substitute a previous clip. Leave other paste command gates unchanged. Stage each result before changing the document. PDF supplies original bytes for Smart Object and a raster for Pixels; Paths and Shape Layer remain disabled. Editable Illustrator clipboard support is a separate follow-up, contingent on a redistributable captured fixture, verified MIME names, and a supported subset; do not infer curves from raster output.

### Repair the external record and preview together

Expose the external record's saved location as untrusted metadata. Enable repair for supported imported `liFE` records. Accept a replacement only when its detected format is the same as the recorded source format (PSD and PSB form one family; PDF and PDF-compatible AI are distinct formats; PNG, JPEG, and TIFF are each distinct) and its source dimensions and selected page/crop match the recorded values. Obtain original dimensions from the placed descriptor `Sz  ` (`Wdth` and `Hght`) and the selected page from `PgNm`; obtain crop from a recognized placed descriptor field only when its interpretation is verified. Never infer original dimensions from a transformed preview or read the old path. Refuse records when required fields are missing, ambiguous, inconsistent across shared instances, or an applicable crop cannot be decoded; refuse mismatches before mutation. Decode the selected file through the format-specific open/render adapters, then stage matching proxy updates for each layer using the source UUID. Patch the selected record location, filename, file type, and file size to match the replacement, and update every recognized timestamp and modification-time field when present. For variants with cached external data, replace it consistently only when its encoding is understood; otherwise refuse repair. Refuse any variant whose associated metadata cannot be safely updated. Preserve UUID, external kind, unrelated records, each layer's placed descriptor and unknown fields. Keep the record external and retain the refreshed pixels as the Photoshop-style preview. Leaving the codec's open path free of link reads means a saved repair can reopen and display its preview through existing channels, even if the replacement is gone.

## Risks / Trade-offs

- External applications can be absent or save by replacing the file. Surface launch failure and reopen the temporary path on each Apply.
- Rotated or scaled instances need per-instance output. Prepare the pixels at the committed size under the existing render limits and invalidate them when the source changes.
- A selection, tab, layer path, or source revision can change during a dialog or worker job. Revalidate document and source identity before one atomic history commit.
- External-record variants can contain fields Pictura cannot safely rewrite or lack reliable source dimensions. Refuse unsupported variants and preserve their bytes.
- Paste data can include unsupported painting operators or shapes. Advertise only destinations representable by the validated payload and refuse a failed conversion atomically.

## Migration Plan

No document migration is required. New embedded artwork uses the existing embedded source records and ordinary preview channels. A saved repair changes the selected external link and its previews. Undo restores both before saving. The implementation requires independent-reader fixture checks for embedded and repaired records.
