# Proposal

## Why

Users can edit embedded PSD and PSB Smart Objects in Pictura, but cannot round-trip embedded vector artwork through a desktop editor, paste editable vector artwork from the clipboard, or repair a broken external link in an imported Photoshop document. This change addresses those parts of [#638](https://github.com/KookaStudio/KookaPictura/issues/638).

## What Changes

- Place PDF and PDF-compatible AI artwork as embedded Smart Objects. Retain the original bytes and selected page, render the artwork at the placed size, and export the source in its original format.
- Extend Edit Contents to launch the desktop-associated application for an embedded vector source. An explicit Apply commits the saved bytes and refreshed rendering to the instances sharing that source in the document, in one undoable edit.
- Offer Paste As for supported vector clipboard data. Enable Smart Object and Pixels for PDF data. Keep Paths and Shape Layer disabled until a separate follow-up establishes a redistributable Illustrator fixture and supported editable representation.
- Enable Relink to File for supported external records in imported PSD/PSB documents. Repair is an explicit post-CS6 interoperability extension and persists across save and reopen.
- The native Files panel is tracked in [#624](https://github.com/KookaStudio/KookaPictura/issues/624).

## Capabilities

### New Capabilities

- `interop/vector-smart-object-placement` covers placement, display, and source fidelity for embedded PDF and PDF-compatible AI.
- `interop/vector-paste-as` covers vector clipboard formats and paste destinations.
- `interop/smart-object-link-repair` covers explicit repair of supported imported external Smart Object links.

### Modified Capabilities

- `compositing/smart-object-layer-actions` extends Edit Contents eligibility and specifies the external editor session.
- `ui/layers-panel` extends row-level Edit Contents eligibility and enables Relink to File for eligible imported links.
- `codec/psd-smart-objects` specifies the selective codec write for a repaired external record.

## Impact

Changes touch PSD/PSB linked records, vector source rendering through the existing PDF import backend, clipboard dispatch, Smart Object commands, and document history. The existing Ghostscript backend and desktop association are runtime prerequisites for the relevant vector actions. No new dependency is planned. The implementation split is vector placement and editing, PDF clipboard integration, then imported link repair. Editable Illustrator clipboard destinations require a separate fixture-backed follow-up.
