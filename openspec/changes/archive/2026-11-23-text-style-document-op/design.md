# Design

## Context

See `proposal.md` - Why. Three owners currently share the invariants of a named
style edit:

- `pictura_core::TextStyleSheet` validates names and mutates the sheet only
  (`pictura-core/src/type_tool.rs:526-650`).
- `pictura-app::history::styles` mutates the documents' layers and re-resolves
  content (`crates/pictura-app/src/history/styles.rs`), but a paragraph rename is
  never routed here; the panel composes create and delete instead.
- The C++ panels call the `type_api` bridge, which records history through
  `commit_style`.

`pictura-render/src/type_layer.rs` already owns the document-level type-layer
primitives (`apply_type_style`, `replace_type_layer`) but sits at 1189 lines
against a 1200 hard cap, so the new operation needs its own module.

The archived `paragraph-styles-panel` design accepted "Rename as delete-then-create";
this change supersedes that decision (see Decisions).

## Goals / Non-Goals

**Goals:**

- One document-level edit operation that both style kinds and both panel adapters
  call, owning validation, rename, attribute write, reference rewrite and
  re-resolution.
- Refusal that is observably empty: no sheet change, no reference change, no
  content change, no history state.
- One history state for an accepted rename plus attribute change, using the
  existing snapshot mechanism.

**Non-Goals:**

- No change to style precedence, manual overrides, `Basic Paragraph` protection,
  or the dialog's Preview/Cancel behavior.
- No change to create or delete, which are already single-call operations.
- No new persisted format, no `.psd` or documentation change.
- No user-visible error dialog; the operation returns failure to its caller, and
  the panel stops claiming success.
- No change to whether an accepted edit that changes nothing records a state;
  the current edit path already records one, and this change does not alter that.

## Decisions

### A new `pictura-render::named_style` module owns the document operation

`edit_named_style(doc: &mut Document, edit: NamedStyleEdit<'_>) -> Result<(), StyleError>`
renames when needed, writes attributes, rewrites every applying layer reference
(recursing groups), then re-resolves those layers. It re-exports from `lib.rs`
as `pictura_render::edit_named_style`.

Alternatives considered:

- **Deepen `pictura-app/src/history/styles.rs` in place.** Smallest diff, but
  leaves the model/layer invariants owned by the history adapter - the split this
  change exists to remove - and the same helpers would remain unreachable from
  the document layer.
- **Put it in `pictura-core`.** The sheet can validate, but it cannot touch
  applied layer references or re-render, so it cannot be the complete operation.
- **Extend `type_layer.rs`.** At 1189 lines it cannot absorb the operation under
  the hard cap; a sibling module is required either way.
- **Name it `type_style`.** Collides with the existing glyph-styling module
  (`pictura-render/src/type_style.rs`); `named_style` names the saved-style sheet
  concept.

### The request type makes kind and attributes inseparable

```rust
pub enum NamedStyleAttrs {
    Character(CharacterOverrides),
    Paragraph {
        character: CharacterOverrides,
        paragraph: ParagraphOverrides,
    },
}

pub struct NamedStyleEdit<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub attrs: NamedStyleAttrs,
}
```

A separate `kind: StyleKind` plus two attribute fields would let a caller pass
paragraph attributes for a character style and would leave `From`/`To`
protection checks dependent on remembering `kind`. Pairing attributes with the
kind removes that class of mistake and keeps the one `from == to` path used by
both plain edits and live previews. `NamedStyleAttrs` lives in `pictura-core`
beside `edit_style` and joins the `pictura_core` re-export list so
`pictura-render` can name it.

### Validate in core, mutate once

`TextStyleSheet::edit_style(from, to, attrs)` performs one validation pass and
then the rename and attribute write:

1. source exists (`StyleError::NotFound`),
2. paragraph kind only: neither `from` nor a differing `to` is `Basic Paragraph`
   (`StyleError::Protected`),
3. if `from != to`, target is free (`StyleError::Duplicate`).

The order fixes the error identity: a rename of another paragraph style to
`Basic Paragraph` returns `Protected`, not `Duplicate`, matching
`rename_paragraph_style` and the existing sheet tests. Every refusal still
leaves the sheet byte-identical. Same-name editing of `Basic Paragraph` must
remain accepted; only a rename into or out of it is protected. The sheet method
is infallible after validation, so the render operation can rely on "Err means
nothing changed".

