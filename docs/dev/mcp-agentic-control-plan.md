# Agentic MCP Control — implementation plan

- **Status:** P0–P3 shipped — the app-side control server is archived as
  `2026-09-22-app-control-server` (`--control`, the JSON socket protocol, and the
  `status`/`get_pixel`/`list_layers`/`list_commands`/`dispatch_command`/
  `document`/`edit`/`set_unsaved_policy` methods), its vision half as
  `2026-09-23-agentic-control-vision` (`screenshot`, `ui_tree`,
  `layer_thumbnail`), its action half as `2026-09-23-agentic-control-actions`
  (`set_tool`, `selection`, `filter`, `adjustment`, `layer_op`,
  `set_gpu_compute`), and its input half as `2026-09-23-agentic-control-input`
  (`pointer`, `key`). Its live end-to-end script is archived as
  `2026-09-23-agentic-control-e2e`. Remaining: P4 `pictura-mcp`, and the P5
  client registration plus long-form docs. Authored before coding so the work can
  resume cold.
- **Track:** infrastructure/automation, **not** the M44 (layer filtering/search)
  milestone program.
- **Proposed OpenSpec change:** `mcp-agentic-control`
- **Proposed capability:** `agentic-control`
- **Proposed long-form spec:** `docs/11-cross-cutting/agentic-testing.md` (XC id TBD)
- **Docs guard:** files under `docs/` require a `TASK-ALLOWS-DOCS` marker in the
  landing commit (`scripts/guard.sh`).

This document is the exhaustive design + task list. It is intentionally more
detailed than the eventual OpenSpec proposal; the proposal should cite it.

---

## 1. Goal

Let any MCP-capable agent (opencode, Claude Code, Claude Desktop, Cursor, …)
drive a running Kooka Pictura instance for **manual, explorable testing**:

- see the UI and the document (`screenshot`, `get_pixel`);
- inspect application, document, layer, selection and command state;
- invoke real commands, tools, filters and layer operations;
- verify results by reading pixels back.

The loop must run through the **same non-interactive code paths `--self-test`
already uses**, so an agent reaches exactly the behavior the test suite locks
down — not a parallel reimplementation.

### Non-goals (v1)

- No ExtendScript/JS/Rust scripting engine (that is `AUTO-010`, a separate track).
- No arbitrary code evaluation tool (avoids RCE-by-design).
- No remote/network transport by default (stdio + local socket only).
- No pixel-perfect computer-use/vision-only automation: we expose a typed,
  semantic control surface **and** screenshots.
- Not a replacement for `--self-test` or the Rust test suite; it is the
  human/agent-driven complement.

---

## 2. What already exists (leverage — do not rebuild)

| Surface | Location | Notes |
|---|---|---|
| App frame | `crates/pictura-app/cpp/frame.{h,cpp}` — `PicturaMainWindow` | documents, tabs, tools, panels, screen modes, status readouts, `refresh()`, `grab()` (inherited `QWidget`) |
| Command registry | `crates/pictura-app/cpp/commands.{h,cpp}` — `CommandRegistry` | `dispatch(id)`, `action(id)`, `refresh()`, `topLevelTitles()`; `entries_` is **private** and needs an enumeration accessor |
| Full command tree | `crates/pictura-app/cpp/command_tree.cpp` | 523 documented leaves + implemented ids; stable id constants in `commands.h::command_ids` |
| Document canvas | `crates/pictura-app/cpp/image_view.{h,cpp}` — `ImageView` | `image()`, `grab()`, `widgetToImage()`, `zoom`, `setOverlayPolygon`, pointer signals |
| Engine bridge | `crates/pictura-app/src/cxxqt_object.rs` — `PictureView` (cxx-qt) | documents, layers, selection, filters, adjustments, history, paint, GPU backend; all callable from C++ via `pictura_app/src/cxxqt_object.cxxqt.h` |
| In-process automation | `crates/pictura-app/cpp/main.cpp` `--self-test` | 5800+ lines of the exact non-interactive call sequences we want to expose; exit codes 0–166 (**128, 129 free**) |
| Headless mode | `main.cpp` `--headless` | selects `offscreen` before `QApplication`; **currently implies `--self-test` when no doc** — control mode must suppress that |
| Session isolation | `main.cpp` `--self-test` branch | `QTemporaryDir` + `XDG_STATE_HOME`; reuse for reproducible agent sessions |
| Screenshots | `QWidget::grab()` | works under offscreen **and** a real platform; no scrot/grim/portal needed |
| Test fixture | `crates/pictura-codec/tests/fixtures/two_layers.psd` | 8×8 red TL quadrant + blue BR quadrant; used by the self-test |

Key framing: the control server is a **runtime, external `--self-test`** —
same method calls, driven by an agent instead of a fixed script.

---

## 3. Architecture

### 3.1 Chosen: two components

```
MCP client (opencode / Claude Code / Claude Desktop)
        │  MCP over stdio (JSON-RPC)
        ▼
pictura-mcp        new Rust binary crate: rmcp + tokio
        │  newline-delimited JSON over QLocalServer (0600, $XDG_RUNTIME_DIR)
        ▼
pictura --control  existing app; ControlServer runs on the Qt GUI thread
        ├── PicturaMainWindow   (documents, tabs, tools, panels, screen modes)
        ├── CommandRegistry     (dispatch by stable id)
        ├── ImageView           (grab, widgetToImage, event synthesis)
        └── active PictureView  (filters, selection, layers, history, GPU)
```

