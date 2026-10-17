# Proposal

## Why

Opening a very large image or PSD/PSB blocks the GUI thread inside
`PictureView::open` / `open_image` with no repaint, so the window looks frozen:
no message, no cursor change, no sign that work is happening (issue #667). The
load is synchronous, so the event loop cannot paint until it finishes; the only
place to give feedback is *before* the blocking read/decode.

## What Changes

- Before the blocking read/decode of any document open, the frame shows a
  **loading banner** at the window's lower-left corner — a rounded box on the
  theme's notice yellow with dark notice text, reading `Opening <file name>…` —
  plus a wait cursor. The banner is painted before the load starts, so it is
  visible while the (blocking) open runs, and it stands out clearly over the
  canvas.
- The banner covers every open entry point — File > Open, Open Recent, a file
  drop, and Open As Smart Object — for both PSD/PSB (native reader) and raster
  (Qt decode) sources.
- When the open finishes the banner is hidden and the wait cursor restored,
  whether the open succeeded or was refused. A successful PSD/PSB open that
  reports a notice (for example `Converted from CMYK; saved as CMYK`) shows that
  notice in the status bar.
- The load stays on the GUI thread: this is a visible banner, not a threading
  change. During the load the banner is static; the window is not made
  interactive.

Not a breaking change: an open shows no indicator today, so this only adds
feedback. The capability spec is the contract.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities

- `document/document-lifecycle`: add an "Open shows a loading banner"
  requirement beside "Open documents".

## Impact

- App C++ (the only affected layer; no engine crate or Qt bridge change):
  - `crates/pictura-app/cpp/frame.h` — `documentOpenStarted` signal, the banner
    member, its test hooks, and the indicator helpers.
  - `crates/pictura-app/cpp/frame_open.cpp` (new) — the open entry points
    (`openPath`, `openImagePath`, `isNativeDocumentPath`, `openDocumentAtPath`,
    `openAsSmartObjectPath`) moved out of `frame.cpp` as a pure move, plus
    `beginOpenIndicator` / `endOpenIndicator` / `positionOpenBanner`.
    `frame.cpp` is at the 1200-LOC hard cap, so the moved methods make room.
  - `crates/pictura-app/cpp/frame_columns.cpp` — reposition the banner on resize
    and show.
  - `crates/pictura-app/cpp/theme.cpp` — the `#documentOpenBanner` stylesheet
    rule (notice tokens, rounded corners).
  - `crates/pictura-app/cpp/CMakeLists.txt` — register `frame_open.cpp`.
  - `crates/pictura-app/cpp/tests/tst_open_loading_banner.cpp` (new),
    `crates/pictura-app/cpp/tests/tst_design_system.cpp` — the Qt Test coverage.
- No new dependencies.
- No `docs/` change.
