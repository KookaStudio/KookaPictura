# Refactor — tool architecture and code splitting (round 2)

- **Status:** done. Tracks 0–3 landed; Track 4 is **superseded** by
  `docs/dev/refactor-view-seam.md` (typed view-seam adapters, issue #276 /
  #281). The single cxx-qt bridge (`cxxqt_object.rs`, 1200) still carries the
  surface until each migration unit deletes its share.
- **Type:** pure-mechanical splits first, then a behavior-preserving tool-handler
  redesign. No spec deltas, no new dependency, no algorithm change.
- **Contract:** every public symbol, signature, registration, self-test exit
  code, and on-disk format stays identical. `scripts/check-file-size.sh` is the
  guard, `scripts/guard.sh` and the §5 gate list are the proof. A commit that
  splits files carries no behavior fix, and a commit that changes behavior
  carries no split.
- **Companion:** `docs/dev/refactor-code-splitting.md` (round 1) took the tree
  from sixteen over-cap files to one allowlisted entry. This round keeps that
  ceiling while the tool catalogue grows.

## 1. Why

The 71-tool CS6 catalogue has about a dozen implemented tools; the rest land
over the coming milestones. Three structures will not absorb them under the
800-LOC preference / 1200-LOC hard cap (`AGENTS.md` rule 9):

| Structure | LOC | Why it grows per tool |
|---|---:|---|
| `crates/pictura-app/cpp/tools.cpp` | 1156 | holds the 71-row catalogue, the press/move/release/overlay switch, cursor policy, every per-tool parameter, and the transform session |
| `crates/pictura-app/src/cxxqt_object.rs` | 1200 | the single cxx-qt bridge; CXX-Qt requires every `#[qinvokable]` declaration here (226 today) |
| `crates/pictura-app/src/cxxqt_object/impl_transform.rs` | 1200 | move + free-transform body, 35 entry points |

The pressure is structural, not a too-small number. Raising the cap would let
the god-files grow and make the guard meaningless; splitting is unavoidable
because 58 tools at even ~150 LOC each need roughly eight new files anyway.

## 2. Goal

- `tools.cpp` under 800; no file over its cap except the allowlisted
  `selftest.cpp`.
- Adding a tool is one new handler file plus one registry entry plus its
  spec/tests — no edit to the dispatcher.
- The single cxx-qt bridge stays lean: new tools reuse existing invokables and
  only a genuinely new engine operation adds a declaration.

## 3. Target layout

```
crates/pictura-app/cpp/tools/
  tool_controller.{h,cpp}  facade: public API, signals, bindCanvas, event routing
  tool_context.h           shared services handed to handlers (view, canvas, settings)
  tool_handler.h           interface: press/move/release/key/cursor/overlay/activate/deactivate
  tool_registry.{h,cpp}    ToolId -> handler; no-op default for unimplemented tools
  tool_catalog.{h,cpp}     toolInfo and allToolIds over the
                           engine tool catalogue (#274), toolIdName,
                           toolCursorId, toolShortcutKeys,
                           toolGroupForKey, toolHintEntries, selectionModeString
  tool_settings.{h,cpp}    brush*, tolerance, marquee style/ratio/size, feather,
                           foreground/background, autoErase
  tool_policy.{h,cpp}      applyToolPolicy, refreshCursor, hoverCursorId, dragCursorId,
                           selectionModeForModifiers, topmostPixelLocked, applyBrushShortcut
  tool_dispatch.{h,cpp}    handlePressed/Moved/Released, updateDragOverlay,
                           warmMovePreview, commitCrop
  tool_transform.{h,cpp}   free-transform session and overlay
  handlers/
    tool_handler.h         the interface
    noop_tool.cpp          default for the unimplemented catalogue
    move_tool.cpp          move, content move, Alt clone
    marquee_tool.cpp       rectangular and elliptical marquee
    lasso_tool.cpp         lasso, polygonal, magnetic
    wand_tool.cpp          magic wand, quick selection
    brush_tool.cpp         brush, pencil, paint press, outline
    crop_tool.cpp          crop
    transform_tool.cpp     free-transform handler
    eyedropper_tool.cpp    eyedropper, color sampler
    hand_zoom_tool.cpp     hand, zoom
  tools_internal.h         translation-unit-shared helpers the siblings need
```

`tools.h` remains the public header. Existing `*ForTest` hooks stay on
`ToolController` and delegate, so the seven self-test suites and the options
bar do not churn.

## 4. Tracks

### Track 0 — guardrail and plan (done)

This brief, cross-referenced from `refactor-code-splitting.md`. No source
changes.

### Track 1 — mechanical splits (done)

`tools.cpp` was split along the existing seams with pure moves
(`df01173`, `d13e906`; 1156 → 764):

- `tools/tool_catalog.cpp` — the 71-row tool table (since #274 built from the
  engine catalogue), `toolInfo`, `allToolIds`, `toolImplemented`, `toolIdName`,
  `toolCursorId`,
  `toolShortcutKeys`, `toolGroupForKey`, `toolHintEntries`, `selectionModeString`.
- `tools/tool_transform.cpp` — `beginFreeTransform`, `commitFreeTransform`,
  `cancelFreeTransform`, `transformSessionActive`, `updateTransformOverlay`,
  `setTransformCursor`.
- `tools.h` re-includes the new headers so every existing include site and
  declaration is unchanged.

### Track 2 — handler architecture (done)

Landed (`1a57fef`, `c30e26e`, `fbfd513`): `tool_context.h`, `tool_handler.h`,
`tool_registry.{h,cpp}`, and handlers for every implemented tool — `Hand`,
`Zoom`, `Eyedropper`, `Brush`/`Pencil`, `MagicWand`, `QuickSelection`, `Move`,
`Crop`, `Marquee`, `EllipticalMarquee`, `Lasso`, and `PolygonalLasso` — plus a
shared `selection_geometry.{h,cpp}`. The legacy `switch (active_)` blocks are
deleted; `ToolController` dispatches to the registry and keeps only the
cross-cutting transform session, content-move routing, and cursor/hover policy.
`tools.cpp` is 417 LOC. Adding a tool is one new handler file plus one registry
entry.

Introduce `ToolContext`, `ToolHandler`, and a `ToolId -> handler` registry; the
default handler is a no-op, so unimplemented tools keep behaving as today.
`ToolController` becomes the facade that owns the context and forwards events.
Migrate one family per commit in this order: move, marquee, lasso, wand,
brush, crop, transform, eyedropper, hand/zoom. Each migration is
behavior-preserving; the existing self-test codes are the proof.

```cpp
// tools/tool_handler.h (sketch)
struct ToolContext {
    PictureView* view();          // active document bridge (may be null)
    ImageView* canvas();
    ToolSettings& settings();     // brush, tolerance, marquee, feather, colours
    void requestRepaint();
};

class ToolHandler {
public:
    virtual ~ToolHandler() = default;
    virtual void onPress(const QPointF& imagePos, int button,
                         Qt::KeyboardModifiers mods) {}
    virtual void onMove(const QPointF& imagePos,
                        Qt::KeyboardModifiers mods) {}
    virtual void onRelease(const QPointF& imagePos,
                           Qt::KeyboardModifiers mods) {}
    virtual bool onKey(int key, quint32 nativeScanCode,
                       Qt::KeyboardModifiers mods) { return false; }
    virtual QString cursorId(Qt::KeyboardModifiers mods) const { return {}; }
    virtual void onActivate() {}
    virtual void onDeactivate() {}
};
```

### Track 3 — bridge and test splits (done)

- `cxxqt_object/impl_transform.rs` (1200) → `impl_transform/{mod,geometry,session,dispatch}.rs`
  by pure move (`4c0b7fe`; the largest is 451). `cxxqt_object.rs` was not
  touched; `mod impl_transform;` still resolves.
- `codec/tests/oracle.rs` (1398) → a root plus `oracle/` submodules
  (`5340fe6`), and `render/src/tests/adjustment.rs` (1396) →
  `tests/adjustment/{mod,payloads,part_decode,part_composite}.rs`. The test set
  is unchanged (1261 Rust tests).
- `composite.rs` (1125) and `frame.cpp` (1068) still have headroom and are left
  for when they are next touched.

### Track 4 — bridge growth policy (superseded)

**Superseded by `docs/dev/refactor-view-seam.md` (#276 / #281).** The
single-bridge policy below is kept for history; the adapter namespaces under
`crates/pictura-app/cpp/api/` are now the growth path, and deleting superseded
invokables as each unit lands replaces the "decide in this order" escalation.

Keep the single `#[cxx_qt::bridge]`. New tool work must reuse existing
invokables; only a genuinely new engine operation adds a declaration. If
`cxxqt_object.rs` must grow past 1200, decide in this order:

1. a second `#[cxx_qt::bridge]` QObject (`ToolBridge`) for tool-specific
   invokables;
2. a small generic `dispatch(verb, args)` invokable for tool actions;
3. a bounded cap bump recorded in `scripts/file-size-allowlist.txt` as a
   ceiling that must shrink.

## 5. Per-step gate

Run after every commit, from the repo root:

```bash
cmake --build build
./build/pictura --headless --self-test
./build/pictura --headless --self-test crates/pictura-codec/tests/fixtures/two_layers.psd
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo test --workspace --doc
openspec validate --all --strict
bash scripts/check-file-size.sh
bash scripts/guard.sh
```

A step is done when the build succeeds, both self-tests exit 0 with unchanged
codes, the Rust suite and doctests pass, the specs validate, and the size guard
passes.

## 6. Deferred

- Behavior gaps recorded during the UI rounds (for example the zero-motion
  Alt-click clone) land after Track 2, as separate commits, never inside a
  split.
- Removing the now-unused pieces of the old single-switch dispatch, once every
  implemented tool has a handler.
- Replacing the `*ForTest` hooks with handler-level test seams, if that ever
  becomes worthwhile; not required for the split.