Why this shape:

1. **GUI stays single-threaded.** The socket's `readyRead`/`newConnection`
   slots run on the Qt event loop, so dispatch is already on the GUI thread.
2. **Async/MCP dependencies stay out of the app.** `tokio`, `rmcp`, `schemars`
   live only in `pictura-mcp`; `pictura_app` gains no Rust deps.
3. **Proven pattern.** Blender/Godot/Unity MCP all split an in-app control
   endpoint from an out-of-process MCP frontend.
4. **Attach or launch.** The MCP binary can connect to a user-launched GUI or
   spawn its own headless instance.

### 3.2 Rejected alternatives

| Option | Why not |
|---|---|
| **B — app hosts `rmcp` itself** (Streamable HTTP on localhost) | Single process, but drags `tokio`+`rmcp` into the app staticlib and requires marshalling tool calls from an async runtime thread into the Qt event loop. More moving parts in the risky place, and Qt objects are not thread-safe. |
| **C — stateless CLI wrapper** (`pictura --exec '<json>'`) | Simplest, but relaunches Qt per call, loses live session/state, and cannot attach to a running GUI the user is watching. |

### 3.3 Process lifecycle

- **Control mode:** `pictura --control [--control-socket PATH] [--state-home DIR] [<doc>]`.
  - Default socket: `$XDG_RUNTIME_DIR/pictura-control.sock`.
  - No `--headless` → normal visible window (manual, human-observable testing).
  - With `--headless` → offscreen, screenshots still work, no display required.
  - `--state-home` isolates the XDG session store (like `--self-test` does).
  - The resolved socket path is printed to stderr: `pictura control: <path>`,
    followed by the session token file: `pictura control token: <path>.token`.
- **MCP frontend:** `pictura-mcp`.
  - Env: `PICTURA_CONTROL_SOCKET` (override), `PICTURA_BIN` (default
    `./build/pictura`), `PICTURA_ARGS` (default `--headless`).
  - On first tool call: if the socket exists → attach (do not kill on exit);
    else spawn `${PICTURA_BIN} --control --control-socket <path> ${PICTURA_ARGS}`,
    poll for the socket and a non-empty `<path>.token` with a timeout, and
    kill the child on MCP shutdown. Either way, read the token from
    `<path>.token` before the first request.
  - A `pictura_session` tool (optional v1) exposes `attach`/`launch`/`close`
    explicitly; auto-behaviour is the default.

### 3.4 Threading / event model

- `ControlServer` is constructed in `main()` after `PicturaMainWindow`, before
  `app.exec()`. It owns a `QLocalServer` and per-socket read buffers.
- All dispatch happens in `ControlServer` slots → Qt GUI thread → direct calls
  into `PicturaMainWindow`/`PictureView`. No locks, no cross-thread marshalling.
- Long operations (large composites, filters) already run synchronously on the
  GUI thread in the app today; the control call blocks until done and returns.
  (Off-GUI-thread compute is a future app-wide change, out of scope here.)

---

## 4. Control protocol

### 4.1 Framing

- Transport: `QLocalServer` (`QLocalServer::UserAccessOption`), one JSON object
  **per line**.
- Requests: `{"id": <int>, "method": "<string>", "params": { ... }, "token": "<hex>"}`.
- `token` is the session token (64 hex digits) the server writes when it
  starts listening to `<socket>.token`, beside the socket (in the temp dir on
  Windows, where the socket is a named pipe). The file is created fresh and
  owner-only (0600), and removed on shutdown; a new server writes a new token.
  A well-formed request with no token, or with a wrong one, is answered
  `unauthorized` with its `id` and is not dispatched. A malformed line is still
  `bad_request`. The in-process `dispatch()` used by the self-test and Qt tests
  needs no token.
- Success: `{"id": <int>, "ok": true, "result": { ... }}`.
- Failure: `{"id": <int>, "ok": false, "error": {"code": "<enum>", "message": "<text>"}}`.
- JSON is serialized **compact** (`QJsonDocument::Compact`) so a message never
  contains a raw newline.
- Unknown/malformed line → `bad_request` (never crashes the app).
- Multiple in-flight requests are not required for v1; process sequentially per
  socket. The MCP frontend has one connection.

### 4.2 Error codes

| Code | Meaning |
|---|---|
| `bad_request` | unparseable line, missing `method`, wrong param type |
| `unauthorized` | missing or wrong session `token` |
| `unknown_method` | method not in the catalog |
| `invalid_param` | value out of range / unknown enum |
| `no_document` | a document-requiring method with no active document |
| `not_implemented` | backing app capability is disabled or unimplemented |
| `refused` | the engine explicitly refused (e.g. locked/Background layer) |
| `io_error` | file open/save failure |
| `internal` | unexpected failure (logged, never aborts the app) |

### 4.3 Request examples

