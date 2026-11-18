# Proposal

## Why

Kooka Pictura users have to edit each version of a repeated design by hand. Issue #622 tracks the five documented Variables and Data Sets commands whose menu entries are already present.

## What Changes

- Define visibility, text replacement, and pixel replacement variables on layers. Link multiple layers to a named variable, and create, edit, rename, and delete data sets in the document.
- Import CSV and tab-delimited data with Photoshop's header, quoting, encoding, and naming rules. Preview a data set on the canvas and apply it as one undoable edit.
- Export selected sets as separate layered PSD files. Each file starts from the same template state.
- Read supported Photoshop-authored variable templates, and save and reopen Kooka-authored templates through PSD image resources `7000` and `7001`. Native Photoshop validation of Kooka-written templates is follow-up verification.

## Capabilities

### New Capabilities

- `document/variables-and-data-sets` covers bindings, typed values, and applying them to layers.
- `codec/psd-variable-resources` covers PSD and PSB persistence and opaque resource preservation.
- `interop/variable-data-set-io` covers delimited-text import and batch PSD export.
- `ui/variables-dialogs` covers command availability, data set editing, and preview.

### Modified Capabilities

## Impact

- `pictura-core` owns the document data. The existing engine layer operations and app edit transaction apply it. `pictura-codec` reads and writes the PSD resources.
- The Qt shell and Rust command catalogue wire the existing menu entries. The roadmap can tick the Variables and Data Sets As Files entries once the feature ships.
- Public Photoshop originals are local black-box references. Automated checks use independently generated fixture data.
