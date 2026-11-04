# view-seam-adapters Specification

## Purpose

Defines how the C++ shell consumes the cxx-qt `PictureView` seam: an exclusive typed
adapter surface for migrated units, batched and typed data contracts where raw
crossings today re-parse strings or fan out per row, and paint-stroke ordering
enforced by the adapter rather than by convention.

## Requirements

### Requirement: Adapters are the only C++ surface over covered bridge operations

Production C++ outside `crates/pictura-app/cpp/api/` SHALL reach the Rust bridge only
through (a) adapter namespaces (`document_api`, `layers_api`, `selection_api`,
`paint_api`, `transform_api`, `history_api`, `type_api`) for bridge operations covered
by a landed migration unit, or (b) a domain membrane header under `cpp/api/` for every
other bridge operation. Generated cxx-qt headers SHALL be included only by translation
units and headers under `cpp/api/`; no deferral is permitted outside that directory.
The type adapter's own header (`cpp/api/type_api.h`) is the named exception that owns
the generated `type_tools*.cxxqt.h` headers, as the existing type-adapter requirement
requires. Adapters SHALL exist only where they absorb real complexity (string
protocols, call ordering, batching, or a unit façade that centralizes multi-call
sequences); a 1:1 forwarder of a single invokable SHALL NOT be introduced, and no
document algorithm SHALL live in an adapter.

#### Scenario: A panel includes only adapter headers

- **WHEN** a production panel, dialog, command, tool, or control-server TU needs a
  bridge operation
- **THEN** it does not include a generated `*.cxxqt.h` header

#### Scenario: Adapter registration is explicit

- **WHEN** a new adapter `.cpp`/`.h` pair or a new membrane `.h` is added under `cpp/api/`
- **THEN** it is listed explicitly in `CMakeLists.txt` and builds into `pictura_shell`

#### Scenario: Pass-through adapters are rejected at review

- **WHEN** a proposed adapter is a 1:1 forwarder of a single invokable with no
  batching, no typed protocol, no ordering guarantee, and no unit-level call-sequence
  centralization
- **THEN** the adapter is not added; callers use the domain membrane instead

#### Scenario: Uncovered operations are named deferrals

- **WHEN** a bridge operation has no landed migration unit
- **THEN** it is reached through a domain membrane header and no generated-header
  deferral is recorded outside `cpp/api/`

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

### Requirement: The type adapter is a session façade

The type adapter SHALL stay the only C++ surface over the type bridge, and its
interface SHALL NOT mirror the bridge one to one. Specifically:

- A type layer's text, font, setting, character and paragraph state SHALL cross
  the adapter as one value, both when read and when passed to preview, commit
  and update.
- Re-editing an existing type layer SHALL be one edit session that begins on the
  hit and ends in exactly one commit or cancel. A session dropped unfinished
  SHALL cancel.
- Warp Text preview SHALL be one warp session that ends in exactly one commit or
  cancel. A session dropped unfinished SHALL cancel.
- The character and paragraph style libraries SHALL be read as one list per
  refresh, not by index loops over count, name and setting getters.

Sessions SHALL hold a guarded reference to the document view. A session whose
view was destroyed SHALL end without touching it. A failed commit SHALL also end
the session.

The generated type bridge headers (`type_tools.cxxqt.h` and those under
`type_tools/`) SHALL be included only by the type adapter's own header and
translation unit. No other production header or source, including other
adapters, SHALL include them.

#### Scenario: Re-edit commits once [type_session_commit]

- **WHEN** a type layer is re-edited through an edit session and committed with
  new text
- **THEN** the layer shows the new text, one history state is recorded, and the
  layer is visible again

#### Scenario: Dropped session cancels [type_session_cancel]

- **WHEN** an edit session begins on a type layer and is destroyed without a commit
- **THEN** the layer is restored unchanged and visible, and no history state is
  recorded

#### Scenario: Warp session previews then cancels [type_warp_session]

- **WHEN** a warp session previews a warp on a type layer and is then cancelled or
  dropped
- **THEN** the layer's warp is restored and no history state is recorded. Committing
  instead records one "Warp Text" state.

#### Scenario: Style library reads in one call [type_style_library]