```json
{"id":1,"method":"status","params":{},"token":"<contents of <socket>.token>"}
{"id":1,"ok":true,"result":{"documents":1,"active":0,"active_tool":"Marquee","documents_info":[{"index":0,"name":"two_layers.psd","path":"...","width":8,"height":8,"mode":"rgb","depth":8,"dirty":false,"layers":2,"selection_px":0,"zoom":1.0}],"backend":"gpu","gpu_available":true,"brightness":1,"screen_mode":"standard","panels":[...] }}

{"id":2,"method":"screenshot","params":{"scope":"canvas","max_dim":1024},"token":"…"}
{"id":2,"ok":true,"result":{"mime":"image/png","base64":"iVBORw0K...","width":8,"height":8,"source_width":8,"source_height":8}}
```

---

## 5. Control method catalog (app side)

All methods return a JSON object. `(v)` marks "verify this backing accessor
exists before use" — most do, based on `main.cpp` and `cxxqt_object.rs`.

### 5.1 State / inspection

| Method | Params | Result | Backing |
|---|---|---|---|
| `status` | — | documents, active index, per-doc info, active tool, backend, gpu_available, brightness, screen_mode, panel names/visibility, active doc layer/selection summary | `PicturaMainWindow` getters + `PictureView` getters |
| `get_pixel` | `x`, `y` | `{argb:"0xAARRGGBB", r,g,b,a}` | `PictureView` composited sample (`sample_argb`/`image().pixel`) |
| `list_layers` | — | array `{index,name,kind,visible,opacity,blend,fill,lock,color}` | `layer_count`/`layer_name`/`layer_kind`/`layer_visible`/`layer_opacity`/`layer_blend`/`layer_fill`/`layer_lock`/`layer_color` |
| `list_commands` | `implemented_only?` | array `{id,path[],label,implemented,enabled,checked}` | new `CommandRegistry::describe()` |
| `ui_tree` | `max_depth?`, `max_children?` | array of `{class,object_name,rect{x,y,w,h},visible,enabled,text,tooltip}` (window-local rects) | `QObject::findChildren<QWidget*>` recursion over `frame` |

### 5.2 Vision

| Method | Params | Result | Backing |
|---|---|---|---|
| `screenshot` | `scope` = `window`\|`canvas`\|`document`; `max_dim?` | `{mime:"image/png", base64, width, height, source_width, source_height}` | `frame.grab()` / `ImageView::grab()` / `PictureView::image()` |
| `layer_thumbnail` | `index`, `size?` | same image shape | `PictureView::layer_thumbnail(index,size)` `(v)` |

`scope` semantics:
- `window` — the whole frame (menus, panels, status bar, canvas).
- `canvas` — the active `ImageView` only (checkerboard, zoom, selection overlay,
  move preview) — best for "does it look right".
- `document` — the raw composited document image (no chrome/checkerboard/overlay).

`max_dim` downscales the returned PNG (default cap **1280** px on the long edge)
to protect model context; the original dimensions are still reported.

### 5.3 Actions

| Method | Params | Result | Backing |
|---|---|---|---|
| `dispatch_command` | `id` | `{dispatched, enabled}` + refresh | `CommandRegistry::dispatch(id)`, `refresh()` |
| `edit` | `op` = `undo`\|`redo`\|`step_backward`\|`step_forward` | `{ok, can_undo, can_redo, depth}` | `PictureView::undo/redo/can_undo/can_redo/history_depth` |
| `set_tool` | `tool` (id name, e.g. `"Marquee"`) | `{active_tool}` | `PicturaMainWindow::setActiveTool` + `tools.h` parsing `(v)` |

### 5.4 Documents

| Method | Params | Result | Backing |
|---|---|---|---|
| `document` | `op` = `open`\|`new`\|`save`\|`save_as`\|`revert`\|`close`\|`activate`\|`list` | `{ok, active, count, path?, name?}` | `openPath`/`newDocument`/`saveActive`/`saveActiveAs`/`revertActive`/`closeDocument(index,false)`/`setActiveDocumentIndex`/`documentCount` |

- `open`: `path`
- `new`: `name?`, `width`, `height`, `mode?` (default `rgb`), `depth?` (8),
  `background?` (`white`|`transparent`)
- `save_as`: `path`; `save`: active path (or `path` → Save As)
- `close`/`activate`: `index`
- **Never** uses `showOpenDialog()`/`showNewDocumentDialog()` (modal). Always
  the non-interactive overloads, matching `--self-test`.

### 5.5 Unsaved-prompt policy

| Method | Params | Result | Backing |
|---|---|---|---|
| `set_unsaved_policy` | `interactive` (bool), `choice?` = `cancel`\|`discard` | `{interactive, choice}` | `pictura::setUnsavedPromptInteractive`, `setNonInteractiveUnsavedChoice` |

Control mode sets `interactive=false`, `choice=discard` at startup so
automation never blocks on a modal; the tool lets an agent deliberately test the
prompt by flipping it back.

### 5.6 Input synthesis

