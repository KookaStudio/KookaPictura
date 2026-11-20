# Design

## Context

See `proposal.md` — Why. The relevant current state:

- `pictura_core::Channel { id, data }` is shared by layer channels and document extra channels (`crates/pictura-core/src/lib.rs:631`). Layer channels are the overwhelming majority of its construction sites.
- `Selection::to_channel` / `from_channel` treat the plane as selection coverage (`crates/pictura-select/src/lib.rs:379`), and the app saves/loads through them (`impl_selection/save_load.rs`).
- The codec already parses, preserves, and rewrites PSD DisplayInfo (resource 1077) as `ChannelDisplayInfo { color_space, components, opacity, mode }` (`crates/pictura-codec/src/image_resources.rs:201`). A real Photoshop 24.7 file uses a `u32` version followed by 13-byte entries; the parser and encoder match that.
- The codec's alpha-name helpers (`alpha_names` / `set_alpha_names`) already follow the pattern of keeping a resource aligned with `Document.channels`.
- The Channels panel's row thumbnail is the raw plane, and the alpha-eye overlay already tints `255 - plane` (`frame_mask_overlay.cpp:74`).
- The `Channel Options…` menu item is transcribed but disabled (`panels/panel_group_menu.cpp:117`); the panel has no double-click handler.

## Goals / Non-Goals

**Goals:**

- One source of truth for the sense: the document's DisplayInfo resource, interpreted through small codec helpers.
- Sense-aware selection conversion everywhere a channel becomes or receives a selection.
- A minimal Channel Options dialog with exactly the two `Color Indicates` options.
- PSD round-trip without changing existing bytes until the user edits a channel.

**Non-Goals:**

- Spot-channel conversion, Split/Merge, or a full spot-vs-alpha row model (the command only needs a spot predicate).
- Renaming, per-channel overlay colour/opacity editing, or the New Channel dialog.
- Painting or filtering directly into a channel as an edit target.
- Preserving the sense through Duplicate Channel or cross-document drags (the selection is preserved; the destination channel is `Masked Areas`).

## Decisions

### Store the sense in DisplayInfo, not in a Rust struct

Add codec helpers around the existing resource instead of a new field:

- `MASKED_AREAS_MODE: u8 = 1` and `SELECTED_AREAS_MODE: u8 = 0` (with `SPOT_CHANNEL_MODE = 2` already present).
- `channel_selected_areas(doc, index) -> bool`: true only when the entry's mode is 0. A missing entry and a spot entry return false.
- `set_channel_selected_areas(doc, index, selected) -> bool`: pads the entry list to `doc.channels.len()` with explicit `MASKED_AREAS_MODE` entries when a resource is present, then writes only that entry's `mode`, preserving colour, opacity, and every other block. `ChannelDisplayInfo::default()` must never be used as a sense default: its mode is 0, which is `Selected Areas`. Refuses a spot entry.
- `remove_channel_display_info(doc, index)`: removes and truncates when entries exist.
- `append_channel_display_info(doc)`: extends an existing list by one explicit `MASKED_AREAS_MODE` entry.

Why not a field:

- `Channel` is shared with layer channels; adding `sense` would touch hundreds of literals and give layer channels a meaningless field.
- `Document` has hundreds of literals too, and a parallel vector would have to be shifted by every resize/crop/rotate/recovery path.
- The PSD file itself stores the sense in the resource, so resource-backed helpers keep file and model from diverging.
- `image_resources` is already preserved across resize, crop, rotate, recovery, and history snapshots.

Byte stability: when no DisplayInfo resource exists and no sense is changed, nothing is written. When a resource already exists, the list is padded/extended with Masked entries; when a channel is explicitly switched to `Selected Areas`, its entry is created. Existing documents therefore re-save byte-identically until edited.

### Mapping and the visual model

| stored kind | sense | plane | load |
|---|---|---|---|
| 1 (protected) | Masked Areas (default) | selection coverage | plane |
| 0 (selected) | Selected Areas | protection = `255 - coverage` | `255 - plane` |
| 2 | spot | ink | not a selection |