Alternatives considered: a public validate/apply pair. It doubles the API for
one caller and lets future callers apply without validating.

### Reference rewrite before re-resolution

For a rename, the layer's `applied_character_style` / `applied_paragraph_style`
must be rewritten to the target name before re-resolving, otherwise resolution
looks up the old name and fails after the sheet rename. Both traversals already
exist as private recursion in `history/styles.rs`; they move behind the
operation (rename applied references, then re-resolve applied layers, preserving
each layer's manual overrides through the existing
`TextStyleSheet::resolve` + `replace_type_layer` path). Type layers inside groups
are covered by the same recursion and by path resolution. Re-resolution is
guarded by `pictura_render::type_layers_editable`: where the document's type
layers cannot be re-set (not 8-bit RGB), the sheet and reference changes still
apply and the rendered content is left as it was.

Editing `Basic Paragraph` under its own name is the exception: every layer that
applies some character or paragraph style inherits `Basic Paragraph` as its base,
so the operation re-resolves every type layer carrying an applied-style
reference, not just the layers naming it. A type layer with no applied style is
deliberately left alone: its authored attributes are local formatting and are not
stored as `type_overrides`, so re-resolving it would clobber them.

### Bridge and history adapter

`type_edit_paragraph_style` and `type_api::editParagraphStyle` gain a target-name
argument. `type_edit_character_style` keeps its shape. Both bridge functions call
`pictura_render::edit_named_style` once inside `commit_style`, so success records
one "Edit Character Style" or "Edit Paragraph Style" state and failure records
none. Live preview continues to call the same operation with `from == to` under
`preview_style`, which records nothing.

`history::create_*` and `history::delete_*` stay as they are.
`history::edit_paragraph_style` gains a target name
(`edit_paragraph_style(doc, from, to, character, paragraph)`), mirroring the
character helper's `new_name`, so the wrapper can delegate to
`edit_named_style`; the `crates/pictura-app/src/history.rs` re-exports follow,
and the live-preview caller at `type_tools/imp.rs` passes `name` twice.
`history` keeps thin wrappers for the existing character tests
(`renaming_an_applied_style_keeps_it_applied`,
`a_style_operation_records_one_undoable_state_each`) and gains paragraph
equivalents and rejection tests.

### Panel refusal restores the dialog's opening values

`ParagraphStylesPanel::editStyle` becomes one `editParagraphStyle` call and
returns its result instead of always returning true. Preview may already have
written the dialog's attributes into the old style with no history state, so on
a refused accept the panel re-previews the opening family and attribute sets
under the original name (the same path Cancel uses) before returning false. That
leaves the sheet, references, pixels and history as they were when the dialog
opened, matching the refusal requirement. The character panel receives the same
restore-on-refusal treatment for consistency; its accepted rename path already
behaves correctly.

No error dialog is added: the spec requires the panel to return failure to its
caller, and the double-click handler discards the bool today. A visible message
can be added later without changing the operation or the specs.

### Superseding the archived design

The archived `paragraph-styles-panel` design chose delete-then-create because
"a rename is rare and the outcome is correct". The outcome was not correct:
create failure is ignored and the delete destroys the applied style. This
design replaces that decision with the validated single operation; the archived
document stays as the record of the earlier choice.

## Risks / Trade-offs

- **A previewed rename refusal could leave preview attributes behind** -> the
  panel re-previews the opening values on refusal; the old name still exists at
  that point, so the preview call is valid.
- **Ordering bugs between sheet rename and reference rewrite** -> the operation
  owns both; tests assert that a later edit to the renamed style reaches a layer
  nested in a group.
- **`Basic Paragraph` attribute edits regress into Protected** -> validation
  distinguishes "rename into/out of Basic Paragraph" from "edit under its own
  name"; a core test covers both.
- **Moving the traversal helpers leaves app code reaching into render internals**
  -> only the operation is public; the helpers stay private to the new module.
- **Error identity is lost at the C++ boundary** -> the bridge keeps returning
  bool per the existing API; Rust tests assert the `StyleError` variant.

## Migration Plan

No persisted data or file-format change, so no migration or rollback plan. The
app-internal C++/Rust bridge signature change is compiled together and has no
external consumers.

## Open Questions

- Whether a refused rename should also raise a user-visible message is a UX
  follow-up; it does not change the operation, the specs, or these tasks.