| Method | Params | Result | Backing |
|---|---|---|---|
| `pointer` | `op` = `click`\|`dblclick`\|`move`\|`drag`\|`scroll`; `space` = `window`\|`image`; `x`,`y`,`x2?`,`y2?`,`button?`,`modifiers?`,`steps?` | `{ok}` | synthesized `QMouseEvent`/`QWheelEvent` via `QApplication::sendEvent` |
| `key` | `sequence` (e.g. `"Ctrl+Z"`, `"B"`, `"Shift+F2"`) | `{ok}` | synthesized `QKeyEvent` to `frame` |

Details:
- `space=window` → coordinates relative to `PicturaMainWindow` (what `ui_tree`
  reports).
- `space=image` → document coordinates on the active `ImageView`; mapped through
  `ImageView::widgetToImage`/the inverse transform.
- `drag` sends press → N moves → release.
- Use `QApplication::sendEvent`/`postEvent` (never link `Qt6::Test` into the
  shipped binary).
- Shortcut testing routes through the real event/focus path; method-level
  actions use `dispatch_command`/bridge calls.

### 5.7 Selection / engine

| Method | Params | Result | Backing |
|---|---|---|---|
| `selection` | `op` = `all`\|`deselect`\|`rect`\|`ellipse`\|`lasso_begin`\|`lasso_point`\|`lasso_end`\|`quick`\|`wand`; `x`,`y`,`w`,`h`,`mode?`,`tolerance?` | `{ok, has_selection, count, bounds?}` | `select_all`/`deselect`/`select_rect`/`select_ellipse`/`begin_lasso`/`lasso_add_point`/`end_lasso`/`quick_select`/`magic_wand`/`selection_count`/`selection_bounds` |
| `filter` | `kind`, `seed?`, `params?` (reserved) | `{ok, kind}` | `PictureView::apply_filter(kind)`; v1 uses the app's fixed in-range defaults (`filter_from_kind`) |
| `adjustment` | `kind` | `{ok, layers}` | `PictureView::add_adjustment(kind)` |
| `layer_op` | `op` = `set_name`\|`set_opacity`\|`set_visible`\|`set_blend`\|`set_fill`\|`set_lock`\|`set_color`\|`move`\|`translate`\|`delete`\|`duplicate`\|`add`; `index`, `value`, `dx?`, `dy?` | `{ok, layers}` | `set_layer_*`, `move_layer`, `translate_layer`, `remove_layer`; `duplicate_layer`/`add_layer` `(v)` |
| `set_gpu_compute` | `on` | `{gpu_compute, backend}` | `set_gpu_compute`/`gpu_compute`/`active_backend` |

`filter` params are deliberately deferred: the app currently exposes filters as
kinds with fixed defaults. Exposing typed params requires plumbing
`pictura_filters::Filter` through the bridge; do it after v1 (see §12).

### 5.8 Session (optional v1)

| Method | Params | Result | Backing |
|---|---|---|---|
| `session` | `op` = `info`\|`save` | `{path, layout_bytes, brightness}` | `saveSession`, `loadSession`, `saveState` |

---

## 6. MCP tool catalog

Tool names are prefixed `pictura_` to avoid collisions. Each maps to one control
method (thin). Param structs derive `serde::Deserialize` + `schemars::JsonSchema`.

| MCP tool | Control method | Annotations | Returns |
|---|---|---|---|
| `pictura_status` | `status` | read-only | text (JSON) |
| `pictura_get_pixel` | `get_pixel` | read-only | text |
| `pictura_list_layers` | `list_layers` | read-only | text |
| `pictura_list_commands` | `list_commands` | read-only | text |
| `pictura_ui_tree` | `ui_tree` | read-only | text |
| `pictura_screenshot` | `screenshot` | read-only | **image content** + text metadata |
| `pictura_layer_thumbnail` | `layer_thumbnail` | read-only | **image content** |
| `pictura_dispatch_command` | `dispatch_command` | mutating | text |
| `pictura_edit` | `edit` | mutating | text |
| `pictura_set_tool` | `set_tool` | mutating | text |
| `pictura_document` | `document` | mutating | text |
| `pictura_set_unsaved_policy` | `set_unsaved_policy` | mutating | text |
| `pictura_pointer` | `pointer` | mutating | text |
| `pictura_key` | `key` | mutating | text |
| `pictura_selection` | `selection` | mutating | text |
| `pictura_filter` | `filter` | mutating | text |
| `pictura_adjustment` | `adjustment` | mutating | text |
| `pictura_layer_op` | `layer_op` | mutating | text |
| `pictura_set_gpu_compute` | `set_gpu_compute` | mutating | text |
| `pictura_session` | `session` | mutating | text |

Tool descriptions must state the non-interactive caveat (no dialogs) and the
coordinate spaces for `pointer`.

### 6.1 Resources

| URI | Content |
|---|---|
| `pictura://status` | latest `status` snapshot (JSON) |
| `pictura://commands` | `list_commands` output (JSON) |
| `pictura://layers` | active document layers (JSON) |

### 6.2 Prompts

- `pictura_testing_playbook` — a short guide the agent can load: coordinate
  systems, how to verify with `get_pixel`/`screenshot`, determinism rules
  (seeded filters, `set_gpu_compute`), the non-interactive caveat, the fixture
  (`two_layers.psd`, 8×8, red TL / blue BR / transparent TR,BL).

---

## 7. Security model

