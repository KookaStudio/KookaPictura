# Proposal

## Why

Renaming a paragraph style from the panel is assembled in the panel itself: it
calls create-with-the-new-name and then delete-with-the-old-name and ignores both
results (`crates/pictura-app/cpp/panels/paragraph_styles_panel.cpp:201-207`). When
the create is refused (for example the target name already exists), the delete
still runs, so the original style and every applied reference to it are destroyed
— a rename that fails must not delete anything. The character panel uses a
different, rename-aware path, but it too sequences two mutations in the bridge,
and the underlying sheet already validates names (`pictura-core` rejects
duplicate, missing and `Basic Paragraph` renames) without owning the document's
applied references or their re-resolution.

The style sheet, the applied references on layers (including layers nested in
groups), and the re-resolved type content are one invariant. Today that invariant
is split between a C++ panel, a Rust bridge function and a history adapter, so
neither caller can be trusted to keep it.

## What Changes

- Add one document-level named-style edit operation that takes the source name,
  the target name, and the new character/paragraph attributes, validates the
  complete request, then changes the style sheet, rewrites every applying layer's
  stored reference (recursing into groups), and re-resolves those layers.
  A refused request changes nothing and returns an error.
- Add a validated sheet-level primitive in `pictura-core` that performs an
  optional rename plus the attribute write as one step, so document callers
  cannot observe or sequence a half-applied edit.
- Route both character and paragraph accepted edits through the document
  operation; the history adapter wraps it in exactly one history state.
- Add the target name to the paragraph-style bridge API so a paragraph rename no
  longer needs create/delete composition.
- Replace the paragraph panel's create/delete rename sequence with one call and
  make it report the operation's failure. On a refused rename, restore the
  dialog's opening attributes (the existing preview path) so pixels and history
  are as they were before the dialog opened.
- Keep style precedence, manual overrides, protected-name rules, and the
  dialog's Preview/Cancel behavior unchanged.
- Add paragraph rename, rejection, applied-reference and one-undo coverage at
  the Rust and Qt Test layers.

The bridge signature change (`type_edit_paragraph_style` and
`type_api::editParagraphStyle` gain a target name) is internal to the app and is
not a document or file-format change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/type-style-model`: add a requirement that a named character or
  paragraph style edit — optional rename plus new attribute sets — is one
  validated document operation that refuses the whole request (duplicate name,
  missing source, protected `Basic Paragraph`) without changing the sheet,
  applied references, pixels or history, and that an accepted rename keeps every
  applying layer's reference, including inside groups, and records one undoable
  state.
- `ui/paragraph-styles-panel`: extend the "manages styles" requirement so the
  options dialog can rename a style, the rename preserves applied references and
  records one history state, and a refused rename leaves the panel, the style and
  its applying layers unchanged.

## Impact

- `crates/pictura-core/src/type_tool.rs`: validated named-style edit primitive on
  `TextStyleSheet` and its error identity.
- `crates/pictura-render/`: new `named_style` module with the document
  operation; re-export from `lib.rs`.
- `crates/pictura-app/src/history/styles.rs`: character and paragraph edit
  helpers delegate to the document operation; delete/create stay single calls.
- `crates/pictura-app/src/cxxqt_object/type_tools.rs` and `type_tools/imp.rs`:
  paragraph edit gains a target name; both kinds call the document operation
  under one `commit_style`.
- `crates/pictura-app/cpp/api/type_api.{h,cpp}`,
  `crates/pictura-app/cpp/panels/paragraph_styles_panel.cpp` and
  `crates/pictura-app/cpp/panels/character_styles_panel.cpp`: one-call rename,
  failure propagation, opening-value restore on refusal.
- Tests: `crates/pictura-app/src/history/tests.rs`,
  `crates/pictura-app/cpp/tests/tst_paragraph_styles_panel.cpp`, plus document
  operation tests in `pictura-render`.
- No new dependencies. No `.psd` format or docs change.