The direction is chosen from the Photoshop SDK's DisplayInfo comment (`0=selected, 1=protected`), Adobe's descriptions of the two `Color Indicates` modes (`Masked Areas` = masked black / selected white; `Selected Areas` = masked white / selected black), and a Photoshop 24.7-authored PSD in the psd-tools fixtures whose kind-1 channel stores white-on-black text. The manual Photoshop CC/CS6 check in `test-plan.md` is the oracle; the automated scenarios encode this mapping as regression tests and cannot prove it. Task 1.1 records the observed kind bytes as the post-implementation Photoshop check; the fixture-based mapping is already implemented.

`Selected Areas` is a display-and-storage inversion, not a different mask: toggling the option complements the plane and flips the kind, so the loaded selection is unchanged. This matches the Adobe help statement that both modes create identical masks and reverse the visual representation.

The panel thumbnail and the alpha-eye rubylith both show/tint the stored plane, so complementing the plane moves them to the areas the new mode indicates. The refresh call is new, and so is invalidating the overlay cache (below).

### Sense-aware conversion points

- `selection_from_target` (app): derive via `Selection::from_channel`, then `.invert()` when the channel is `Selected Areas`. This one function feeds Ctrl-click loads, the Load Selection dialog, the channel drag payload, and `channel_coverage`.
- `save_selection_shape`:
  - `NewChannel`: unchanged write of canonical coverage (new channels are Masked); extend an existing DisplayInfo list with a Masked entry.
  - `Channel(index)`: derive the existing coverage through its sense, combine in canonical coverage, then write back through the same sense (`result.invert()` before encoding for Selected).
- `channels_set_sense` (new app bridge function): one `Edit::Meta` named `Channel Options`; if the requested sense equals the current one return `Edit::None`; otherwise check eligibility, complement the plane bytes in place, and set the kind. The eligibility check happens before any mutation, so a refused spot entry never leaves a complemented plane. The existing `edit` transaction snapshots the document, so the plane and the resource undo together.
- Delete: remove the name and the DisplayInfo entry at the same index so later channels stay aligned.

Two paths outside the channel commands must keep the resource aligned. PDF preserve-editing (`crates/pictura-codec/src/pdf/mod.rs:288`) must default a missing entry to an explicit Masked entry, not `ChannelDisplayInfo::default()`. Any path that clears `Document.channels` must clear its display info with it: `convert_to_bitmap` currently clears the channels and names only (`crates/pictura-render/src/document_ops/mode/flat.rs:255-256`).

Duplicate/drag normalisation is deliberate: the transfer payload carries canonical coverage (after the `selection_from_target` change), and the destination save creates a Masked channel. The selection is correct; only the destination display mode differs. This is recorded as the ceiling in the parity gap.

### UI wiring

