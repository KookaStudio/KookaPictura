# Tasks

## 1. Loading banner

- [x] 1.1 Add `documentOpenStarted(const QString& path)` to `frame.h` as a
  `Q_SIGNALS` test seam, the `openBanner_` member with `openBannerTextForTest` /
  `openBannerVisibleForTest` hooks, and the private `beginOpenIndicator` /
  `endOpenIndicator` / `positionOpenBanner` declarations.
- [x] 1.2 Create `crates/pictura-app/cpp/frame_open.cpp` and move `openPath`,
  `openImagePath`, `isNativeDocumentPath`, `openDocumentAtPath`, and
  `openAsSmartObjectPath` there verbatim (a pure move; `frame.cpp` is at the
  1200-LOC hard cap). Add `frame_open.cpp` to
  `crates/pictura-app/cpp/CMakeLists.txt`.
- [x] 1.3 Implement the banner: a lazily built `QLabel#documentOpenBanner`
  showing `Opening <file>…`, shown at the window's lower-left above the status
  bar with a wait cursor, painted (repaint + `processEvents` excluding user
  input) before the blocking load, then hidden when the open finishes. Add the
  `QLabel#documentOpenBanner` rule in `theme.cpp` (notice tokens, rounded
  corners) and reposition on show/resize in `frame_columns.cpp`.

## 2. Tests

- [x] 2.1 Add `crates/pictura-app/cpp/tests/tst_open_loading_banner.cpp` and
  register it in `crates/pictura-app/cpp/tests/CMakeLists.txt`: the banner text
  and visibility for a PSD, a raster, and Open As Smart Object
  (`bannerShownForEachOpenKind`); hidden and cursor restored on success and
  refusal (`bannerClearsOnSuccessAndRefusal`); a notice replaces the banner
  (`noticeReplacesBanner`, via a minimal flat CMYK PSD).
- [x] 2.2 Add `tst_design_system::openBannerUsesTheNoticeStyle`, asserting the
  `#documentOpenBanner` rule carries the notice background, the dark notice
  text, and a border radius.

## 3. Integration

- [x] 3.1 Configure and build with CMake, then run
  `ctest --test-dir build -R '^tst_(open_loading_banner|design_system)' --output-on-failure`.
- [x] 3.2 Run `openspec validate --all --strict` and
  `bash scripts/verify-fast.sh`; confirm fmt, clippy, the unified report,
  file-size, guard, and the self-test budget all pass.
