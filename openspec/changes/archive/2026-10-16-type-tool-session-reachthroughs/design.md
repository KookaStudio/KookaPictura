# Design: type-tool-session-reachthroughs

## Context

See `proposal.md` - Why. Current state that shapes the approach:

- `ToolHandler` (`tool_handler.h`) is a single 42-virtual base. Its concrete
  subclasses (`CropToolHandler`, `TypeToolHandler`,
  `PerspectiveCropToolHandler`, ...) are anonymous-namespace classes defined
  file-locally inside their `tool_*.cpp` translation units
  (`tool_crop.cpp:66`, `tool_type.cpp:155`, `tool_perspectivecrop.cpp:35`); they
  are never visible to callers.
- `ToolRegistry` (`tool_registry.h:24`) is a homogeneous
  `std::array<std::unique_ptr<ToolHandler>, 71>` keyed by `ToolId`. Handlers are
  constructed once in the `ToolController` constructor (`tools.cpp:192-256`) and
  owned by the registry for the controller's whole lifetime; `setActiveTool`
  only calls `onDeactivate`/`onActivate`, it never destroys or replaces a
  handler. A resolved handler/port pointer therefore cannot dangle on a tool
  switch today.
- The predecessor change `tool-options-handler-seam` (#357) already resolves a
  typed per-family surface with `ToolController::optionsFor<T>(ToolId)`
  (`tools.h:195`) → `dynamic_cast<T*>` over `handler->optionGroups()`, where the
  cast target is a concrete type declared in the shared `tool_options.h` and the
  handler registers its groups at runtime (`addOptions`, `tool_handler.h:162`).
  There is no registration-invariant test.
- The crop commit/cancel path is active-aware: `ToolController::commitCrop`
  (`tools.cpp:901`) and `cancelCrop` (`tools.cpp:403`) both compute `active_ ==
  ToolId::PerspectiveCrop ? ToolId::PerspectiveCrop : ToolId::Crop`, so a Crop
  page (staged box survives a switch) and a Perspective page share them.
- The frame's key routing is part of the messy surface: `frame.cpp:1073` hardcodes
  `Enter -> {PolygonalLasso, MagneticLasso, Pen}`, `frame.cpp:1082` calls the Esc
  hook for any active tool and gates fall-through on its `bool`, and
  `frame_menus_edit.cpp:329` routes Delete through the remove-point hook.

## Goals / Non-Goals

**Goals:**
- Shrink `ToolHandler` to a stated generic surface: `onPress`/`onMove`/
  `onModifiers`/`onRelease`, `onActivate`/`onDeactivate`/`onDocumentRefreshed`/
  `onOptionsChanged`, `nudge`, `pointerCursor`/`hoverCursor`/`pointerCursorKind`/
  `cursorVariant`, `dragMods`/`marqueeMirror` (test hooks), `toolUndo`/`toolRedo`/
  `canToolUndo`/`canToolRedo`, `clearAnnotations`, and the renamed generic
  gesture hooks `onFinishPath`/`onEscape`/`onDeletePoint`/`gestureInProgress`.
- Give the frame, options bars, and panels typed access to crop-commit, crop-box,
  perspective, and type sessions without the `tools.cpp` trampolines.
- Stop the "lasso family" misclassification.
- Preserve behavior exactly; the Qt suites and self-tests are the oracle.

**Non-Goals:**
- Moving session *state* out of handlers into a controller-owned store. That
  makes "staged crop survives a tool switch" structural rather than remembered,
  but it is a larger lifetime/signature refactor; the port seam is the enabling
  step, and the store is a separate follow-up.
- Free Transform (`tool_transform.cpp`) — not a `ToolHandler`.
- Any new dependency, and any change to `openspec/specs/` (pure refactor).

## Decisions

### D1. Four narrow ports, splitting shared commit from Crop-only box

`tool_sessions.h` declares four pure-virtual ports:

| Port | Methods | Implemented by |
|---|---|---|
| `CropCommitSession` | `commitCrop` | Crop, Perspective Crop |
| `CropBoxSession` | `cropActive`, `resizeCrop`, `cropWidth`, `cropHeight`, `resetCropBox`, `cropCenter`, `setStraightenMode`, `hasPendingCrop`, `pendingCropRect` | Crop only |
| `PerspectiveCropSession` | `setPerspectiveOutput`, `perspectiveQuadComplete`, `setPerspectiveShowGrid` | Perspective Crop only |
| `TextSession` | `commitText`, `cancelText`, `textActive`, `insertText` | Type |

- *Why split `CropCommitSession` from `CropBoxSession`:*
  `PerspectiveCropToolHandler` implements `commitCrop`
  (`tool_perspectivecrop.cpp:153`) but none of the Crop box methods. Folding both
  into one pure-virtual `CropSession` would force Perspective Crop to stub nine
  methods it does not have — contradicting "implement only the ports you
  support."
- *Why ports over exposing concrete handler classes:* handler internals stay
  private, a handler can implement several unrelated ports cleanly, and the shape
  mirrors #357. Exposing concrete classes leaks file-local implementation
  surfaces and still needs a shared base for the two crop tools.
- Port methods take primitives/quanta only, so the header stays a declaration
  sheet. If it accretes helpers, split per family then, not preemptively.

### D2. Resolve ports with `dynamic_cast`, and state the RTTI requirement

`ToolController` gets `cropCommitSession()`, `cropBoxSession()`,
`perspectiveCropSession()`, `textSession()`, each returning `Port*` via
`dynamic_cast<Port*>(handlerFor(id))` and `nullptr` when the handler does not
implement the port.

- *Why over a declared-id table:* the port is the single source of truth; a
  parallel id table duplicates the inheritance graph and drifts. RTTI cost is nil
  at click frequency.
- *Differences from `optionsFor<T>` (do not overclaim it as the same idiom):*
  that casts a `ToolOptions*` to a concrete type in a shared header and reads a
  runtime registration list. These ports cross-cast a `ToolHandler*` to a second
  base of an anonymous-namespace class defined in another `.cpp`. It works under
  the current build (no `-fno-rtti`, no `-fvisibility=hidden` in any
  `CMakeLists.txt` or `.cargo/config.toml`), and the design assumes RTTI stays
  enabled across the app target; the build step verifies it.
- *Over `std::variant`/reflection:* those add type-index churn or Qt reflection
  on non-QObject handlers for a plugin/hot-reload need that does not exist.

### D3. Resolver targets: one active-aware, the rest fixed

- `cropCommitSession()` is **active-aware** and is the single place the ToolId
  choice lives: it resolves `active_ == ToolId::PerspectiveCrop ?
  ToolId::PerspectiveCrop : ToolId::Crop`, exactly preserving `tools.cpp:906`
  (Crop's staged box survives a switch; a Perspective quad lives only while
  active). Both the Crop and the Perspective Apply buttons and the frame Enter
  branch call it.
- `cropBoxSession()` and `perspectiveCropSession()` resolve a **fixed** `ToolId`.
- `textSession()` resolves the **active** tool (the edit lives only while the
  type tool is active).
- Resolvers are cheap; callers may hold the result for the length of one
  command. Because handlers outlive tool switches (Context), holding one across
  an event boundary is safe today, but the contract is still "resolve, use,
  discard" so a future registry rebuild does not silently break it.

### D4. Reclassify the generic gesture quartet in place

Rename on `ToolHandler`, keeping the flat base and the `bool`
consumed/passthrough contract: `commitPolygonLasso` → `onFinishPath`,
`cancelPolygonLasso` → `onEscape`, `removeLassoPoint` → `onDeletePoint`,
`lassoInProgress` → `gestureInProgress`. Rename the controller forwarders
(`commitPolygonLasso`/`cancelPolygonLasso`/`removeLassoPoint`, `tools.h:354-357`)
to `handleFinishPath`/`handleEscape`/`handleDeletePoint`.

- *Why not a subscriber registry:* the ~7 Esc implementers are
  compile-time-linked and already reachable through the vtable; a registry
  duplicates virtual dispatch and adds ordering/ownership rules for zero new
  capability.
- `cancelCrop()` (`tools.cpp:403`) is deleted: it is only the Esc hook under a
  crop-specific name. The options-bar Cancel buttons retarget to `handleEscape`.
- The rename is only honest if `frame.cpp` is cleaned too: replace the Enter tool
  list with a dispatch to the active tool's `onFinishPath`, and keep the Esc and
  Delete fall-through on the returned bool.

### D5. Verify port resolution with a Qt Test, not a fake invariant

Add a Qt Test that resolves each port for an owning `ToolId` and asserts non-null,
and asserts null for a tool that does not implement it (e.g. a paint tool for
`cropBoxSession()`). This fails into visibility if a port implementation is
dropped. *Not* a declared-id startup assertion — that needs the very table D2
rejects, and a self-test `ST_` check would break the shrink-only self-test budget
(`scripts/selftest-budget.txt`). The GUI Qt Test is the growth path.

## Risks / Trade-offs

- [`CropCommitSession` and `CropBoxSession` are easy to confuse] →
  `commitCrop` lives on exactly one port; every other crop-box method on the
  other; the test in D5 pins which `ToolId` resolves which port.
- [`dynamic_cast` requires RTTI; a future build flag could disable it] → Build
  step verifies the resolvers compile and the D5 test passes; note the RTTI
  assumption in `tool_sessions.h`.
- [`frame.cpp` is 1198/1200 lines] → the Enter/Esc routing edit replaces a
  condition with a call and must stay net-neutral or negative; verify
  `scripts/check-file-size.sh` in the same task.
- [Renaming silently breaks the `bool` consumed/passthrough contract] → Read
  every override's return before the rename; the frame routing tests and
  self-tests are the oracle.
- [New header not compiled] → `CMakeLists.txt` lists headers explicitly
  (`:421-425`); add `tool_sessions.h`.
- [Dangling a resolved pointer] → Safe today because handlers outlive switches;
  kept safe by the resolve-and-discard contract if the registry ever rebuilds.

## Migration Plan

Incremental, family by family; each step compiles and its suites pass.

1. `tool_sessions.h` with `CropCommitSession` and `CropBoxSession`; `CropToolHandler`
   inherits both; add `cropCommitSession()`/`cropBoxSession()`; retarget the
   controller's internal crop callbacks (`tools.cpp:148-162` for
   `cropCenter`/`setStraightenMode`) and the inline `tools.h:298-307`
   `hasPendingCrop`/`pendingCropRect`; retarget `options_bar_crop.cpp`;
   run `tst_crop_*` + `selftest_crop_tool`.
