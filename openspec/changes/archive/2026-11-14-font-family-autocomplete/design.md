# Design

## Context

See `proposal.md` - Why. The family inputs are plain `QFontComboBox` instances
that all call `useFastFontPreviews()` in `crates/pictura-app/cpp/font_combo.cpp`,
which currently replaces only the item delegate: the Character, Paragraph Style,
and Glyphs panels each own one (`characterFamily`, `paragraphStyleFamily`,
`glyphsFamily`), and the Type options bar builds one `optionsTypeFamily` combo
per Type tool page (four instances, one visible at a time).

Qt 6.11's default completer for an editable `QComboBox` is
`Qt::CaseInsensitive` with `InlineCompletion`, and `QFontComboBox` does not
override it. A scratch probe against the same Qt confirmed that typing `noto`
into an empty or selected field matches `Noto Sans` case-insensitively. Two
other behaviors cause the reported failure:

1. When the field shows an existing family, typing appends to it. The inline
   completer uses the whole line-edit text as its prefix, so
   `Liberation Sansnoto` matches nothing.
2. The Type options bar's `sync()` and the Character panel's `refresh()` both
   call `setCurrentFont(QFont(family))` on a family change. The options bar sync
   runs synchronously on `typeOptionsChanged`; the Character panel's own typing
   suppresses its synchronous refresh through `applying_`, so its guard matters
   on the deferred (panel-timer) refresh, where the `setCurrentFont` call would
   clear the inline completion selection. A `QFont`
   built from just a family differs from the combo's stored font in size and
   resolve bits, so `QFontComboBox::updateModel()` runs, selects a model row,
   and rewrites the line edit - usually to the first family in the list because
   the partial prefix matches no family exactly. This destroys the typed prefix
   before the completer can use it.

## Goals / Non-Goals

**Goals:**

- Typing into any family field starts a new search, so a typed prefix replaces
  the shown family and case-insensitive completion works.
- Option/session writes stop interrupting an in-progress prefix.
- One shared implementation for all four fields; selecting from the popup is
  unchanged.

**Non-Goals:**

- Changing the family list to a filtered popup (`PopupCompletion`). The
  documented CS6 behavior is type-ahead to the first matching family.
- Adding a font search/filter UI, font-preview changes, or changes to the font
  style/size fields.
- Changing stored families, PSD data, or font registration.

## Decisions

### D1. Put the typing behavior in the shared font-combo helper

`useFastFontPreviews()` is the only code all four family fields already share.
Add a small event filter there (owned by the combo, installed on the combo and
its line edit) instead of touching four panels. Alternative: a `FontComboBox`
subclass - more files and a widget-class change for a behavior local to these
four fields.

### D2. Only the first printable key replaces the field

Track a per-focus "fresh search" state:

- armed on focus-in, `activated`, `editingFinished`, and `currentIndexChanged`;
- a printable key with no Ctrl/Alt/Meta while armed selects all first, then
  lets the key replace the selection and disarms the search;
- only caret-movement and editing keys (arrows, Home/End, PageUp/PageDown,
  Backspace/Delete) disarm the search without replacing, so a user can still
  position the caret and edit a name precisely; modifier keys (Ctrl, Alt, Meta,
  Caps Lock, Num Lock) do not disarm it;
- an input-method commit while armed selects all first, so an IME's first
  commit replaces the shown family; `textEdited` disarms, covering paste.

The navigation/deletion escape hatch is covered by `ffp_nav_keeps_text`.

Alternatives: select-all on focus - does not survive the mouse press that
positions the caret; select-all on every click - breaks placing the caret to
edit. Arming on `currentIndexChanged` re-arms after an external family change or
a popup pick without re-arming on each typed key (typing does not move the
combo's current index).

### D3. Guard the per-keystroke family writes

In `options_bar_type.cpp`'s `sync()` and `character_panel.cpp`'s `refresh()`,
skip `setCurrentFont` when `family->currentText()` already equals the target
family. The guard suppresses only same-family writes; a different-family external
write still differs from `currentText()` and updates the field, so the narrowed
spec requires only that the same-family sync not disturb typing. The guard also
stops the feedback loop (`currentText()` is exactly what the sync wrote). The
existing `QSignalBlocker` stays; it prevents signal recursion but not the
model/line-edit reset. The Glyphs panel's `followTypeFont` is a third
`typeOptionsChanged` write, gated by `familyChosenHere_`; it never targets the
field being typed and stays unchanged.

### D4. Rely on Qt's documented case-insensitive default, test it

No production line sets `Qt::CaseInsensitive`; the completion behavior is
already the Qt default and a test asserts it. If a supported Qt changes the
default, the test fails and the one-line `completer()->setCaseSensitivity()`
can be added at that point. Adding it now would be unverifiable code.

### D5. Tests assert the completion set, then a distinguishing prefix

After typing `noto`, inline completion selects the first matching family, which
can be `Noto Color Emoji` depending on installed fonts. The test asserts that
the completer's match set contains `Noto Sans`, then types ` s` and asserts the
field selects `Noto Sans`. This proves case-insensitive offering without
depending on the machine's font ordering.

## Risks / Trade-offs

- Typing a letter when the user wanted to insert it mid-name replaces the whole
  family. A user editing part of a name can press an arrow/Home/End or
  Backspace first; those keys disarm replace-on-type (`ffp_nav_keeps_text`).
- `currentText()` comparison assumes an editable combo (all four family inputs
  are). A future non-editable family combo would need a different guard.
- The new tests hard-assert the bundled `Noto Sans` is installed, as
  `tst_character_paragraph_panels` already does; the bundled UI family makes it
  present in the app build.
- The fresh-search state is per-widget and not persisted; no migration needed.

## Migration Plan

None - UI behavior only, no data or settings change. Rollback is reverting the
change.
