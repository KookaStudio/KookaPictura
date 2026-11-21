# Proposal

## Why

Document alpha channels are plain grayscale planes: the model has no idea whether a plane stores selected coverage or protected (masked) coverage. Photoshop stores that choice per channel as the DisplayInfo kind (PSD image resource 1077), and `pictura-codec` already parses the resource, but only the PDF spot export consumes it. As a result, a PSD whose channel is in `Selected Areas` loads the complementary selection, and the Channels panel's `Channel Options…` entry stays disabled. This is one of the ceilings recorded by the save/load-selection work (#615; the remaining ceilings are 16/32-bit channels #1331 and the Load-dialog colour-row list #1333) and the docs↔code parity gap SEL-3 (#611).

## What Changes

- Model a per-channel sense: `Masked Areas` (default; plane = selection coverage, white = selected) and `Selected Areas` (plane stores protection, so selection = `255 - plane`), tracking PSD DisplayInfo `kind` 1 and 0. A missing entry means `Masked Areas`, preserving every existing document and file.
- Make selection conversion sense-aware: loading, saving, combining, and the coverage export used by channel drags all derive and write canonical coverage through the channel's sense.
- Add a `Channel Options…` dialog for alpha channels, offering `Masked Areas` / `Selected Areas`. It is enabled only when an alpha row is current (spot rows are excluded), opens from the Channels panel menu and on double-clicking an alpha row, flips the channel plane and its stored kind together so the mask is unchanged, and records one `Channel Options` history state.
- Keep DisplayInfo entries aligned with the extra-channel order on add, delete, and duplicate, and round-trip the sense through PSD while leaving names, spot entries, and other resources untouched.
- Out of scope: spot-channel conversion, the Name/Color/Opacity fields, the New Channel dialog, painting directly on a channel, Split/Merge (#513), and preserving the sense through Duplicate Channel / cross-document drags (those keep the same selection but land as `Masked Areas`).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/selection-channels`: channel sense semantics, sense-aware selection conversion, and PSD DisplayInfo round-trip.
- `ui/channels-panel`: the enabled `Channel Options…` affordance, double-click entry point, and the sense-aware channel thumbnail/overlay wording.

## Impact

- `crates/pictura-codec/src/image_resources.rs`: sense constants, accessors, and DisplayInfo entry maintenance alongside the existing alpha-name helpers; `crates/pictura-codec/src/pdf/mod.rs` and `pdf/tests/mod.rs`: explicit Masked default for missing entries.
- `crates/pictura-render/src/document_ops/mode/flat.rs`: clear DisplayInfo when a mode conversion clears the extra channels.
- `crates/pictura-app/src/cxxqt_object/impl_selection/` (`save_load.rs`, `exports.rs`): sense-aware coverage derivation and write-back.
- `crates/pictura-app/src/cxxqt_object/channels.rs` and `channels/imp.rs`: read/write bridge, spot predicate, set-sense operation, entry alignment on New/Delete.
- `crates/pictura-app/cpp/panels/channels_panel.{h,cpp}`, `panels/panel_group_menu.cpp`, `menu_panel_commands.cpp`, `menu_registrar.h`, `panel_host.h`, `panel_host_build.cpp`, `frame.{h,cpp}`, `frame_selection_dialogs.cpp`: menu/panel wiring, spot-row exclusion, and the dialog.
- `crates/pictura-script/src/command_catalogue/{const_names.rs,entries.rs}`, `crates/pictura-script/tests/golden/{const_names.tsv,catalogue.tsv}`, and the generated `command_ids.h`: one new panel command id and catalogue spec.
- Tests: Rust codec/app/render unit tests, the PDF preserve-editing test, `tst_channels_panel`, `tst_channel_options`, `tst_selection_save_load`, and a psd-tools oracle check for the written mode.
- Docs (separate `TASK-ALLOWS-DOCS` commit): parity gap SEL-3 and SEL-7, PAN-5, `ROADMAP.md`, `docs/dev/m42-panel-menus.md`, and the resolved PSD-round-trip open questions in `docs/08-selection/channel-based-masking.md` and `docs/08-selection/save-and-load-selections.md`.
