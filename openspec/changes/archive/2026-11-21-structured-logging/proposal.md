# Proposal

## Why

Kooka Pictura needs persistent diagnostics to investigate failures across Rust and Qt. Issue #643 also calls for the CS6 History Log so users can keep a record of their editing sessions and changes. [Issue #643](https://github.com/KookaStudio/KookaPictura/issues/643)

## What Changes

- Add structured Rust diagnostics and a Qt message bridge to a shared subscriber. Write reviewed diagnostic fields to a rotating local file and keep stderr available when the file sink fails.
- Add an opt-in History Log under General preferences, with Metadata, Text File, and Both destinations and Sessions Only, Concise, and Detailed verbosity.
- Store metadata history as `photoshop:History` in XMP and preserve it across save and reopen for PSD, PSB, JPEG, and TIFF. Show the stored history in File Info.
- Carry History Log into Save for Web JPEG and PNG when the selected Metadata option includes the full XMP packet.

## Capabilities

### New Capabilities

- `verification/structured-logging` for persistent application diagnostics and Qt-to-Rust message routing.
- `document/history-log` for user-controlled edit provenance and text and XMP persistence.

### Modified Capabilities

## Impact

The change touches app startup, the Qt/Rust bridge, document sessions, General preferences, File Info, save and open paths, and codec XMP handling. Rust needs `tracing` for typed events, `tracing-subscriber` for filtering and output layers, and an app-owned bounded writer for buffered rotation. The existing Save for Web metadata policy also applies to the History Log property.