- Add one command id `panel.channels.channelOptions` to `crates/pictura-script/src/command_catalogue/const_names.rs` and a `CommandSpec` row with the same id to `crates/pictura-script/src/command_catalogue/entries.rs`; update `crates/pictura-script/tests/golden/const_names.tsv`, the `const_names` count test, and `crates/pictura-script/tests/golden/catalogue.tsv`, then regenerate `command_ids.h` with `scripts/gen-command-ids.py`. The registry is built from `command_catalogue()`, so `const_names` alone is not enough.
- `panels/panel_group_menu.cpp`: turn the disabled row into the `channelOptions` action mapped to the new command, including the `{ "channelOptions", PanelChannelsChannelOptions }` entry in the channels action map.
- `menu_panel_commands.cpp`: handler opens the dialog; the enabled provider is true only when the Channels panel exists, the current row is an alpha row, and the row is not a spot channel.
- The bridge gains `channels_selected_areas` (read), `channels_alpha_is_spot` (read), and `channels_set_sense` (write) in `channels.rs`/`channels/imp.rs`; the dialog initializes from the getter.
- `MenuRegistrar::Runtime` gains `openChannelOptionsDialog`; `frame.cpp` binds it to a new `PicturaMainWindow::openChannelOptionsDialog()` in `frame_selection_dialogs.cpp`.
- The dialog is built inline like `showQuickMaskOptions()` rather than in a new file: title `Channel Options`, `Color Indicates:` with two radios and stable object names, OK/Cancel. Accept calls the Rust bridge; on success the frame invalidates `kMaskOverlayKey`, refreshes the Channels panel, and re-syncs the alpha overlay.
- `ChannelsPanel` gains a `channelOptionsRequested()` signal, emitted by a new left-button `MouseButtonDblClick` branch in `eventFilter` when the row is an alpha row. `PanelHost::Hooks` gains a matching `channelOptionsRequested` field, connected in `panel_host_build.cpp` and bound in `frame.cpp`, mirroring `alphaVisibilityChanged`.
- Spot channels still appear as alpha rows. The command queries the codec's spot indices through the `channels_alpha_is_spot` bridge helper and disables the menu entry and double-click for a spot row. `channels_set_sense` checks eligibility before touching the plane, so a refused spot is never partially mutated.
- The alpha overlay cache key is the shown indices plus `canvas_revision`; `Edit::Meta` does not bump that revision. The accept path must clear `kMaskOverlayKey` (or include a channel/sense revision) before `refresh`, otherwise a shown channel keeps the old rubylith.

Alternatives considered: a new command id versus a panel-menu hook. The command id reuses the Duplicate Channel pattern, gives the enabled provider and tooltip for free, and keeps the transcribed menu and the live menu in sync; the hook would require bespoke enabling and dispatching.

### History

`Channel Options` is a metadata edit (`Edit::Meta`), one state, no compositing. Plane complementing allocates a full channel copy, which is the same cost as the existing channel save paths and acceptable for an explicit dialog action. `New Channel`, `Save Selection`, `Load Selection`, and `Duplicate Channel` keep their current labels and counts.

## Risks / Trade-offs

- [Kind-to-label mapping is inferred; the kind byte is observed in a Photoshop 24.7 file but its UI label is not] → Task 1.1 records the observed kind bytes in Photoshop CC/CS6 after the fixture-based implementation; the two constants are the only place to flip, and the manual check is the test-plan oracle. The automated scenarios are regression tests, not an oracle.
- [DisplayInfo alignment drifts as channels are added/deleted] → Keep add/delete/duplicate in the codec helpers, mirror the name handling, and test a three-channel delete and a Selected-add. Pad/append only with an explicit Masked entry.
- [A stale, shorter DisplayInfo list after adding a channel] → Missing entries default to Masked and are padded whenever a resource exists; later entries stay index-aligned because adds append and deletes remove at the index.
- [Stale entries survive a channel clear] → Clear DisplayInfo wherever `Document.channels` is cleared (Bitmap conversion) and test it.
- [PDF preserve-editing turns a missing entry into Selected] → Use an explicit Masked default in the PDF path and cover it with a preserve-editing test.
- [Spot entries overwritten] → `set_channel_selected_areas` refuses mode 2 and the UI excludes spot rows before mutating. Spot channels still appear as ordinary rows in this change; that is the existing #513 ceiling, not a regression.
- [Byte churn in existing PSDs] → No resource is created unless the user switches a channel to Selected Areas; Masked channels with no resource stay absent.
- [Duplicate/drag loses the source sense] → Deliberate scope cut; the selection is preserved and the ceiling is recorded in the parity register.

## Migration Plan

No data migration. Existing documents and files have no sense entry (or kind 1) and behave exactly as today. Rollback is a code revert: a file edited by this change may carry kind 0 or a new entry, which older builds read as ordinary coverage semantics, so the selection would invert on reopen. Record this caveat in the PR body (task 5.4) rather than adding a migration path.

## Open Questions

None. The mapping is verified by task 1.1 before code; it is a gate, not an open design question.
