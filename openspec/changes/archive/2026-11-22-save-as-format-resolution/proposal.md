# Proposal

## Why

Save As format identity is re-encoded in roughly twenty places. The filter order
is one table, the filter-position-to-kind mapping is a second switch, the
canonical-suffix mapping a third, and suffix aliases are written independently
in the shell, in the app bridge, and in the codec. Adding or moving a format
means editing several switches that must agree. The integer wire codes cross the
C++/Rust seam, and the remembered settings persist as string keys
(`saveFormatOptions`) plus the action-step `as` suffix, so a silent disagreement
is costly. EPS and DCS show why a suffix alone cannot carry the decision: both
are `.eps`, distinguished only by filter hint or remembered kind.

Issue #1395 asks for one authoritative format module so Save, Save As, Save a
Copy, and their tests consume a single format decision. It is a pure refactor:
no format is added, removed, or reordered, and no observable save behavior
changes.

## What Changes

- **Add one format catalog in `pictura-codec`.** A new `SaveFormat` layer (14
  Save As formats) owns each format's stable ordinal id (0..13), name, wire
  code, suffix aliases, canonical extension, writer route, and the policy flags
  callers need (native PSD/PSB, PDF, raster, keeps layers, offers profile).
  `SaveKind` remains the wire-code set the dispatch recognizes.
- **Make the shell table presentation only.** One ordered table in the shell
  carries the label, name-filter pattern, canonical suffix, and a reference to a
  catalog row. Every position, suffix, and policy decision becomes a lookup
  against that table plus the catalog instead of its own switch:
  filter position → format, format → default filter index, format → canonical
  suffix, suffix → options kind, source format → filter index, and the
  profile-prompt, layer-retention, and feature-warning predicates.
- **Delete the parallel decisions.** `SaveKind::from_suffix`,
  `raster_writer_for_suffix`, the EPS override in the Rust dispatcher (replaced
  by the shell-supplied id), the C++ `saveFormatKindForFilter` /
  `defaultSuffixForFilter` / `defaultSaveFilterIndex` switches, the two
  independent suffix lists (`isKnownImageSuffix`, `isRasterImageSuffix`), and
  the duplicated native-path helper are removed in favor of catalog lookups.
- **Hold the All Formats filter to the table.** The aggregate entry stays a
  single display string, with a check that its patterns equal the union of the
  catalog aliases queried through the bridge, so it cannot drift from the
  formats it advertises.
- **Pass the resolved format to the save edges.** The sync and native save
  bridges receive the resolved ordinal id explicitly. The coordinator chooses
  the native route from that id when supplied, and the native worker resolves the
  same id to choose PSD versus PSB. Plain Save resolves from the remembered
  per-document kind (`lastSaveKinds_`); Save a Copy resolves from the chosen
  filter hint, falling back to the suffix. The Rust dispatcher no longer reads
  `save_options.kind` for routing (it still uses it to select the options
  blob).
- **Keep writer routing where the writers live.** The route each format takes
  (native PSD/PSB worker, PDF export, GIF encoder, engine writer, Qt raster
  writer, refusal) moves into the catalog, and the document save dispatches on
  it. Encoders and option dialogs are untouched.
- **Preserve everything the issue pins.** Filter ordering and labels, existing
  integer wire codes and stored settings, case-insensitive aliases, All Formats
  behavior, chosen-format precedence over a typed suffix, refusal of missing and
  unknown suffixes at the existing entry points, EPS/DCS disambiguation, the
  plain-Save remembered kind, the PSD/PSB and PDF routes, the JPEG/Qt writer
  route, TIFF layer retention, GIF handling, and the existing profile policy.

## Capabilities

No spec delta: this is an internal seam refactor with no requirement change
(`skip_specs`). The behavior it must preserve is already governed by
`interop/save-format-options`, `interop/save-a-copy`, and
`codec/raster-save-encoders`.

## Impact

- **Codec (`pictura-codec`):** a new `save::format` catalog with its unit tests;
  `save/mod.rs` keeps the encoder dispatch and loses `from_suffix`, the unused
  `is_engine_writer` helper, and the duplicated alias match.
- **App (Rust):** `impl_core/save.rs` dispatches on the catalog route and takes
  an explicit resolved id on the sync save edge; `impl_core/save_jobs` resolves
  the same id for the native worker and its PSD/PSB container;
  `impl_core.rs::raster_writer_for_suffix` is removed; `save_format/imp.rs`
  implements the six read-only catalog queries and `export.rs` exposes them to
  the shell.
- **App (C++):** `save_format_options.{h,cpp}` owns the display table and the
  pure format helpers; `dialogs.cpp` and `dialogs.h` lose their literal suffix
  lists and position switches and keep only the view-dependent policies as
  table lookups; `import_save_save.cpp`, `import_save_open.cpp`,
  `import_save_pdf.cpp` (`isPdfPath`), and `control_server_documents.cpp` (the
  remembered-DCS pre-check) derive the profile prompt, native route, PDF route,
  remembered-format index, and DCS pre-check from the catalog.
- **Tests:** existing `tst_open_save::chosenFormatWinsOverTypedSuffix` and the
  named `tst_save_format_options` cases keep passing unchanged; new Rust catalog
  and bridge tests, a codec route-matrix test, and Qt table cases that tie each
  displayed format to its path, kind, and writer route.
- **Docs, dependencies, assets:** none. No new dependency, no golden-baseline
  change, and no `docs/` edit.
