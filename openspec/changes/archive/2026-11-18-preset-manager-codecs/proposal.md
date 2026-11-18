# Proposal

## Why

Pictura has several built-in preset pickers and a persistent swatch list, but users lack a common way to manage reusable libraries or bring in presets from Photoshop. Issue #489 tracks that gap and the standalone color-settings codec.

## What Changes

- Add `Edit > Presets > Preset Manager` for Brushes, Swatches, Gradients, Styles, Patterns, Contours, Custom Shapes, and Tools. Support load/append, replace, reset, rename, delete, reorder, display modes, and Save Set for a selection or the whole list.
- Share each ordered library with its existing pickers and provide an application path for imported presets. Restore the current lists across restart and discover drop-in libraries in per-type preset directories.
- Include the effective current lists in portable settings backup and staged restore. Publish replacement lists together so a failed restore can continue with the previous sets.
- Add import for supported Photoshop variants of `.abr`, `.aco`, `.ase`, `.grd`, `.asl`, `.pat`, `.shc`, `.csh`, and `.tpl`. Provide lossless native Save Set and `.aco` and `.ase` export for supported swatch records. Document the supported versions, mapped fields, and export limits.
- Add the `.csf` color-settings codec and its tests. Issue #569 owns the Color Settings dialog, applying settings, and presenting import diagnostics. Curves `.acv`, earlier Photoshop preference migration, workgroup bundles, and standalone panel authoring workflows are separate work.

## Capabilities

### New Capabilities

- `codec/preset-libraries` describes library import and export, native sets, and standalone `.csf` color-settings interchange.
- `ui/preset-manager` describes the eight managed types, library editing and persistence, and integration with preset consumers.

### Modified Capabilities

- `ui/color-swatches-panel` moves authoritative swatch persistence to the native current-list store, with migration from the session palette.
- `ui/settings-backup` includes and restores the effective current lists, with support for older backups and retry of failed imports.

## Impact

`pictura-codec` gains format adapters and reuses descriptor and pattern primitives where the records match. `pictura-app` owns the application-wide libraries, the Qt dialog, settings migration, and the picker connections. Brush, shape, and pattern consumers gain a path for library payloads in addition to their existing built-ins. Imported assets applied to documents follow the existing document save path for effects, shapes, and pattern resources.