- **Local only:** Unix-domain socket under `$XDG_RUNTIME_DIR` (or a user-chosen
  path) created with `QLocalServer::UserAccessOption` (0600). No TCP listener.
- **No eval:** there is no "run code" tool. Actions are a fixed typed surface.
- **Session token (#844):** every request must carry the token from the
  owner-only `<socket>.token` file. On POSIX the server unlinks whatever entry
  is at that name and creates the file with `O_CREAT | O_EXCL | O_NOFOLLOW`, so
  a planted symlink is never followed. The comparison is constant-time. A
  same-user process that can reach the socket but not read the file gets no
  `document open/save` oracle; one that can read the user's files can read the
  token too, so the token does not stop a process with that access, which
  could reach those files directly anyway.
- **Same file access as the app:** `document open/save` can read/write anywhere
  the invoking user can. Document this clearly; a future `--control-root`
  allowlist is the hardening path if agents get untrusted.
- **Opt-in:** the server exists only with `--control`. No effect on normal runs
  or `--self-test`.
- **No secrets in logs:** stderr names the socket and token-file paths, never the
  token itself; nothing is logged that the app would not already log.

---

## 8. Screenshot / vision implementation notes

- Encode with `QImage` → `QBuffer` → `QImageWriter("PNG")` → `toBase64()`.
- Sources:
  - `window`: `frame.grab().toImage()`
  - `canvas`: `frame.imageView()->grab().toImage()` (active canvas or error)
  - `document`: `frame.activeView()->image()`
- Downscale with `QImage::scaled(..., Qt::KeepAspectRatio, Qt::SmoothTransformation)`
  when `max_dim` is exceeded.
- Return both `width`/`height` (returned image) and `source_width`/`source_height`.
- MCP side: decode base64 into `ContentBlock`/`RawContent::Image` with
  `mimeType: "image/png"` plus a text block carrying the metadata.
- Size guard (ponytail ceiling): default `max_dim=1280`; raise per call only if
  the agent needs to read small pixels. A 4000×4000 canvas must never be sent
  raw.

---

## 9. `ui_tree` implementation notes

- Recurse `frame->findChildren<QWidget*>()` (or a manual DFS from `frame`) up to
  `max_depth` (default 6) and `max_children` (default 64 per node).
- Per node: `metaObject()->className()`, `objectName()`, `mapTo(frame, QPoint(0,0))`
  + `size()` (window-local rect), `isVisible()`, `isEnabled()`,
  `property("text")`/`QAbstractButton::text()`/`QLabel::text()`, `toolTip()`.
- `max_depth`/`max_children` caps keep the payload bounded; a full CS6 frame has
  thousands of widgets.
- This is the semantic alternative to vision-only computer use: the agent can
  resolve `layersPanel` and click its center, then confirm with a screenshot.
- Later (optional): use `QAccessible` roles/names for a11y-accurate trees and
  add an accessibility smoke check (see `docs/11-cross-cutting/testing-strategy.md`).

---

## 10. File-by-file change list

### 10.1 New

| File | Purpose |
|---|---|
| `crates/pictura-app/cpp/control.h` | `pictura::ControlServer` declaration |
| `crates/pictura-app/cpp/control.cpp` | QLocalServer, framing, dispatch, screenshots, ui_tree, event synthesis |
| `crates/pictura-mcp/Cargo.toml` | binary crate: `rmcp`, `tokio`, `serde`, `serde_json`, `schemars`, `anyhow` |
| `crates/pictura-mcp/src/main.rs` | stdio serve loop, tool router, resource/prompt handlers |
| `crates/pictura-mcp/src/control_client.rs` | connect/launch/poll, send line, parse response |
| `crates/pictura-mcp/src/tools.rs` | `#[tool]` handlers + param structs |
| `crates/pictura-mcp/tests/stdio.rs` | `initialize` → `tools/list` → `tools/call status` |
| `scripts/verify-control.sh` | end-to-end recipe driver |
| `opencode.json` (or documented snippet) | register `pictura` MCP server (local, stdio) |
| `.mcp.json` (project) | Claude Code registration (opt-in) |

### 10.2 Edit

| File | Change |
|---|---|
| `crates/pictura-app/cpp/main.cpp` | parse `--control`/`--control-socket`/`--state-home`; **suppress the `--headless`⇒`--self-test` implication in control mode**; construct `ControlServer`; `app.exec()`; add `mcp_control` self-test block |
| `crates/pictura-app/cpp/commands.h` / `commands.cpp` | add `QJsonArray describe() const` enumerating `entries_` (id, path, label, implemented, enabled, checked) |
| `CMakeLists.txt` | `find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Svg Network)`; add `control.cpp`/`control.h` to `add_executable(pictura ...)` |
| `Cargo.toml` | add `crates/pictura-mcp` to workspace `members` |
| `crates/pictura-app/cpp/frame.h` / `frame.cpp` | only if a `status` getter is missing (most exist); candidate: a public layer-summary helper |
| `AGENTS.md` | `## Agentic testing (MCP)` section: commands, socket, client config |
| `docs/INDEX.md`, `docs/TRACEABILITY.md` | index the new long-form spec |

### 10.3 `main.cpp` control-mode sketch

```cpp
// after arg parsing
bool control = false;
QString controlSocket;
QString stateHome;
// ... parse "--control", "--control-socket <p>", "--state-home <d>"

// do NOT let headless imply self-test when controlling:
if (headless && !selfTest && !control && psdPath.isEmpty()) {
    selfTest = true;
}

// ... construct frame, open doc, show ...

if (control) {
    // non-interactive automation defaults
    pictura::setUnsavedPromptInteractive(false);
    pictura::setNonInteractiveUnsavedChoice(pictura::UnsavedChoice::Discard);
    if (!stateHome.isEmpty()) qputenv("XDG_STATE_HOME", stateHome.toUtf8());

    pictura::ControlServer server(&frame, controlSocket);
    if (!server.listen()) {
        std::fprintf(stderr, "pictura control: FAIL: cannot listen\n");
        return 153;
    }
    std::fprintf(stderr, "pictura control: %s\n", qPrintable(server.socketPath()));
    std::fflush(stderr);
    return app.exec();
}
```

### 10.4 `control.h` sketch

```cpp
#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QHash>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>
#include <QtCore/QObject>
#include <QtCore/QString>

class QLocalServer;
class QLocalSocket;

namespace pictura {

class PicturaMainWindow;

// Opt-in JSON control endpoint for agentic testing. Serves newline-delimited
// requests on a local socket and dispatches on the Qt GUI thread.
class ControlServer : public QObject {
    Q_OBJECT
public:
    ControlServer(PicturaMainWindow* frame, const QString& socketPath,
                  QObject* parent = nullptr);
    ~ControlServer() override;

    bool listen();
    QString socketPath() const { return socketPath_; }

    // Pure dispatch, also used by the in-process self-test block.
    QJsonObject dispatch(const QString& method, const QJsonObject& params);

private slots:
    void onNewConnection();
    void onReadyRead();
    void onDisconnected();

private:
    QJsonObject error(const QString& code, const QString& message) const;
    QJsonArray grabPngBase64(const QString& scope, int maxDim);

    PicturaMainWindow* frame_;
    QLocalServer* server_;
    QString socketPath_;
    QHash<QLocalSocket*, QByteArray> buffers_;
};

} // namespace pictura
```

### 10.5 `pictura-mcp` skeleton

```rust
use rmcp::{ServiceExt, model::*, transport::stdio};
use rmcp::handler::server::router::tool::ToolRouter;

#[derive(Clone)]
struct Pictura { router: ToolRouter<Self>, client: ControlClient }

#[rmcp::tool_router(server_handler)]
impl Pictura {
    #[rmcp::tool(name = "pictura_status", description = "Active document and app state (JSON)")]
    async fn status(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        self.client.call_json("status", serde_json::json!({})).await
    }

    #[rmcp::tool(name = "pictura_screenshot", description = "Screenshot the window, canvas, or document; returns a PNG")]
    async fn screenshot(&self, Parameters(p): Parameters<ScreenshotParams>)
        -> Result<CallToolResult, rmcp::ErrorData> {
        let r = self.client.call_json("screenshot", serde_json::json!({
            "scope": p.scope, "max_dim": p.max_dim
        })).await?;
        // build an image ContentBlock from result.base64 / result.mime
        Ok(CallToolResult::success(vec![image_block(&r), text_meta(&r)]))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = Pictura::new(ControlClient::from_env()).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

> **Pin the exact `rmcp` version at P4.** The API has churned (0.x → 1.x/3.x):
> verify `serve`/`stdio`, `ContentBlock`/`RawContent` image construction, and the
> feature flags (`server`, `macros`, `transport-io`, plus `schemars` derive)
> against docs.rs for the pinned release.

---

## 11. OpenSpec + docs plan

### 11.1 New change `openspec/changes/mcp-agentic-control/`

```
proposal.md      why / what / capabilities
design.md        this document condensed + decisions
specs/agentic-control/spec.md   ADDED requirements
tasks.md         phased checklist (§13)
```

### 11.2 Capability `agentic-control` — candidate requirements

Each requirement needs SHALL wording + ≥1 `#### Scenario:` (four `#`).

1. **Opt-in control mode** — `--control` starts a local only socket; default
   `$XDG_RUNTIME_DIR/pictura-control.sock`; prints the path; no effect without
   the flag.
2. **Newline-delimited JSON protocol** — request/response shapes and error
   codes (§4).
3. **Non-interactive operation** — control mode never opens a modal dialog
   (`showOpenDialog`/`showNewDocumentDialog` unreachable); unsaved policy is
   non-interactive by default.
4. **State readback** — `status`, `get_pixel`, `list_layers`, `list_commands`,
   `ui_tree`.
5. **Vision readback** — `screenshot` with `window|canvas|document` scopes,
   `max_dim` cap, PNG image content over MCP.
6. **Action surface** — `dispatch_command`, `document`, `edit`, `set_tool`,
   `pointer`, `key`, `selection`, `filter`, `adjustment`, `layer_op`,
   `set_gpu_compute`.
7. **MCP frontend** — `pictura-mcp` serves stdio, attaches or launches, exposes
   the catalog as tools and the resources/prompt.
8. **Local-only security** — 0600 socket, no network listener, no eval tool.
9. **Verification** — in-process self-test block + end-to-end script + MCP
   stdio test.

### 11.3 MODIFIED requirement

- `verification-harness` — add a scenario that CI runs the control smoke check
  (`--self-test` `mcp_control` block) as part of the headless job.

### 11.4 Long-form docs

- `docs/11-cross-cutting/agentic-testing.md` (XC id TBD) — the human-readable
  contract: architecture, protocol, tool tables, security, examples.
- Update `docs/INDEX.md` and `docs/TRACEABILITY.md`.
- `AGENTS.md` `## Agentic testing (MCP)`:
  ```bash
  ./build/pictura --headless --control &
  ./build/pictura-mcp               # stdio MCP server (or via opencode.json)
  ```

---

## 12. Client registration

### 12.1 opencode

Project `opencode.json` (verify the exact schema key against opencode docs
before writing):

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "pictura": {
      "type": "local",
      "command": ["/mnt/work2/repos/kooka-pictura/build/pictura-mcp"],
      "enabled": true,
      "environment": {
        "PICTURA_BIN": "/mnt/work2/repos/kooka-pictura/build/pictura",
        "PICTURA_ARGS": "--headless"
      }
    }
  }
}
```

### 12.2 Claude Code / Claude Desktop

Project `.mcp.json`:

```json
{
  "mcpServers": {
    "pictura": {
      "command": "/mnt/work2/repos/kooka-pictura/build/pictura-mcp",
      "env": {
        "PICTURA_BIN": "/mnt/work2/repos/kooka-pictura/build/pictura",
        "PICTURA_ARGS": "--headless"
      }
    }
  }
}
```

Claude Desktop uses the same `mcpServers` object in
`claude_desktop_config.json` (Linux: `~/.config/Claude/claude_desktop_config.json`).
Document both in `agentic-testing.md`; do not overwrite a user's global config.

---

## 13. Phased task breakdown

### P0 — Proposal and docs (no code)

- [x] Write `openspec/changes/app-control-server/{proposal,design}.md` (the plan
      is split so each slice archives; this change is P0/P1).
- [x] Write `specs/agentic-control/spec.md` (ADDED, scenarios above).
- [x] Write `tasks.md` from §13.
- [ ] Draft `docs/11-cross-cutting/agentic-testing.md` (lands with P5).
- [x] `openspec validate --all --strict`.

### P1 — Control server skeleton

- [x] `control.{h,cpp}`: socket, framing, error codes, dispatch scaffold.
- [x] `main.cpp`: `--control`/`--control-socket`/`--state-home`, suppress
      headless⇒self-test, enter the event loop.
- [x] `CMakeLists.txt`: `Qt6::Network` + sources.
- [x] `CommandRegistry::describe()`.
- [x] Methods: `status`, `list_commands`, `list_layers`, `dispatch_command`,
      `document`, `edit`, `set_unsaved_policy`.
- [x] Self-test `mcp_control` block (next append-only code, 463+): start the
      server,
      round-trip `status`, `dispatch_command`, `list_commands`.
- [x] Gates: CMake build, `./build/pictura --headless --self-test`.

### P2 — Vision

- [x] `screenshot` (`window|canvas|document`, `max_dim`).
- [x] `layer_thumbnail`.
- [x] `ui_tree`.
- [x] `get_pixel`.
- [x] Self-test assertions (PNG magic bytes, dimensions, tree contains
      `layersPanel`).

### P3 — Input + engine actions

- [x] `pointer`, `key` (event synthesis) — archived
      `2026-09-23-agentic-control-input`. Image-space coordinates map through the
      inverse of `ImageView::widgetToImage`; Qt 6.11 routes a non-spontaneous key
      press through the shortcut machinery, so a shortcut fires in an active
      window and the widget handler runs otherwise.
- [x] `set_tool`, `selection`, `filter`, `adjustment`, `layer_op`,
      `set_gpu_compute` (archived `2026-09-23-agentic-control-actions`).
- [x] Self-test: the marquee-drag (`pointer`) and arrow-nudge (`key`) halves
      plus the `no_document` and `invalid_param` cases are asserted in the
      in-process `mcp_control` block; the shortcut path is left to the live
      `verify-control.sh` (P5).

### P4 — `pictura-mcp`

- [ ] New crate; pin `rmcp`; tool router + param structs.
- [ ] `ControlClient`: attach, launch (`PICTURA_BIN`/`PICTURA_ARGS`), poll,
      teardown.
- [ ] Resources (`pictura://status|commands|layers`) + `pictura_testing_playbook`.
- [ ] `tests/stdio.rs`: `initialize` → `tools/list` → `tools/call status`.
- [ ] Gates: `cargo fmt/clippy`, `cargo nextest`.

