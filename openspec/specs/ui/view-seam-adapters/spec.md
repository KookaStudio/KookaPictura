# view-seam-adapters Specification

## Purpose

Defines how the C++ shell consumes the cxx-qt `PictureView` seam: an exclusive typed
adapter surface for migrated units, batched and typed data contracts where raw
crossings today re-parse strings or fan out per row, and paint-stroke ordering
enforced by the adapter rather than by convention.

## Requirements

### Requirement: Adapters are the only C++ surface over covered bridge operations

Production C++ outside `crates/pictura-app/cpp/api/` SHALL reach the Rust bridge only
through adapter namespaces (`document_api`, `layers_api`, `selection_api`, `paint_api`,
`transform_api`, `history_api`, `type_api`) for bridge operations covered by a landed
migration unit. Generated cxx-qt headers SHALL be included only by adapter translation
units, except a named temporary deferral (recorded in this change's tasks) for lines
in `frame_includes.h` whose owning unit has not yet landed. Bridge operations not yet
covered by a migration unit SHALL be listed as named deferrals in this change's tasks
until a later unit claims them. Adapters SHALL exist only where they absorb real
complexity (string protocols, call ordering, batching, or a unit façade that
centralizes multi-call sequences); a 1:1 forwarder of a single invokable SHALL NOT be
introduced, and no document algorithm SHALL live in an adapter.

#### Scenario: A panel includes only adapter headers

- **WHEN** a production panel, dialog, command, tool, or control-server TU needs a
  bridge operation covered by a migration unit that has landed
- **THEN** it includes the adapter header for that unit and does not include a
  generated `*.cxxqt.h` header or hold raw `PictureView*` calls outside the adapter

#### Scenario: Adapter registration is explicit

- **WHEN** a new adapter `.cpp`/`.h` pair is added under `cpp/api/`
- **THEN** it is listed explicitly in `CMakeLists.txt` and builds into `pictura_shell`

#### Scenario: Pass-through adapters are rejected at review

- **WHEN** a proposed adapter is a 1:1 forwarder of a single invokable with no
  batching, no typed protocol, no ordering guarantee, and no unit-level call-sequence
  centralization
- **THEN** the adapter is not added; callers keep the existing surface until a unit
  that absorbs complexity lands

#### Scenario: Uncovered operations are named deferrals

- **WHEN** a bridge operation has no landed migration unit
- **THEN** it is listed as a named deferral in this change's tasks rather than
  claimed as exclusive-surface complete

### Requirement: String geometry crosses as typed values

For each migrated unit, geometry and bounds that today cross as a space-separated
`QString` SHALL cross the seam as a typed value (`LayerRect`, `Quad`, `CanvasSize`,
`SelectionContour`, `SelectionBounds`, or an equivalent fixed-width matrix) that the
adapter converts once. C++ consumers of a migrated unit SHALL NOT re-parse bridge
strings for that geometry. Once no production or test caller remains, the superseded
string invokable SHALL be removed from the bridge. Unit ownership: document owns
`display_level_size`/`CanvasSize`; selection owns `selection_contour` and
`selection_bounds`; transform owns `layer_rect`, `quad`, and the preview matrix;
crop/composite bounds (`composite_bounds`) are claimed with transform (tool_crop /
tool_perspectivecrop) or a named follow-up if deferred.

#### Scenario: Layer rect is typed at the adapter

- **WHEN** C++ needs a layer's document-space rectangle after the transform unit lands
  (including `properties_panel.cpp` and `tool_transform.cpp` / `image_view.cpp`
  parsers)
- **THEN** it receives a typed rect from `transform_api` with no `split`/`toInt` parse
  of a bridge string

#### Scenario: Display level size is typed at the adapter

- **WHEN** C++ needs a pyramid level's size after the document unit lands
  (including `frame_canvas.cpp`, `dialogs.cpp`, navigator/channels/histogram panels)
- **THEN** it receives a typed `CanvasSize` (or equivalent) with no string split

#### Scenario: String invokable deleted after last caller

- **WHEN** every production and test caller of `layer_rect` (or another superseded
  string geometry invokable) has migrated to the typed API
- **THEN** the invokable is removed from `cxxqt_object` and the build succeeds with
  both self-test runs exiting with unchanged codes

#### Scenario: Equivalence during migration

- **WHEN** a typed geometry API lands alongside the string invokable
- **THEN** for any fixture state, the typed value equals the parsed string invokable
  (asserted by the unit's Qt Test or migrated self-test)

### Requirement: Layers panel rows fetch in one snapshot

The Layers panel SHALL obtain its per-row fields for a refresh in a single
`layers_snapshot(thumb_size, entire_doc)` call returning one typed row record per
layer, rather than ~20 per-field invokables per row plus a per-effect
`layer_style_value` probe. While the pre-migration per-getter surface exists, the
snapshot MUST be field-for-field equivalent to it on a fixture document (asserted by
`tst_layers_snapshot`); once that surface is removed, the fixture's expected typed
values are the regression check.

#### Scenario: Snapshot equals per-getter rows during migration

- **WHEN** `tst_layers_snapshot` builds a fixture document while the legacy per-getter
  sequence still exists and compares `layers_snapshot` output to it for every row
- **THEN** every field (path, depth, name, kind, visibility, blend, opacity, fill,
  lock bits, colour, clipping, mask/style/adjustment/vector-mask flags, child count,
  linked, placed, shape, smart-object, style effect names, row and mask thumbnails at
  the requested size) matches; document width/height and thumbnail size parameters
  are compared once per snapshot, not per row

#### Scenario: Panel refresh uses one crossing batch

- **WHEN** the Layers panel rebuilds its rows after the layers unit lands
- **THEN** row data comes from one snapshot call per rebuild (plus document size
  already on the snapshot), not one crossing per field per row

#### Scenario: Fixture covers styled and nested rows

- **WHEN** the snapshot fixture is built for equivalence
- **THEN** it includes styled, vector-masked, smart-object, clipped, shape, nested,
  and adjustment rows so every field is exercised at least once

### Requirement: Paint stroke ordering is enforced by paint_api

`paint_api` SHALL expose stroke begin / present / end so that a stroke cannot be left
open on the normal exit path: a tool-lifetime stroke object (constructed on press,
finished or cancelled on release/Esc, destructor still closes exactly once) covers
multi-event tools; mid-stroke reads of the full composite SHALL NOT be issued through
the stroke API. The mid-stroke present path SHALL remain "queue `flush_present`, do
not rebuild via `image()`". Deferred presents (including `frame.cpp`'s next-turn
timer) SHALL route through the adapter once unit C lands.

#### Scenario: Stroke closes on scope exit or tool release

- **WHEN** a tool begins a stroke through `paint_api` and the press handler or tool
  object goes out of scope without an explicit end (normal return or early return)
- **THEN** the stroke is finished or cancelled exactly once and the history state
  matches an explicit begin/end pair

#### Scenario: Mid-stroke present does not rebuild the frame

- **WHEN** a dab is presented mid-stroke through `paint_api`
- **THEN** the call queues `flush_present` and does not call `image()` or otherwise
  force a full composite rebuild for that present

### Requirement: Migration units land independently

Adapter work SHALL land in migration units that each leave the build green:
document+history, then selection, then paint+transform, then layers+snapshot, then
type. A unit is done when its consumers use the adapter, its superseded raw surface
is deleted or explicitly deferred with a named last caller, and the full verification
suite for the change still passes (including both headless self-test runs, workspace
tests, `ctest -R '^tst_'`, `openspec validate --all --strict`, file-size, guard, and
selftest budget).

#### Scenario: Unit completes with deletions

- **WHEN** a migration unit finishes
- **THEN** its consumers no longer call the superseded raw entry points for that
  unit, `scripts/check-file-size.sh` and `scripts/guard.sh` and
  `scripts/check-selftest-budget.sh` pass, and if `cxxqt_object.rs` dropped under the
  cap the allowlist entry is removed or lowered

#### Scenario: Order is preserved within the PR

- **WHEN** commits on the change branch are reviewed
- **THEN** units appear in the order document+history → selection →
  paint+transform → layers+snapshot → type, each independently buildable

#### Scenario: Self-test codes and budget unchanged

- **WHEN** any adapter unit lands
- **THEN** `./build/pictura --headless --self-test` and the same run with
  `crates/pictura-codec/tests/fixtures/two_layers.psd` exit with unchanged status
  codes, and no new `ST_BEGIN` site is added (selftest budget does not rise)
