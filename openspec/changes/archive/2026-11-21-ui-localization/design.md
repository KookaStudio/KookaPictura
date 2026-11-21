# Design

## Context

The startup path in `main.cpp` applies pending settings before constructing
`PicturaMainWindow`. The typed Preferences store holds the language choice. Menu paths and tool metadata come from the Rust
catalogs through the Qt bridge, and the command registry uses the canonical
paths for lookup and separator placement. `NumericField` sets a C-locale
validator and parses its contents separately on commit and on arrow-key steps.
Ruler and Info formatting currently uses locale-neutral string conversions.

## Goals / Non-Goals

- Keep the canonical menu, tool, and numeric data as the source of truth for
  dispatch, saved files, and machine-readable output.
- Use one Qt presentation lookup for bundled and community translations.
- Apply the language at startup to cover widgets, models, and cached labels
  built during construction. Live retranslation and RTL interface mirroring
  are outside this change. [Live retranslation follow-up](https://github.com/KookaStudio/KookaPictura/issues/1501) · [RTL follow-up](https://github.com/KookaStudio/KookaPictura/issues/1488)

## Decisions

### Bundle one Qt catalog per language

Maintain German and Finnish Qt Linguist `.ts` files in the repository.
Use Qt LinguistTools to compile them into `.qm` resources at build time. Link
the resources once through the shell so they are registered in both the shared
test build and the static release build. Account for static resource retention
and exercise catalog lookup in the linked release executable. Embedded
resources keep the catalogs available across installed packages and portable
builds without translation sidecars. English uses the source strings.
[Qt Linguist CMake integration](https://doc.qt.io/qt-6/qtlinguist-cmake-qt-add-translations.html)

Use ordinary Qt translation contexts for C++ strings. Expose a deterministic
extraction input for command and tool text from the canonical Rust catalog
records. Translate those labels at their Qt display call sites, including
menu rebuilding, refreshed labels, toolbox flyouts, and tooltips. Keep
canonical paths, IDs, shortcuts, and user names intact. Locate the Open Recent
and Workspace menus by stable canonical identity, never by their translated
display titles, so their population and dispatch continue in either language.
Use disambiguation when identical source text needs different wording. Extract
marked Qt strings from production shell sources and headers. Convert visible
literals at their display sites, including Options bar pages, panel models,
tabs, and standalone dialogs. Store choices as typed values or item data and
translate their labels when building the control. Keep panel layout storage
keyed by object names. Use full phrases with placeholders for sentences that
contain dynamic values. Compare extracted messages with both bundled catalogs
to find missing translations.

### Resolve the language after settings and before widgets

Add a language key to the Interface section of `prefs.toml`. Store stable
values for System, English, Deutsch, Suomi, and app-managed import IDs.
Use System when no language setting exists. Read the startup choice after
initializing the profile and applying any pending settings restore.
For System, match the ordered Qt UI language preferences to German, Finnish,
or source English with regional fallback. For an explicit choice, use that
catalog or source English directly. Install any selected translator before
constructing the window and keep it alive for the life of the UI.

Add the selector and **Import Translation** to the Interface page. Use the
existing app file-picker helper and localized error presentation. On selection
commit, patch the language through the atomic Preferences store. Only a
confirmed save updates the pending selection and shows a restart notice. On
failure, restore the last saved choice in the selector and show an error. The
active language and translator stay unchanged through shutdown. If the
startup catalog is unavailable, fall back to System without rewriting the
saved choice. Settings backups carry the typed preferences. A restored imported
selection resolves against local app data and uses the same fallback when the
catalog is absent.

Keep the startup selection on the window. While the last saved selection
differs, show a Restart Pictura button beside the selector. Reuse the existing
save-before-restart checks and persist session layout before starting the new
process. Use the original AppImage when running from one. Quit only after a
successful launch.

### Import Qt Linguist source as bounded text data

Read one user-selected `.ts` file with Qt's streaming XML reader. Accept
finished singular context/source entries with optional disambiguation and a
valid target language. Ignore editor metadata, unfinished entries, and
unsupported message forms. Require usable entries and validate the complete
file before publishing it, with byte, entry, depth, and text-size bounds.
Allow Qt Linguist's bare `<!DOCTYPE TS>` declaration and reject entity or
external declarations and catalog dependencies. Reject conflicting duplicate
keys and handle malformed input before creating a selectable catalog. The
supported form can be produced with standard Qt Linguist authoring tools.
[Qt TS format](https://doc.qt.io/qt-6/linguist-ts-file-format.html)

Use a `QTranslator` subclass backed by the validated message table for a
selected community catalog. Report whether the table is empty and restrict its
lookups to messages also present in the bundled catalog. Newly covered contexts
accept plain community text and use source fallback for markup entries. Return a null string on no match so
Qt uses the English source. Render translated content as plain text at plain
text display sites, including labels that currently use AutoText; translator
input must not choose the text format. Where app-owned markup is intentional,
the application controls that markup and escapes translated text before
insertion, rather than treating catalog text as markup.
Bundle binary `.qm` only from the build pipeline. The text path avoids
loading downloaded binary data with Qt's catalog loader and keeps Linguist
an authoring and build tool. Community plural and ID-based entries can be
added later with explicit coverage.
[Qt translator lookup and catalog security](https://doc.qt.io/qt-6/qtranslator.html#security-considerations)

Publish a validated import atomically under an app-generated ID in the
existing per-user app-data location. Copy the catalog data so a Downloads
file or removable drive can disappear after import. Treat any restored local
import as input to the same validation path. Keep the selection separate
from import so a failed import or cancelled picker cannot change it.

### Keep the system numeric locale separate from UI language

Use a system-derived `QLocale` for UI numeric entry and display. Do not set
Qt's process default to the chosen interface language. Configure compact
readouts to omit group separators. Use the same locale in the shared field's
validator, typed-decimal detection, Enter and focus-out commits, and arrow-key
stepping. Reject grouped field input so a separator cannot be read as a
different number. Keep range checks, decimals, suffixes, and signals in their
existing control paths. Ensure the New Document dimension spin boxes use the
same system numeric locale for the inch-width round-trip.

Format ruler ticks and ruler measurement strings at their display boundary.
Apply the locale to Info measurements, ruler angle and length, channel values,
and document size text while preserving conversion, rounding, and geometry.
Format Info custom-scale values from their existing typed scale data, and
return a numeric calibrated ruler length to the options bar for locale-aware
display with its existing unit. Preserve canonical scale and measurement data.
The bridge, PSD, preset data, command parameters, and logs continue to receive
numbers or use their existing canonical formats.

### Keep the existing self-test deterministic

Run the existing self-test with English UI and C numeric locale even when the
launch environment requests other locales. Preserve its existing checks and
tokens. Cover localized startup in Qt Test, and verify the existing self-test
under a non-English UI and numeric environment without adding self-test tokens.

## Risks / Trade-offs

- Menu and tool labels originate outside `lupdate`'s usual C++ inputs. Check
  the extraction and rendered UI so new catalog strings do not silently revert
  to English on refresh.
- Imported catalogs can be partial or out of date. Missing and unfinished
  messages use English per lookup. Error handling keeps a damaged catalog
  from blocking startup.
- Qt may use environment-dependent locale defaults. Start locale integration
  tests in fresh Qt processes or use explicitly injected test locales so a
  previously initialized application cannot affect the result.

## Migration Plan

The language field is optional in existing profiles. The typed store migrates
the legacy language key when present. Roll out the compiled resources with the
app, then test bundled and imported activation across a restart and verify the
packaged executable can discover both bundled resources.