### P5 — Registration + e2e

- [ ] `opencode.json` + `.mcp.json` (or documented snippets).
- [x] `scripts/verify-control.sh`: launch `--headless --control`, drive
      status→rect-select→`add-noise`→64-pixel diff (changed inside the selection,
      none outside)→undo→screenshot, fail on mismatch (archived
      `2026-09-23-agentic-control-e2e`). The recipe uses a rect that is a strict
      subset of the active layer so a mask-ignoring filter fails (the plan's
      `wand` selects the whole layer and could not).
- [x] Wire the script into `scripts/verify-full.sh` and the CI `qt-headless` job.
- [ ] `AGENTS.md`, `agentic-testing.md`, `INDEX.md`, `TRACEABILITY.md`.

### P6 — Verify, archive, commit

- [ ] `scripts/verify-fast.sh`, `scripts/verify-full.sh`.
- [ ] `cargo nextest run --workspace`, `cargo test --workspace --doc`.
- [ ] `openspec validate --all --strict`.
- [ ] `bash scripts/guard.sh` (docs → `TASK-ALLOWS-DOCS`).
- [ ] `openspec archive mcp-agentic-control`; commit.

---

## 14. Verification matrix

| Check | Where | What it proves |
|---|---|---|
| `mcp_control` self-test block | `main.cpp`, CI headless job | socket + framing + core methods work in-process, offscreen |
| `scripts/verify-control.sh` | CI (`qt-headless` job) or manual | full recipe through the live app: open → select → filter → pixel assert → undo |
| `pictura-mcp` stdio test | `cargo nextest` | MCP handshake, `tools/list`, a real `tools/call` |
| `openspec validate --all --strict` | CI | spec/change format |
| `guard.sh` | CI | docs gated, no non-goal artifacts |

