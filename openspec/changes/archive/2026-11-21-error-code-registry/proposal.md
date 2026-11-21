# Proposal

## Why

Issue [#645](https://github.com/KookaStudio/KookaPictura/issues/645) tracks the missing stable operation error codes and panic boundaries. Native Open can lose its completion when a worker panics, filter apply can lose undo history, and the save and document-command bridges currently collapse some distinct failures into generic outcomes.

## What Changes

- Introduce a shared, append-only `pictura-error` registry, fixed-width `OpStatus` values, canonical message keys, and an unwind guard for Rust operation boundaries.
- Deliver structured status to Qt for native PSD/PSB Open and Save, the shared filter and adjustment commit worker, and shared whole-document commands in both synchronous and worker modes.
- Complete failed opens, preserve document and undo state on failed edits, and preserve the distinction between a save that failed before rename and one whose publication is uncertain after rename.
- Classify typed failures at the migrated boundaries and use the existing workspace `thiserror` dependency where touched library error types need to carry a source.

## Capabilities

### New Capabilities

- `interop/operation-status` defines stable status codes, canonical keys, typed error reporting, and the panic translation contract.

### Modified Capabilities

- `document/document-lifecycle` gains structured native Open completion and publication-aware native Save failures.
- `document/long-operation-progress` gains structured outcomes and rollback for synchronous and worker execution through the shared document command.
- `imaging/filter-dialog-preview` gains rollback and structured failure for the shared filter and adjustment commit worker.
- `interop/save-a-copy` distinguishes a pre-publication failure from uncertain native copy publication while preserving the source session.

## Impact

- Adds a Qt-free workspace crate. Affects the native codec error paths, cxx-qt bridge declarations, the Qt command adapters, and existing Qt Test suites.
- The migration covers staged operation work and its completion in the listed paths. Specialized filter paths, layer-mask operations, other file formats, and unrelated bridge calls are outside this change. The plug-in SDK and loader are tracked in [#623](https://github.com/KookaStudio/KookaPictura/issues/623).