- **WHEN** the Character Styles or Paragraph Styles panel refreshes
- **THEN** it receives every style's name, font and settings from one adapter
  read in library order

#### Scenario: Generated type header stays inside the adapter [type_header_confined]

- **WHEN** the production C++ tree, other than `cpp/api/type_api.{h,cpp}` and
  `cpp/tests/`, is searched for an include of a generated type bridge header
- **THEN** there is none

### Requirement: Generated headers are confined to cpp/api

Generated cxx-qt headers SHALL be included only by headers and translation units under
`crates/pictura-app/cpp/api/`. Every C++ file outside that directory, including
`cpp/tests/` and `frame_includes.h`, SHALL include a domain membrane or an adapter
header instead, except for the generated type bridge headers governed by the existing
type-adapter requirement. The type bridge headers remain owned by
`cpp/api/type_api.{h,cpp}`, and the existing `cpp/tests/` allowance for them is
unchanged. A generated header with no C++ consumer is exempt from the membrane mapping.
A membrane SHALL contain an include guard or `#pragma once`, includes, and comments
only: no function bodies, no state, and no document algorithm.

#### Scenario: Generated headers are confined to cpp/api

- **WHEN** the C++ tree outside `crates/pictura-app/cpp/api/` is searched for an
  include of a generated `*.cxxqt.h` header
- **THEN** there is none, except the type bridge headers governed by the existing
  type-adapter requirement

#### Scenario: A membrane carries no logic

- **WHEN** a membrane header is reviewed or parsed
- **THEN** it contains `#pragma once` (or an include guard), includes, and comments
  only, with no function bodies or state

### Requirement: Domain membranes bound the per-translation-unit include surface

Generated headers SHALL be grouped into domain membranes by consumer co-inclusion, so
that a translation unit includes the fewest membranes that cover the verbs it uses. No
membrane SHALL include every generated header, and each membrane's generated includes
SHALL be a proper subset of the set of generated headers. The global bridge stamp still
rebuilds every shell and test translation unit on any bridge edit; the change keeps
that fan-out global.

#### Scenario: No membrane is a universal header

- **WHEN** the membranes under `cpp/api/` are inspected
- **THEN** no single membrane includes all generated headers, and each membrane's
  generated includes are a proper subset of the set of generated headers

### Requirement: Bridges with two or fewer functions are folded

A standalone `#[cxx_qt::bridge]` file that exposes at least one `extern "Rust"` or
`extern "RustQt"` function and two or fewer SHALL NOT exist. Its declarations SHALL
live in a domain-neighbour bridge file, its `imp` module SHALL be re-homed (or the file
SHALL remain as a plain, non-bridge module), and the build script's bridge-file list
SHALL reflect the folded set. A bridge module that only imports `extern "C++"`
functions (no `extern "Rust"`/`extern "RustQt"` function) is exempt.

#### Scenario: The small bridges are gone

- **WHEN** the bridge files are counted and each is inspected for exposed functions
- **THEN** every bridge that exposes at least one Rust/RustQt function declares more
  than two, import-only bridges are exempt, and the bridge count is `<= 69`

#### Scenario: A folded verb keeps its call sites working

- **WHEN** a function moves from a folded bridge into its domain neighbour
- **THEN** its generated C++ symbol and signature are unchanged (or
  `decode_bridge::copy_bytes` stays unnamespaced), and the build and Qt suites pass
  without call-site edits

### Requirement: The bridge seam ratchet only shrinks

A repository check SHALL fail when a C++ file outside `cpp/api/` includes a generated
`*.cxxqt.h` (except the type bridge headers governed by the existing type-adapter
requirement), when the bridge-file count exceeds 69, or when
`crates/pictura-app/src/cxxqt_object.rs` exceeds 1,204 lines. The check SHALL run from
`scripts/verify-fast.sh`.

#### Scenario: A new direct generated include fails the gate

- **WHEN** a C++ file outside `cpp/api/` gains an include of a generated `*.cxxqt.h`
  other than the type bridge headers
- **THEN** `scripts/verify-fast.sh` fails and names the file

#### Scenario: Bridge growth fails the gate

- **WHEN** a change adds a bridge file or grows `cxxqt_object.rs` past 1,204 lines
- **THEN** `scripts/verify-fast.sh` fails