Recipe for `verify-control.sh` (uses the existing fixture):

```
open two_layers.psd            → status: 2 layers, 8x8
wand(6,6,tol=10)               → selection count > 0 and < 64
apply add-noise (seed 1)       → pixel(6,6) changed
                                → pixel(2,2) unchanged (outside selection)
undo                           → pixel(6,6) restored to the pre-filter value
screenshot(scope=canvas)       → PNG, 8x8 source
```

---

## 15. Risks, ceilings and ponytail notes

- **Dependency weight (isolated).** `rmcp`+`tokio`+`schemars` sit only in
  `pictura-mcp`; the app gains `Qt6::Network` (already installed, 6.11.1) and no
  Rust deps. `// ponytail:` justification: official SDK beats hand-rolled MCP
  protocol drift; a hand-rolled stdio server is the fallback if the dep is ever
  rejected.
- **Screenshot size.** `// ponytail: default cap 1280 px, raise per call` — a
  4000² raw PNG would blow model context.
- **Blocking GUI thread.** Long composites/filters block the control call. Fine
  for testing; the upgrade path is app-wide off-GUI-thread compute (separate
  work).
- **Dialog reachability.** Only non-interactive paths; if a future tool wants a
  dialog, it needs a non-interactive overload first.
- **Wayland/global coordinates.** `ui_tree` returns window-local rects and
  `pointer space=window` is frame-local, avoiding global-coordinate portability
  problems.