2. `PerspectiveCropSession`; `PerspectiveCropToolHandler` inherits it and
   `CropCommitSession`; retarget the perspective reads in `options_bar_crop.cpp`;
   run `tst_crop_preview`.
3. `TextSession`; retarget `options_bar_type.cpp`, `panels/glyphs_panel.cpp`, and
   the text branch of `frame_menus_edit.cpp`; run `tst_type_*`.
4. Delete the crop/perspective/type virtuals from `tool_handler.h` and their
   `tools.{h,cpp}` trampolines/aliases (`cancelCrop`, `resetCrop`); keep the
   generic `toolUndo` family and `clearAnnotations`; add the D5 resolution test.
5. Rename the gesture quartet across `tool_handler.h`, the controller
   (`tools.{h,cpp}`), the handler overrides (`tool_pen.cpp`, `tool_selection.cpp`,
   `tool_magneticlasso.cpp`, `tool_slice.cpp`, `tool_rotate_view.cpp`,
   `tool_path_selection.cpp`, `tool_crop.cpp`, `tool_perspectivecrop.cpp`) and the
   self-tests; clean `frame.cpp`/`frame_menus_edit.cpp` routing; run
   `selftest_magnetic_lasso` and `selftest_tools_selection`.
6. Full verification: `ctest -R '^tst_'`, both app self-test invocations,
   `scripts/check-file-size.sh`, `scripts/check-selftest-budget.sh`,
   `scripts/guard.sh`, `openspec validate --all --strict`.

Rollback is a plain revert: no file format, persisted state, or public contract
changes.

## Open Questions

- Whether to later move crop/text session state into a controller-owned store to
  make the tool-switch survival structural. Deferred by design (Non-Goals); it
  does not change this change's approach or task breakdown.