- **GPU determinism.** `set_gpu_compute` + seeded filters; the playbook tells the
  agent to pin CPU when comparing pixels.
- **Tool schema drift.** The MCP tool set is thin over the control methods; keep
  the mapping 1:1 so a new control method is one small tool wrapper.
- **`filter` params** are deferred until typed `Filter` params are plumbed
  through the bridge; v1 kind+seed covers the test loop.
- **One instance per socket.** A fixed default path means a single controlled
  instance; `--control-socket` enables parallel sessions.

---

## 16. Open questions

1. **`rmcp` version + exact API** — pin at P4 and verify `serve`/`stdio`/image
   content/feature names.
2. **`pictura-mcp` crate name** — `pictura-mcp` (binary) vs a bin target inside
   `pictura-app` (keeps the workspace smaller; but pulls tokio into the app
   *crate graph*, not the app binary). Recommendation: separate crate.
3. **`--control` visibility default** — always require explicit `--control`, never
   auto-enable. (Recommend: yes, explicit only.)
4. **`ui_tree` size cap** — confirm defaults (depth 6 / 64 children) against the
   real CS6 frame; tune if payloads exceed context.
5. **Expose `run_recipe(steps[])`?** — a batch tool would cut round-trips;
   defer until the single-step loop proves the need.
6. **Streamable HTTP transport** — only if remote/containerized agents are
   required; stdio covers local use.
7. **Docs spec id** — assign the next free XC id for `agentic-testing.md`.

---

## 17. Appendix — reference calls already proven by `--self-test`

These sequences in `crates/pictura-app/cpp/main.cpp` are the ground truth for
the backing calls (use them when wiring `control.cpp`):

- document lifecycle, dirty state, tabs, unsaved prompt — exit codes 33–37;
- tools + marquee/lasso/quick select/crop/move/eyedropper — 39–46;
- command dispatch + enablement + theme/screen/session/panels — 25–32;
- new doc + save/open round-trip — 33–35;
- adjustments, masks, filters, undo/redo, history — 9–24;
- document ops (rotate/canvas/resize) — 19–21;
- layers (name/opacity/visible/blend) — M20 exit codes 50–52;
- GPU compute + backend + session — 70/71;
- filter GPU parity — 72/73;
- transparency/clipping/region refresh/present cache — 74–78;
- layer attributes (fill/lock/color) — 86–89.

When adding `mcp_control`, reuse these exact call shapes so the control surface
and the self-test assert the same behavior.
