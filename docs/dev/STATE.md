# STATE — project resume anchor

Snapshot for resuming after a context break. Update after each milestone.

## Where things are

- Repo: `github.com/KookaStudio/KookaPictura`, branch `master`.
  A working Rust/Qt engine and the documentation-first corpus it was built
  from. Licensed GPL-3.0-or-later; the corpus is a documentation-first
  independent reimplementation, not a formal two-team process
  (`docs/00-overview/licensing-and-provenance.md`).
- **Provenance history:** the git history was rewritten so no reachable commit
  contains reproduced Adobe Help prose or the old wording.
- **Warp Text** (#684, branch `feat/684-warp-text`, OpenSpec change
  `warp-text`, archived into `tools/warp-text`). `Type > Warp Text…` and the
  type tools' options-bar Warp button open a window-modal dialog: Style (None
  and the fifteen presets), Horizontal/Vertical, Bend, and H/V Distortion
  (−100…+100 %).
  - **Preview, Cancel, OK.** Every change previews on the canvas with no
    history state. The first preview stashes the layer, and Cancel puts that
    copy back, so an opened layer's own pixels survive. OK records one "Warp
    Text" state.
  - **Rendering** (`pictura-render/src/type_warp.rs`): glyph outlines are
    flattened and mapped through `TextWarp::over(text box)`, then the layer
    matrix, then filled with zeno. Edges stay as sharp as unwarped type, and
    the layer rect bounds the warped ink.
  - **Live warp.** `TypeSetting` carries the warp, and `TypeFrame::load` /
    `store` now own the frame↔setting copy. Retyping, panel edits, styles,
    point/paragraph conversion, and Free Transform keep it.
  - **PSD.** `decode_type_warp` reads the `TySh` warp descriptor, and the
    author writes it.
  - **Refusals.** Faux Bold, fonts with no `glyf`/`CFF `/`CFF2` table, and
    on-path or area type are refused with a non-blocking warning.
  - **Layers panel.** A warped row's T card stands on a bent baseline.
  - **Placeholder text** (OpenSpec change `type-placeholder-text`, archived
    into `tools/type-placeholder-text`; Photoshop CC behaviour, not CS6). A new
    Horizontal or Vertical Type session, from a click or a dragged box, starts
    with "Lorem Ipsum Pictura" selected. Typing replaces it, and committing it
    untouched keeps it. Type Mask and reopened layers start as before.
    `Edit > Preferences > Type` is now a real page whose Fill New Type Layers
    With Placeholder Text checkbox (on by default, saved in the session) turns
    it off. The Warp Text button has its own `options.warpText` icon, and
    `options.warp.svg`, which was never listed in `pictura.qrc`, is now
    bundled.
  - **Character panel on new type** (change `character-panel-new-text`).
    While new text is being set, the Character panel edits that session (a
    `TextSession` override the preview and the commit share), so toggles
    preview at once. Before, it edited the previously selected layer, if any.
  Ceiling (`ponytail:`): the caret, the selection highlight, and the box stay
  unwarped while a warped layer is edited. The outlines are rebuilt on every
  render. CS6's refusal of path type is unconfirmed. Tests: `tst_warp_text`
  (16 cases) and Rust `type_warp`, codec, and core units.
- **Help > Search on Shift+Space** (#788, branch `feat/788-searchbar`,
  OpenSpec change `ui-search-bar`, archived into `ui/search-bar`). Not a CS6
  feature; added by owner decision and labelled non-parity.
  - **A registry command:** `help.search` is bound to `Shift+Space`, so the
    Keyboard Shortcuts editor rebinds it and `list_commands` sees it. Text
    fields and type edits keep the key through `ShortcutOverride`, and the
    window's Space pan ignores a Space pressed with Shift.
  - **The index** (`search_index.cpp`, widget-free): every command (live
    label and enabled state), each panel once, the real Preferences pages,
    every tool, and the curated control table in `search_controls.cpp`.
    Matching ANDs the query words; results rank by exact label, prefix, word
    prefix, substring, then path or keyword matches, with runnable entries
    first. The index is rebuilt every time the bar opens.
  - **The overlay** (`search_bar.cpp`) is an in-window `QFrame`, not a popup.
    Enter or a click runs a result; Alt+Enter opens the menu chain or outlines
    the panel, control or toolbox slot (`flashWidget`). Greyed rows do not run.
    The status bar carries a `Search` keycap that follows the binding.
  - **Tests:** 20 Qt cases in `tst_search_bar`.
- **System Info and Submit Issue** (#815, branch
  `feat/815-system-info-panel`, OpenSpec change `system-info-panel`, archived
  into `ui/system-info`). `Help > System Info` opens a modeless snapshot report
  with Copy: Pictura version/build, Qt, OS/kernel/architecture, desktop/session,
  CPU model/cores/clock, RAM/swap/system CPU, the process's RAM/CPU, the wgpu
  adapter's name/type/backend/driver, and each display's model/resolution/size/
  DPI/refresh. `Help > Submit Issue` sits below it, copies the report, and opens
  the repository's new-issue page prefilled with it; a failed browser launch
  names the clipboard fallback. Linux `/proc` supplies the CPU/memory/process
  rows; other platforms show `Unavailable`. The report excludes the hostname,
  user name, monitor serials, network addresses, environment-variable dumps,
  and document paths. VRAM, GPU clock, desktop build version, and monitor type stay
  `Unavailable` (no portable API). Tests: `tst_system_info` plus a Rust
  `gpu_adapter_info_never_panics`; opening the real browser is a manual check.
- **Justified text renders justified** (#792, branch
  `fix/792-justify-text-renders-left`, OpenSpec change
  `2026-10-21-justify-text-renders-left`). A type layer whose paragraph
  justification was 3 to 6 - Justify Last Left, Justify Last Right, Justify
  Last Center and Justify All - rendered left-aligned: the renderer collapsed
  the seven justification values the engine models to three alignments, and
  nothing spread a line to the column width. The codec, the Paragraph panel
  and the Paragraph Style dialog already carried all seven, so the panel's own
  choices did nothing.
  - The renderer keeps the justification kind. A justified line's word spaces
    take the leftover width, so the line's advance reaches the column; the
    paragraph's last line is the one line a justification does not spread, and
    its variant aligns it.
  - The column is the paragraph box's width. A point-type layer has no box and
    so no column: its lines are not spread, and the variant still selects the
    line's alignment. A line with no word space to spread stays flush left.
  - The vertical placement follows the same table read as CS6's vertical
    justification: Justify Last Left is the top, Justify Last Right the
    bottom, Justify Last Center the centre.
  - The caret measures the same spread, so it stands between the painted
    glyphs; converting a justified paragraph to point text drops the spread
    and anchors the lines as the paragraph's last line was.
  Ceiling (`ponytail:`): the word-spacing min/desired/max bounds, the letter
  spacing and the glyph scaling are not applied - the spread divides the
  leftover width evenly - the spread is computed for left-to-right text, a
  line with no word space is never spread, and the Qt `Rasterize Type` path
  still collapses the justify variants to `Qt::AlignLeft`. A point-type
  layer's lines follow the variant's alignment, so a layer that already
  carried Justify Last Right is now right-aligned.
- **Plain-Rust document session** (#740, branch
  `refactor/740-rust-document-session`, OpenSpec change `document-session`,
  archived into `document/document-session`). The document, the selection, and
  the history live in a `DocumentSession`; the `PictureView` QObject is the
  adapter that presents what the session reports, and `cargo test` is the
  second adapter.
  - **One live session:** the live session is one owned `ActiveSession` value
    in the `DocumentSession`, with a variant per kind — a begin of another kind
    is refused, a repeat begin follows its kind's rule (a transform replaces
    its own session, a filter or mode preview updates it, a lasso attaches to
    the magnetic gesture it started), and one `cancel_active_sessions` drops
    the live variant.
  - **No C++ copy:** the tool controller reads the transform drag handle from
    the session over the bridge instead of keeping its own drag state.
  - **Cancel first:** a replacement cancels against the outgoing document,
    clears every outgoing pointer, preview, and cache, and installs the
    incoming document, history, and PDF password; history navigation cancels
    too.
  - **A change report:** the session returns a `SessionChange` (the effect and
    the regions); the adapter emits the region blits and at most one
    `changed`.
  - **The tool handlers reset:** `sessionCancelled` reaches every registered
    handler, so a Pen anchor, a Path Selection drag, a Move Alt-clone, or a
    Type edit cannot commit into a restored document.
  - **No stale installs:** an open request carries an id, so a result from a
    superseded request is dropped instead of installing over a newer document.
  - **Tests:** 27 Rust session tests, 12 Qt cases in `tst_document_session`,
    and the stale-request guard.
  - **Follow-up:** the headless display frame is #783.
- Toolchain: Rust 1.98 (`rust-toolchain.toml`), system Qt **6.11.1**, cxx-qt
  **0.10.0**, wgpu **30.0.1**, lcms2 **6.2.0** (system Little CMS 2.19).
- **Documentation sweep: the README, the roadmap and the corpus against the
  shipped code** (#785, branch `docs/785-reconcile-readme-roadmap-corpus`,
  OpenSpec change `documentation-corpus`, archived into
  `meta/documentation-corpus`). The roadmap and the README described a project
  that had not been built yet. The erasers, gradient and paint bucket, clone and
  pattern stamp, dodge/burn/sponge, blur/sharpen/smudge, the type tools, the
  healing family, the pen and path tools, the shape tools, magnetic lasso,
  rotate view, the slice tools, the colour sampler/ruler/note/count tools,
  perspective crop, Save As and Export, Save for Web, recent files, align and
  distribute, and clipping masks were all listed as planned; all of them ship.
  The corpus's own banner said "No code exists in this repository" in 137 spec
  files, and `docs/README.md` claimed the repository has no code, no build
  files and no dependencies. What changed:
  - `ROADMAP.md` is re-cut into Shipped, Open and Not planned. "Open work"
    groups the gaps in `docs/dev/doc-code-parity-gaps.md` by domain, and
    every open item links the issue tracking it; the Phase 1/Phase 2/Later
    framing is gone, because it no longer described anything.
  - `README.md`'s feature list covers the shipped surface, each bullet citing a
    path in the tree, and its two over-claims now carry the real counts: 110
    filters with three CS6 families still without an engine, and the classic
    adjustment families, each naming the control it still lacks.
  - The "no code exists" banner is gone from all 137 corpus files; each now
    says the repository holds the implementation, that the file stays the
    behavioural contract, and that the gap register records what shipped.
    `docs/README.md` and `00-overview/product-overview.md` say the same, and
    `00-overview/feasibility-and-non-goals.md` no longer lists counting as a
    non-goal, because the Count tool ships.
  - Five rows of the gap register were wrong and are fixed: `FILT-1` (the
    Lighting Effects engine ships) and `LAY-7` (rubylith shipped in #672) are
    removed, two "code ahead of docs" rows are dropped because the specs caught
    up, and §E's "linked/embedded objects — verified absent" now says the
    codec parses the linked records.
  - This file's known-risks list had nine claims the tree had moved past —
    history cloning the whole document, the Layers panel showing top-level rows
    only, marching ants, the Filter Gallery, brush presets, the recent-files
    menu, the Info colour samplers, the OS font picker and the paint-mode list
    — and they are corrected.
  - New `scripts/check-doc-claims.py` plus two `scripts/guard.sh` rules fail a
    corpus file that denies the implementation, an open roadmap item that names
    no issue, and a cited path that does not exist.
  - New `AGENTS.md` rule 12 requires a pull request that ships or removes a
    feature to update `ROADMAP.md` in the same change.
  Ceiling: the corpus's own `Status: Draft` labels are left alone — they
  describe the maturity of a spec document, not the state of the code — and the
  corpus's broken path references and mis-bound `ARCH-0xx` dependency ids are
  left for a separate sweep.
- **Perspective rebuilt, pending-transform lock** (#750, branch
  `fix/750-perspective-transform`, OpenSpec change
  `perspective-transform-gesture`, archived into `document/free-transform` and
  `document/transform-reference-point`). No photorust/photocraft source
  existed, so Perspective was rebuilt from the owner's CS6 screen recording:
  - **Gesture** (`impl_transform/perspective.rs`): a corner slides along the
    edge the drag follows most, re-picked on every move, and that edge's other
    corner slides the other way about its midpoint. Corners may cross into a
    bowtie. An edge handle slides only its edge. The reference point keeps its
    bilinear place, and the drag shows a `∠` readout. Editable type is refused
    and the menu disabled.
  - **Free Transform shortcut:** Ctrl+Alt+Shift on a corner does the same
    drag. It stores a source-space `base_quad` after which the similarity
    applies, so plain drags still scale (in the box's projective frame) and
    rotate it.
  - **Crossed quad** (`layer_ops/crossed_quad.rs` plus the CPU preview warp):
    fills the quad's own outline, two straight triangles meeting at the
    crossing, instead of the homography's wrap-around.
  - **Preview fix:** the preview divides by the projective divisor itself,
    since `QTransform::map` clamps a negative one.
  - **Lock and prompt (all transforms):** while a modal transform is pending,
    the Layer menu is gated (`CommandRegistry::setBlockedProvider`, dispatch
    refused) and the Layers panel is greyed. A tool switch, by click or
    shortcut, asks "Apply the transformation?" (Apply / Cancel / Don't Apply);
    headless and the control server answer Don't Apply without a modal.
  - **Inferred, not oracle-verified:** the readout sign, the base composition,
    and the pixel layout inside a crossed quad (the recording used a flat
    layer).
  - **Ceiling (`ponytail:`):** a mask outside the layer rect takes its default
    colour on a crossed quad.
  - **Tests:** `tst_transform_perspective` (14), `tst_transform_lock` (7), Rust
    gesture/engine units.
  - **Self-test:** `lpr_free_transform`'s Perspective sub-check now expects the
    new rule. `core float_screen_bounds` (437) fails locally on its own and is
    unrelated.
- **Warp ported from photocraft** (#751, branch `fix/751-warp-algorithm`,
  OpenSpec change `port-photocraft-warp`, archived into
  `document/transform-warp-engine`, `document/transform-session-warp`,
  `ui/transform-options-bar`, with the old warp requirements removed from
  `document/free-transform`). The preset dialog, `apply_warp_preset`, the
  Patchy preset nets and the 16×16 cell rasterizer are gone. The port has three
  parts:
  - `pictura_core::warp` is photocraft's `geom::warp`: styles fitted to
    Photoshop renders, and the bicubic Bezier mesh with pull and point drag.
  - `layer_ops/warp_raster.rs` is photocraft's adaptive triangle resampler:
    premultiplied, area-averaged when shrinking, and parallel by 64-row bands.
  - Warp is a Free Transform mode (`impl_transform/warp.rs`). The menu or the
    options-bar toggle starts a one-patch mesh over the source box and keeps
    the box's quad as a projective frame after it (review fix: a baked patch
    bent a Perspective box). The options bar
    shows the style, orientation, and Bend/H/V. The canvas draws the grid and
    points.

  Split Warp and the Grid menu (post-CS6) are not ported, and smart-object and
  Warp Text warps are unchanged. Self-test `lpr_warp_preset` (exit 527) is
  retired, and Qt Test `tst_transform_warp` covers the mode. Ceiling: the
  preview re-warps the full-resolution layer on every edit (`ponytail:` in
  `refresh_warp_preview`). Credit is in `NOTICE.md` and
  `docs/03-tools/move-and-transform.md`.
- **Distort transform parity** (#748, branch `fix/748-transform-distort`,
  OpenSpec change `distort-transform-parity`, archived into
  `document/free-transform` and `document/transform-reference-point`).
  `Edit > Transform > Distort` now moves the grabbed corner by the
  press-to-pointer delta (keeping the grab offset), stops at the last convex
  quad, carries the reference point with the box, snaps corners to visible
  guides within 8 screen px, and refuses editable type (menu disabled) unless a
  selection lifts its pixels. The projective engine (`QuadMap`) and the live
  preview clip to the front side of the horizon, so a direct quad past it no
  longer samples the mirrored far side. Qt's projective `drawImage` could drop
  a triangle at some angles, so the live preview is CPU-warped
  (`image_view_transform_warp.cpp`) and the Qt projective draw is only a
  fallback. Perspective and Skew are unchanged. Ceilings (`ponytail:`): Distort
  snaps whenever guides are visible because
  `View > Snap` / `Snap To > Guides` are stubs; gate it on that state when it
  exists. Tests: `tst_transform_distort` (10), Rust gesture/engine units; the
  self-test's Distort/Perspective/Skew checks still pass.
- **Large brush and shape performance** (#719, branch
  `fix/719-large-brush-perf`, OpenSpec change `large-tool-performance`,
  archived into `tools/banded-dab-raster`, `tools/bounded-tool-commit`,
  `verification/tool-profiler`, plus ADDED requirements in
  `compositing/vector-mask-sampling` and `interop/agentic-control`). The
  shell's Brush/Pencil/Eraser start through `paint_tools::begin` and never took
  the preview or GPU stroke (only `begin_paint`, used by the self-test, does);
  instead the exact raster now runs a dab of 128² px or more in parallel row
  bands (`stroke/raster.rs`, byte-identical; Dissolve serial). In-stroke
  presents patch `doc.composite`, so `end_paint` presents only the pending
  region and `cancel_stroke` re-composites the extent. `VectorMask::inside`
  answers rows beyond the path without the per-pixel edge scan (the ellipse's
  10 s on the world map). New shape layers and `fills::apply` refresh only
  their changed rect via `refresh_edit` (full recomposite when another layer
  is styled, or a clipped layer sits above a new shape); `fill_layer` runs rows
  in parallel; shape coverage sweeps only the path's columns.
  `scripts/profile-tools.py` times tools end to end over the control server
  (new `tool_options`; `pointer` `down`/`up`) with `--save`/`--compare`;
  before/after tables and remaining ceilings (undo, Paint Bucket, Marquee
  panel refresh, Gradient) are in the change's `design.md`.
- **Hex shorthand, Add to Swatches, and swatch persistence** (#603, branch
  `feat/603-hex-shorthand-swatches`, OpenSpec changes
  `hex-shorthand-swatches` and `swatches-session-persistence`, archived into
  `ui/color-picker` and `ui/color-swatches-panel`). The picker's hex field
  accepts `#`/`0x` and 3-digit shorthand (`#123` → `#112233`): six digits apply
  live, shorthand applies on Enter/focus loss, a commit normalises the field,
  and invalid input leaves the colour and restores the field. Add to Swatches
  opens a name prompt defaulting to the panel's next `Swatch N`, appends the
  current colour, and leaves the picker open; `PicturaMainWindow` registers its
  `SwatchGrid` as the picker's default target so every picker entry point
  reaches the panel. `SwatchGrid::nextSwatchName` also names the panel's New
  swatches. The grid persists in the session store: `SessionState` carries
  `swatches` (`{name, color}`) plus a `swatchesLoaded` flag, the frame saves
  the live grid on every add/delete/reset and restores it in `buildPanels`; an
  absent key opens the default palette and a stored empty panel stays empty.
  Ceilings (`ponytail:`): `.aco`/`.ase` libraries and a dedicated preferences
  file remain with #605/#489; 4/8-digit alpha shorthand, the Color panel hex
  field, and the CC-only library checkbox are out of scope. Tests:
  `tst_color_picker` (6), `tst_swatches_panel` (9), and `tst_session_store`
  (3 new).
- **Spin Blur** (#464, branch `feat/464-spin-blur`, OpenSpec change
  `spin-blur-gallery`, archived into `imaging/blur-gallery` and
  `imaging/blur-filters`). A post-CS6 Blur Gallery effect ported on the
  owner's request from their CC screenshots, labelled non-parity.
  `pictura_filters::blur::spin` (`SpinPin { x, y, radius_x, radius_y,
  angle, feather, blur_angle, pivot_x, pivot_y }`, `SpinStrobe { strength,
  flashes, duration }`, `MAX_SPIN_ANGLE = 360`, `Filter::SpinBlur`): a
  feathered elliptical mask (1 inside the feather ring, smoothstep to 0 at
  the outline, largest mask wins) and an arc gather of up to 128 bilinear
  taps round the rotation centre — the full Blur Angle inside the feather
  ring, past it the ring's arc length × mask, so the smear shortens toward
  the outline (owner's CC comparison) — with strobe
  flashes of a set angular duration; alpha included, Noise after.
  `Filter::BlurGallery` gains `spin` / `spin_strobe` (spin pass after the
  path pass; Noise takes the largest weight; no spins is byte-identical).
  App: `spin-blur` (26 canonical) and `blur-gallery` (62: spin section after
  the paths, then motion and one flash-duration slot; spins share the
  motion strength and flashes). `BlurGalleryMode::Spin`: Iris's outline
  without the diamond, the marker on the rotation centre with an
  angle ring (1°/px), Alt-drag moves the rotation centre; the Spin Blur row
  (Blur Angle `0..=360°`, default 15) and Strobe Flash Duration on the
  Motion Effects tab (shown for Spin only); spin code in
  `blur_gallery_spin.cpp`. Ceilings (`ponytail:`): mask, arc gather, flash
  windows, shared strobe, ranges, linear sliders, no drag-duplicate.
  Tests: 7 spin units + 2 gallery units, two new kernel digests (existing
  unchanged), a no-equivalent oracle row, mapping tests, `tst_spin_blur`
  (7) and the gallery / filter-menu suites updated for the live row.
- **Generic PDF / EPS import and Open As** (#627, branch
  `feat/627-eps-pdf-import`, OpenSpec change `pdf-eps-import`, archived into
  `interop/pdf-eps-import` and `interop/open-as`). Ghostscript (`gs`, or
  `PICTURA_GHOSTSCRIPT`) runs as a `-dSAFER` child process, never linked
  (`pictura_codec::ghostscript`); absent, opening a PDF/EPS reports an error
  naming it. `pictura_codec::pdf::import`: page boxes and `/Rotate` from a
  `runpdfbegin`/`pdfgetpage` PostScript program (PDF box defaults applied, all
  in rotated view space), Bounding Box from the `bbox` device, pages rendered
  with `pngalpha` at per-axis resolution and cropped to the box (a minimal PNG
  reader in `png.rs`), thumbnails in one run. Images mode: a scan-based object
  reader (`N G obj` in file order, object streams; no xref parsing) lists
  image XObjects (through Form XObjects) and wraps each in a one-page PDF for
  Ghostscript; encrypted PDFs disable it. `pictura_codec::eps`: DOS binary
  header, `%%HiResBoundingBox`/`%%BoundingBox` (`(atend)` too), `-dEPSCrop`.
  App: routing by signature (`vector_sniff`; a Pictura Photoshop PDF still
  reopens with layers first, then Photoshop's own: `pdf::photoshop_psd`
  rebuilds a PSD from `/PieceInfo /AdobePhotoshop /Private`, change
  `photoshop-pdf-layers`), password prompt (`-sPDFPassword` in a 0600 `@file`, never argv), the Import
  PDF dialog (`ImportPdfDialog`, CS6 layout and defaults from the owner's
  screenshots: Bounding Box, 300 ppi, Inches, RGB, 8 bit, Small) and Rasterize
  Generic EPS Format dialog sharing `RasterSizeFields`; each page/image opens
  as an untitled document with one transparent `Layer 1`, mode/depth via
  `convert_mode`/`convert_bit_depth` after an 8-bit RGB render (ponytail);
  several pages are named `<stem>-<n>.<ext>`; Ghostscript's `****` warnings
  show in one box unless Suppress Warnings. `File > Open As…` (`file.openAs`)
  checks the chosen format's signature and reports "format mismatch or damaged
  file". Agentic `open` imports with the dialog defaults, no modal. The Flatpak
  builds Ghostscript 10.06.0 as its own module.
- **Release build hardening** (#703 / #710, OpenSpec change
  `release-build-hardening`, archived into `meta/release-build-hardening`). All
  six packagers (`appimage`, `linux-packages`, `snap`, `flatpak`, `windows`,
  `macos`) now use unique concurrency groups, a fork gate that requires the
  release-please head branch in this repository, read-only PR dry runs with a
  separate `contents: write` upload job, SHA-pinned actions and pinned
  downloads, and the same tag-normalization expression in each workflow.
  Artifacts are exercised in the form users receive them: the AppImage in a
  clean `ubuntu:24.04` container, the `.deb`/`.rpm` installs and the snap
  install each run the self-test, the Flatpak bundle's self-test, the Windows
  portable zip and silent-installed NSIS installer (with the install marker
  guard and uninstall round-trip), and the mounted macOS DMG. The Flatpak lint
  job also runs `scripts/check-flatpak-cargo-sources.py`, and snap asserts the
  tag matches `version.txt`. `README.md`, `DEVELOPING.md`, and the parity
  register were updated. The temporary PR dry-run trigger widening used to
  exercise all six workflows was reverted before merge, so feature PRs still
  skip the full builds.
- **Photoshop PDF writer** (#626, branch `feat/626-photoshop-pdf-writer`,
  OpenSpec change `photoshop-pdf-writer`, archived into `interop/pdf-export`).
  `pictura_codec::pdf` writes one page holding the rendered composite as one
  image: page size from the resolution; Gray / RGB / DeviceCMYK (the
  profile-free CMYK source, inverted) / Lab / Indexed / 1-bit CCITT G4 by
  output mode, Multichannel refused; ICC tagging (v2 at PDF ≤ 1.4); soft-mask
  alpha or flatten onto white; None / ZIP / JPEG (encoded at the Qt boundary,
  `encode_jpeg`; JPEG2000 listed but disabled); Average / Subsampling / Bicubic
  downsampling; 16-bit at PDF ≥ 1.5; thumbnails; notes as text annotations;
  XMP; PDF/X-1a/X-3/X-4 and PDF/A-1b output intents and info keys; standard
  security R2/R3/R4/R6 (`md-5`, `aes`, `sha2`; RC4 and CBC hand-written);
  linearization with hint tables (`qpdf --check-linearization` clean).
  Options are one `PdfOptions` serialized as canonical `.joboptions` (also the
  preset file format and the C++↔Rust interchange: `pdf_option` /
  `pdf_set_option` / `pdf_option_locked`), with twelve inferred built-in
  presets. Preserve Photoshop Editing Capabilities embeds Pictura's own PSD in
  `/PieceInfo /Pictura` (approximation); `File > Open` / Revert of such a PDF
  restores the layers (`pdf_open`), any other PDF stays #627. App: Save As
  `Photoshop PDF (*.PDF *.PDP)` → `PdfSaveDialog` (Save options, preset,
  standard, compatibility; General / Compression / Output / Security / Summary),
  the path becomes the `.pdf`, Save on it reopens the dialog with that
  document's settings, `Edit > Adobe PDF Presets…` (`PdfPresetsDialog`, user
  presets in `<state dir>/PDF Presets/`). CMYK destination and PDF/X intent:
  PhotoCraft's synthetic CC0 profile (`assets/icc/`). CI installs
  `poppler-utils`, `ghostscript`, `qpdf` for the render / structure /
  linearization / decryption oracles (renders sampled at 4× pixel centres:
  poppler filters images at 1:1).
- **Paragraph type and the type caret** (#682 / #681, branch
  `feat/541-advanced-type-tools`, OpenSpec changes `paragraph-type` and
  `type-caret-blink`, archived into `tools/paragraph-type` and
  `tools/type-caret`). #541 is split into #682 (paragraph type), #683 (type
  on a path), #684 (Warp Text), and #685 (gamma blend, on by default at 1.45
  with goldens rebaselined, plus missing fonts). `TypeSpec` / `TypeTool`
  carry `box_bounds` (left, top, right, bottom relative to the origin, before
  the matrix). `pictura_render::type_paragraph::visual_lines` is the single
  line source for layout, placement, caret stops, and baselines: greedy word
  wrap to the box width (character break for a too-wide word), vertical
  columns of `⌊h / cell⌋`, and lines past the box are overflow (laid out for
  the caret, not rendered, `type_overflows`). The EngineData gains `/Rendered`
  with `ShapeType` 0/1 and `BoxBounds` (`pictura_codec::type_shape`), read
  back on open. The tool opens a box on a drag past 4 screen px or on an
  Alt-click (Paragraph Text Size dialog). Its eight handles resize and
  reflow, and a `+` on the bottom-right handle marks overflow. `Type >
  Convert To Point / Paragraph Text` are live (`frame_menus_type.cpp`). The
  caret line blinks on a 1.5 s cycle (`ImageView::kTypeCaretBlinkMs`, reset
  on every overlay set). Point type marks its origin with a placement square
  and draws a guide along each visible line's baseline (`type_baselines`).
- **Type on a path and area type** (#683, branch
  `feat/683-type-on-a-path`, OpenSpec change `type-on-path`, archived into
  `tools/type-on-path`). `TypeSpec` / `TypeTool` carry `path` (`TextPath`:
  one `Subpath`, `OnPath` or `Area`, and the click's arc-length start).
  `pictura_render::type_path` lays on-path glyphs by arc length, rotates their
  outlines by the tangent, wraps a closed path once, and reports overflow at
  the path end; area type reuses the paragraph layout in the path's bounding
  box and clips its alpha to the closed interior. The Type tools hit the Work
  Path and saved paths (`type_path_at`), start at the click, show the path as
  the baseline guide, and draw the overflow `+`. The Work Path stays visible
  with the Type tools, hovering its edge shows the type-on-path cursor, and
  Move / Free Transform carry the stored path and scale the type. Path type
  round-trips through
  the document-level `Txt2` `ResourceDict.TextFrameSet` (one entry per text
  layer, `Type` 2 on-path / `Type` 1 area), read by
  `pictura_codec::type_path`; the `ag-psd` oracle reads the authored structure.
  Ceilings (`ponytail:`): one subpath per session, hard line breaks become
  spaces, area type clips a bounding-box layout rather than reflowing each line
  to the shape, and there is no start-marker drag or side flip.
- **Additional brush presets and the pen-tip preview** (#731, branch
  `feat/731-paintbrush-types-pen-tip`, OpenSpec changes
  `paintbrush-types-pen-tip` (archived into `ui/brush-preset-picker` and
  `ui/brush-tip-preview`), `paintbrush-preset-defaults`,
  `pencil-grain-texture`, `paintbrush-reference-texture`,
  `paintbrush-oval-wet-edges`, `two-b-pencil-reference-profile`, and
  `non-photo-blue-pencil-reference-profile` (archived into `ui/brush-preset-picker` and
  `tools/paint-engine`)). The Brush Preset
  picker appends the owner's four brushes — `2B Pencil` (8 px, hardness 25,
  25 % spacing, grain 50, `#47484a`), `Airbrush Soft Round 50% flow` (45 px, `#47484a`,
  Flow 50, 25 % spacing so its core stays light), `Non-Photo Blue Pencil`
  (4 px, hardness 0, 20 % spacing, grain 55, `#73d6ff`), and `Paintbrush Oval 45 pixels
  Multiply` (45 px, hardness 100, roundness 50, angle −50, 5 % spacing,
  Wet Edges, `#47484a`, Multiply, Opacity 35, Flow 50) —
  after the CS6 set (48 total). Selecting one of the four applies its tip and
  those default paint settings; the existing CS6 presets still change only
  the tip. The options bar re-reads Opacity, Flow, and Mode through
  `PaintOptions::paintSettingsChanged`, so a preset never leaves stale values
  there. Tip values are approximated from the issue's screenshots (no
  `.abr`).
  The two pencils set `PaintOptions::penTip` (persisted with the brush
  options), which shows a 2D pen-nib model (`cpp/brush_tip_preview.*`) at the
  active canvas's top-left while the Brush or Pencil tool is active. The
  overlay is a child of each document's `ImageView`, hides for other
  tips/tools/documents, and is draggable within the canvas without starting a
  stroke. Both pencils also set `PaintOptions::grain`, which the stroke engine
  applies as a fixed alpha modulation keyed to the layer coordinates. The
  noise is `fill::grain_noise` (a per-pixel hash blended with 2 px and 3 px
  bilinear value noise, weighted 0.2 / 0.35 / 0.45 at contrast 1.3 to match
  the 2B reference's clumps), so the 2B Pencil's 50 % grain leaves about 75 % mean
  core coverage and the blue pencil's 55 % grain about 72 %, reading as
  light, clumpy paper texture rather than static; the soft edge stays
  anti-aliased (the Pencil tool's own aliased edge is separate), and grainy
  strokes stay on the CPU path. The options bar's tool icon lists the four added presets with their
  tip icons; the adjacent tip button opens the full Brush Preset picker.
  The oval is measured from the owner's ~770 % pixel-grid screenshot. On white,
  a lone dab paints 26 below white, a stroke's interior 20, and its rim about
  26. `StrokeConfig::wet_edges` reproduces this with `tip::wet_edge`
  (`1.3·a·(1 − 0.76·a)` on the accumulated coverage, fitted to those two
  tones). Wet-edge strokes stay on the CPU path, and the large-dab preview
  applies the same curve. `PaintMode::Multiply` replaces the Brush's old
  Normal fallback for `"multiply"`; the other unmapped Brush modes still paint
  as Normal. In this engine a positive tip angle tilts the major axis
  down-right, so the preset uses −50 for Photoshop's up-right tilt. Stroke
  caps show 2–3 stepped rings where Photoshop draws one thin rim.
  The 2B Pencil is fitted the same way (~670 % screenshot): a ~7 px stroke
  with soft shoulders integrating to ~3.7 px of coverage. At 100 % flow our
  accumulation saturates overlapping dabs, so a larger or denser tip paints a
  flat top; Photoshop's dab-raises-toward-its-own-alpha accumulation is not
  modelled. The Non-Photo Blue Pencil is fitted the same way: a ~3.5 px line
  integrating to ~1.8 px of coverage with peaks around 0.74.
  `tst_brush_tip_preview` covers the preset data, the quick list, and the
  overlay; ceilings: the brush values are non-parity approximations, and the
  preview is painted 2D, not Photoshop's 3D Brush Preview.
- **Tool feedback and persistence** (#541 branch, OpenSpec change
  `tool-feedback-and-persistence`). Show Transform Controls is **on by
  default** (owner decision; CS6 ships it off). The controls box is now
  **idle**: it is drawn from `pictura_render::content_rect` (no session, layer
  untouched), and a Free Transform session (with its trim) starts only when a
  handle is grabbed. `untrim_to` pads a trimmed layer back when a session ends
  without a transform, and type layers are never trimmed. The box no longer
  steals focus, and Space pans over it. Every tool's options persist in the
  session store's `toolOptions` (`tool_options_json`: one `fields()` list per
  struct, restored before the options bar is built). The caret draws white
  over a dark image. A type or shape layer's forced Transparency/Image locks
  show no row padlock (`LockBadgeRole`). A Move drag draws marching ants round
  the content plus a ΔX / ΔY readout in the ruler unit (`formatRulerLength`,
  shared with the rulers). The self-test, and older tests that read canvas
  pixels under the Move tool, run with the box off.
  While typing, the pointer away from the text is the Move tool's cursor
  (`pointerCursor` / `hoverCursor` on `TypeToolHandler`). A drag there moves
  the session origin, so the text, caret, guides, and box follow; a plain click
  still commits (change `type-session-move`).
  The session's placement lives in `TypeFrame` (type_frame.{h,cpp}): paragraph
  type's box, and point type's text bounds while Ctrl is held, with handles
  (resize the box, or scale the text with Ctrl or on point type), a rotate band
  just outside it, and a draggable reference point, each with its cursor and a
  W / H, angle, or X / Y readout (`ImageView::measureReadout`). The transform
  is the layer's own `TySh` matrix (change `type-transform-box`).
- **SVG export parity** (#477, branch `feat/477-svg-export-parity-followups`,
  OpenSpec change `svg-export`, now archived into `interop/svg-export`): the
  SVG export from #459 walks the layer tree instead of top-level layers only.
  `pictura_codec::svg` serializes a tree of `SvgNode { content, opacity, blend,
  clip }` with `SvgContent::Group { children, isolated }` (document-unique clip
  ids; an isolated group with no opacity/blend carries `isolation:isolate`).
  `pictura_render::svg_nodes` / `encode_document_svg` build it, taking a
  `raster` closure that composites a scratch document (the app bridge passes
  `composite_active`). Rules (design D10): a stack is a base plus its clipped
  layers; an adjustment layer, a non-CSS blend, a non-plain clipping stack, or a
  faded/masked pass-through group flattens its container's layers up to it into
  one raster (a pass-through group defers to its parent); a plain clipping stack
  is one raster; a raster fallback no longer bakes in the opacity its `<g>`
  re-applies (v1 applied it twice); a shape scales its fill by fill opacity and
  rasterizes with effects or a pixel mask. Text stays a raster (decision).
  Oracle: `pictura-render/tests/svg_oracle.rs` renders a vector document with
  ImageMagick's `RSVG:` coder (librsvg) and compares with the CPU composite on
  white (2% / mean 1.5 edge tolerance; self-skips without the coder). Docs:
  `export-formats.md` *SVG (CC)*. Not done (#652): gradient/pattern
  `<defs>`, `vstk` strokes, per-layer SVG files.
- **Semantic theme tokens** (change `semantic-theme-tokens`, issue #419): every
  key emitted by `Theme::tokens` and substituted into the stylesheet is renamed
  from an implementation-surface name to a semantic role name in kebab-case —
  39 colour tokens (`panel`→`surface`, `window`→`surface-recessed`,
  `hover`→`surface-hover`, `inputBg`→`surface-input`, `border`→`border`,
  `graphFill`→`graph-fill`, …), 8 metrics
  (`panelBorderWidth`→`border-width-panel`, `slotBaseW`→`slot-base-width`, …),
  and the 3 pseudo-tokens (`${tabFontSize}`→`${font-size-tab}`,
  `${inputFontSize}`→`${font-size-input}`, `${selectChevron}`→`${select-chevron}`).
  The `Ramp` fields take the same names, so `theme.cpp` reads coherently;
  values, token count, both families, and the brightness levels are unchanged —
  a pure rename. The six custom-painted widgets' `Theme::tokenColor` keys, the
  in-app design-system browser's key lists and descriptive `note()` prose, the
  drift guard (`tst_design_system`), and `scripts/gen-styleguide.py`'s
  `var(--…)` set are updated, and `design-system.html` is regenerated (idempotent;
  it also gains the four paint-only tokens it was missing). The full one-to-one
  mapping is archived in the change's `design.md`. Verified: build green, 4394
  Rust tests and the Qt design-system suites pass, `verify-fast` green.
- **Windows C2026 stylesheet fix** (#474, branch `fix/474-windows-c2026-theme-qss`,
  OpenSpec change `fix-windows-c2026-theme-qss`, ADDED requirement in
  `meta/windows-release-build`): `theme.cpp` built its stylesheet from one
  `QStringLiteral(R"(...)")` literal, which had grown to ~19.7 KB — over MSVC's
  16,380-byte per-literal cap (error C2026) that applies before adjacent
  literals are concatenated. GCC/Clang accept the longer literal, so the failure
  only surfaced on the Release PR's `windows` job. The literal is now two
  `QStringLiteral` pieces joined with `+` at a CSS section boundary, assembling
  byte-for-byte the same stylesheet. New `scripts/check-msvc-string-limits.py`
  (wired into `guard.sh`) fails when any single C++ string literal under
  `crates/pictura-app/cpp` exceeds 16,000 bytes, so the breakage is caught on
  Linux before a Windows runner. The `windows.yml` job's release-please-only
  `if:` is relaxed **temporarily** (the C++/packaging paths added to its
  `paths:`) so the fix builds on a normal PR; this scaffolding is reverted within
  the change once the Windows job goes green, so the change's final diff leaves
  `.github/workflows/windows.yml` with no net diff. Verified: the split
  round-trips byte-identically, the guard fails on the pre-split file, and its
  `--self-test` passes.
- **Path Blur** (#462, branch `feat/462-path-blur`, OpenSpec change
  `path-blur-gallery`): a **post-CS6** Blur Gallery effect brought into scope
  by owner decision and labelled non-parity (row tooltip, workspace hint,
  `blur-gallery.md`). `pictura_filters::blur::path` (`BlurPath`,
  `PathMotion`, `Filter::PathBlur`) blends each path's Catmull-Rom tangent
  and arc-length-interpolated end speeds into a gridded motion field
  (inverse-square, as Field Blur's pins) and gathers up to 64 bilinear taps
  along its streamline (Centered Blur or trailing), weighted by Taper,
  Strobe Strength/Flashes, and Rear Sync Flash; alpha included, then Noise.
  `Filter::BlurGallery` gains `path`/`motion`: the paths blur after the
  radius effects, Bokeh stays on the radius weights (inferred), and the
  no-path output is byte-identical (the old gallery digest held). Slots:
  `path-blur` 27 canonical, `blur-gallery` 51. `BlurGalleryMode::Path`
  (`blur_gallery_path.cpp`, `blur_gallery_canvas_path.cpp`): the Path Blur
  row (Basic Blur / Rear Sync Flash / Custom, Speed, Taper, Centered Blur,
  End Point Speed for a selected end, a disabled Edit Blur Shapes), a live
  Motion Effects tab, and paths with points and an end arrow: clicks on
  empty picture pin a new path's stops while the curve follows the pointer
  (double-click / Enter / Esc ends it), under a hand-drawn `cursor.pinAdd`
  pushpin; drag a point, press the curve to insert one, Ctrl-drag the
  path, Delete. Selecting Path Blur disables the Effects tab (Bokeh) and
  enables Motion Effects; the Noise checkbox now defaults on in every
  gallery mode (Amount 0, so no output changes). Opening path and defaults
  follow the owner's CC screenshots. The workspace's minimum size is now its
  layout's (`QLayout::SetMinimumSize`, the hint label clipping), so
  squeezing it never crushes a slider or value box, and the Blur Tools rows
  are an accordion (opening one closes the open one): at least 756 px wide
  and 594–754 px tall, Path Blur's row the tallest
  (`squeezingKeepsEveryControlReadable`). Ceilings (`ponytail:`): the motion
  field, gather, flash weights, ranges, and gestures are inferred; Edit Blur Shapes is not built
  (blur shapes follow the tangent); Spin Blur stays a stub. Tests: 15
  engine units + 2 gallery units, two new kernel digests, a no-equivalent
  oracle row, mapping tests, `tst_path_blur` (11), and the gallery suites
  updated for the live row. Verified: `TASK_ALLOWS_DOCS=1 bash
  scripts/verify-fast.sh` OK — nextest 3120 passed / 23 skipped, app
  self-test 510, Qt 675, unified 4305 passed · 24 skipped · 0 failed;
  `openspec validate --all --strict` 263/263.
- **macOS release build fix** (#461, branch `fix/461-macos-rhi-header`, OpenSpec
  change `fix-macos-release-build`; new spec `meta/macos-release-build`, ADDED
  requirement in `ui/application-shell`): `interop.cpp`'s `#include
  <rhi/qrhi.h>` failed only on the macOS framework Qt because the RHI include
  dir came from a hand-built `qmake6 -query` path that matches Debian alone.
  CMake now links the exported `Qt6::GuiPrivate` target and drops the
  header/version query; a private-header Qt install (`qt6-base-private-dev` on
  Debian; CI's `install-qt-action` already has it) is now a prerequisite
  (`DEVELOPING.md`). `interop.cpp` now guards its Vulkan-only includes behind
  the same `QT_CONFIG(vulkan)` condition as its probe body, because the
  official macOS Qt is built without Vulkan and ships no `QVulkanInstance`
  header. `macos-bundle` now copies Qt's offscreen QPA plugin into the bundle
  before `macdeployqt`, so the `--headless` smoke check runs from the bundle
  (linux/windows already deployed it). `pictura-render`'s `request_device` now
  returns "no GPU" when wgpu has no implemented backend for the target, instead
  of panicking in `Instance::new` on macOS. The `macos.yml` job's release-please-only `if:` is relaxed
  **temporarily** (the C++/packaging paths are added to its `paths:`) so the fix
  builds on a normal PR; both edits are reverted within the change once the
  macOS job goes green (`.github/workflows/macos.yml` has no net diff). New `tst_native_menu_bar` asserts the shell never opts out of the
  native menu bar and that screen modes do not change the presentation.
  Verified: `interop.cpp` compiles against `Qt6::GuiPrivate`,
  `tst_native_menu_bar` passes offscreen. Follow-up: `appimage.yml:40` carries
  the same release-only gate.
- **Frameless dialogs and Keyboard Shortcuts** (#85, branch
  `feat/85-frameless-dialogs`, OpenSpec change
  `frameless-dialogs-keyboard-shortcuts`): `installDialogChrome()`
  (`dialog_chrome.cpp`, installed in `main.cpp`) is one app event filter
  that makes every Qt-drawn `QDialog` frameless at polish and adds a
  `dialogTitleBar` (title, close = reject, `startSystemMove` drag) and a
  `dialogFrame` outline; the title height is added to the layout's top
  margin, so no dialog changed. Native/portal pickers are exempt (a picker
  counts as Qt-drawn when `DontUseNativeDialog` is set or its widget tree
  exists at polish). `KeyboardShortcutsDialog` edits Application Menus and
  Tools: an in-cell focused key field, several bindings per command
  (`CommandRegistry::shortcuts`/`setShortcuts`), cross-list conflicts (a
  tool letter owns its Shift+letter), refused `workspaceKeys()`, and an
  inline dark-on-yellow `notice` label (theme tokens `noticeBg` /
  `noticeText`). Overrides persist as `shortcuts` / `toolShortcuts` in the
  session store; `rebuildToolShortcuts()` skips any key a menu command
  holds. Ceilings: no named sets, Summarize, Panel Menus, legacy channel
  keys, or Menus tab; edge resize only on the dialog's own margin band.
- **Isolation mode** (#112, OpenSpec change `isolation-mode`; new spec
  `ui/isolation-mode`, MODIFIED `compositing/layers-filtering-search`):
  Photoshop's Isolate Layers shipped as the Layers panel's **`Selected`** filter
  type. The filter row gains the dimension (no criteria control — its criterion
  is the isolated set); entering it captures the selection as a **fixed set**
  shown alone in the panel, and scopes the Move tool's Auto-Select
  (`auto_select_path` takes the set; a hit outside it keeps the active layer).
  The layer row context menu offers **Release from Isolation** (drops a row; the
  last one ends isolation), the Move tool's canvas right-click offers
  **Isolation Mode**, and Escape / the toggle off / a document switch exit. It is
  view-only: no document mutation, no history state, reset on document switch.
  Ceilings (`ponytail:`): adding a layer to the set from the canvas is not built
  (Release only shrinks it), and the Move canvas menu carries only the Isolation
  Mode entry, not the CC layer picker. The docs contract
  (`05-layers/layer-filtering-and-search.md`, `02-ui-ux/panels/layers-panel.md`,
  `GLOSSARY.md`) moves `Selected` from the CC-only exclusion into scope by
  decision (the CS6 Help does not list it). Tests: Rust `#[test]`s for the scoped
  Auto-Select (`tests_auto_select.rs`) plus `tst_layers_panel::isolationMode`.
  Verified: nextest 3100 passed / 23 skipped (the one failure is the pre-existing
  local ImageMagick fixture mismatch), ctest 89/89, app self-test 474,
  `openspec validate --all --strict` 260/260.
- **Export As (CC)** (#457, branch `feat/457-export-as-dialog`, OpenSpec change
  `export-as-dialog`; new spec `ui/export-as-dialog`, MODIFIED
  `interop/raster-export`): `File ▸ Export As…` is now the CC dialog — a layer
  list (the flattened document plus each top-level layer, adjustment layers
  excluded, each with a checkbox), a preview with Fit / 1:1 / ± zoom, **File
  Settings** (Format PNG / JPG / PNG-8 / GIF, with Transparency for the alpha
  formats and Quality for JPG), **Image Size** (linked Width/Height/Scale + the
  CC Resample menu), **Canvas Size** (+ Reset), and a debounced output-size
  estimate capped at 4 MP. One checked row writes a file; several write one file
  each into a chosen folder. A layer target is rendered alone and cropped to its
  bounds (`web_export.rs::layer_flattened`). PNG/JPG go through the Qt edge,
  PNG-8/GIF through the `pictura_codec::web` encoders. It is a deliberate
  **CC-only scope extension** (CS6 has no Export As dialog); SVG shipped in
  #459 and #477 (entry above). Ceilings (`ponytail:`): the bicubic Resample family collapses to Qt's
  smooth filter (only Nearest Neighbor differs), Canvas Size anchors top-left,
  and a layer's effects are not expanded past its rect. Tests: Rust `#[test]`s
  for `layer_flattened`, Qt Test `tst_export_as` (8 cases); the self-test's
  `raster_export` (code 532) shrank by its old dialog assertions.
- **Mask Density, Feather, and Invert** (#260, branch `feat/260-mask-refinement`,
  OpenSpec change `mask-density-feather`): `LayerMask` and `VectorMask` carry
  `density` (PSD byte, default 255) and `feather_bits` (`f64` bits so `Layer`
  keeps `Eq`). The codec parses and writes the mask-parameters block (flag bit 4;
  layout from psd-tools `MaskParameters` and ag-psd, the real-mask header first
  when a `-3` channel exists), routes the vector half to `VectorMask`, and keeps
  records without parameters byte-identical; psd-tools reads our output back
  (`mask_params_oracle`). `pictura-render::mask_refine` applies density
  (`255 - d(255-m)/255`) and feather (the selection feather's Gaussian, 250 px
  cap) with a per-thread cache keyed on the new lazy `Plane::ident`, so painting
  a mask retires its feathered plane. Properties Mask/Vector Mask sections have
  live Density/Feather fields (preview while dragging, one history state on
  commit) and a permanent Invert. Ceilings (`ponytail:`): both formulas are
  inferred; refined high-depth masks sample at 8 bits; a parameter-only
  record's default colour (255) is unsourced.
- **Tool options seam, pen gesture undo, and the Delete key** (#357, #307,
  #304; archived `tool-options-handler-seam`, `pen-gesture-undo`,
  `delete-key-layer`; new specs `tools/pen-edit-session`, `ui/delete-key`):
  `ToolContext` (95 pure virtuals) and `tool_context.h` (399 LOC) are deleted.
  Option state lives in per-family groups in `tool_options.h` (Paint,
  Selection, Crop, CloneSource explicit; the rest `ToolOption<T>`), created by
  `ToolController`, owned by the handlers that share them, and resolved typed
  by `controller_->optionsFor<T>(id)` through `ToolHandler::optionGroups()`;
  `options_bar*.cpp` and the brush/clone/glyph/paths panels write the groups,
  the controller keeps only signal wiring and its own reads, and `tools.cpp`
  shrank 1198→991. A live Pen-group gesture is now one modal session step:
  Ctrl+Z cancels the in-progress anchor/handle/freeform gesture (restoring the
  pre-press path via `paths.rs::path_gesture_*`) before the document history,
  and the release records nothing. Delete/Backspace route through window
  shortcuts (`handleDeleteKey`): a type session or editable field keeps the
  key, a tool hook consumes it, a pixel selection clears (Edit > Clear), else
  the selected layer(s) delete — regardless of focus. Ceiling (`ponytail:`):
  a cancelled gesture is reverted from an in-view path backup, not a replayed
  inverse edit.
- Oracles installed for tests: `psd-tools` 1.19, ImageMagick 7.1.2, `magick`.
- Test suite: **3101 tests, 3101 passed, 23 skipped, 0 failed** from
  `cargo nextest run --workspace` (the local ImageMagick fixture mismatch,
  `imagemagick_fixtures_reproduce`, no longer fails here). The C++ self-test reports
  **510 passed, 0 failed** in the unified report
  (`scripts/verify-fast.sh`, which reruns both plus the workspace probes and the
  Qt Test suites), which reports **4274 passed, 24 skipped, 0 failed**.
- **Large images** (#185, branch `perf/185-large-image-boundaries`, PR #257,
  OpenSpec change `large-image-stroke-boundaries`, numbers in its `design.md`):
  measured on a 16507×16196 PNG. History keeps a private copy of the current
  state (plane stamps say which planes are known equal) and undo/redo/jump apply
  tiles in place and repaint only the restored region; strokes paint the live
  document in place, saving the tiles they touch; lock/colour label do not
  composite and blend/opacity/fill repaint only the layer's bounds; the CPU
  compositor bands per-pixel stacks; GPU composites write staging directly,
  reuse their readback buffer and keep unchanged layer uploads resident; a
  document canvas holds no full-resolution image. First dab 0.5–1.1 s → ~5 ms,
  undo 6 s → ~30 ms, lock 2–3 s → ~5 ms, full-size blend change ~3 s → ~1.25 s
  (GPU on). A software Vulkan adapter (lavapipe) counts as no GPU. Follow-up
  ceilings: history capture still compares written planes whole (~82 ms on the
  map) — photocraft-style tiled storage is the next step.
- **Autosave & crash recovery** (#184, branch `feat/184-autosave-recovery`,
  OpenSpec change `autosave-crash-recovery`, `XC-012`): each interactive launch
  owns a `QLockFile`-held session dir under
  `$XDG_STATE_HOME/kooka-pictura/recovery/`; a timer (File Handling pref,
  default 10 min) writes a PSD snapshot of each dirty, changed document on a
  worker thread; save/close drops it, a clean exit deletes the session. A
  launch that finds a dead owner's session offers Recover / Discard / Later and
  reopens documents as unsaved `<name>-Recovered`. The PSD codec now writes and
  reads the Work Path (1025) and saved paths (2000-2997), so paths survive both
  save and recovery (psd-tools oracle `path_resource_oracle`). Ceilings: no
  journal, crash handler, or Background Save; History is not recovered.
- **Rulers & guides** (#294 + #299, branch `feat/294-rulers-guides`, OpenSpec change
  `rulers-and-guides`): `View > Rulers` (`Ctrl+R`) adds rulers above and
  left of every canvas (`CanvasRuler` in the `CanvasScrollBars` grid); dragging
  out of a ruler drops a guide (default Cyan `#4AFFFF`). Guides are document
  data (`Document.guides`), undoable ("New/Move/Delete Guide", "Clear Guides"),
  moved with the Move tool or Ctrl, deleted by dragging off the canvas, and
  saved as PSD resource 1032 (psd-tools oracle `guide_resource_oracle`).
  `Show > Guides`, `Lock Guides`, `Clear Guides`, `New Guide…`, and the
  Guides, Grid, & Slices preferences page (guide colour/style) are real.
  Rulers default to inches; right-click or Edit > Preferences > Units &
  Rulers (a real page; double-click a ruler) picks the unit and point size.
  Ceilings: no snapping or Alt-flip; Smart
  Guides/Grid/Slices prefs are shown disabled.
- **Workspace scrollbars** (#823, branch `fix/823-workspace-scrollbars-inverted`,
  OpenSpec change `fix-workspace-scrollbar-direction`, archived into
  `document/canvas-scrollbars`): the bars project the canvas's **pan range** —
  the same range that clamps the pan, so the handle reaches every position the
  canvas can be dragged to, the reveal margin included — and the handle travels
  *against* the content: a bar's value is the distance from the range's
  maximum, so the bar's minimum shows the start of the document and its maximum
  its end, and raising the value lowers the canvas offset (`ox = maxX - value`).
  The canvas offset keeps its meaning for the pan, the zoom, the rulers and the
  Navigator. The reveal margin is pannable, so the bar's ends park the canvas
  partly off the workspace — the same limits the hand tool has. Ceilings: the
  margin takes a large share of the handle's travel when the document barely
  overflows the viewport.
- **In flight:** the large-image PR above. Open PRs are docs-only — #191 (README AI
  disclosure, real screenshot, `CLAUDE.md`, developer guide) and #135 (Fedora 43
  build instructions). The Paragraph Styles panel + Edit/Image menu commands
  (#76/#84 → #177), the CS6 adjustments/filters/Smart Filters port (#167 → #170),
  and the adjustment dialogs + Image ▸ Mode + Properties canvas section
  (#181 → #182) have all merged, so the earlier in-flight entries are closed.
- **Lucide icon set** (archived `lucide-icon-set`, spec `ui/icon-assets`): all 165
  `assets/icons/` SVGs are now Lucide-derived (91, vendored verbatim from
  `lucide-static` 1.50.0) or Lucide-style custom (61), with 3 flagged notes and
  10 out-of-scope 3D tool glyphs left as-is. Eleven orphaned glyphs (the reserved
  `window.panels.3d|character|libraries|masks|measurementLog|paragraph|smartObjects|timeline|toolPresets|typeStyles`
  ids and `panel.closeChevron`) were removed from disk and the qrc. Ids for the
  live icons are unchanged. `pictura::icon(id)` renders through the new `SvgIconEngine`
  (`svg_icon_engine.{h,cpp}`), tinting by alpha with the palette foreground (or
  an explicit `icon(id, QColor)`), disabled-aware, and re-rendering after a
  brightness change (theme generation folded into the engine key +
  `QPixmapCache::clear()` in `Theme::apply`). Attribution: `LICENSES/Lucide.txt`
  (ISC + Feather MIT), a `NOTICE.md` section, and an icon-assets block in the
  generated `THIRD-PARTY-LICENSES`; provenance in `docs/dev/icon-provenance.md`
  + `assets/icons/lucide-map.json`, reproduced by `scripts/sync-lucide-icons.py`
  and enforced by `scripts/check-icon-provenance.py`. Cursors are a follow-up.
- **UI font** (`bundle-noto-sans`, spec `ui/bundled-fonts`): the app now pins the
  bundled **Noto Sans** as the `QApplication` font. Eight static faces
  (400/500/600/700, upright + italic) are embedded from `assets/fonts/` via
  `assets/pictura.qrc` and registered by `fonts.{h,cpp}`;
  `main.cpp` calls `applyBundledUiFont()` right after the `QApplication`, before
  any widget, at a fixed 12 px. This fixes the weighted chrome that was silently
  snapping to a system face because the platform default family has no
  intermediate weights (DejaVu Sans: Book/Bold only) and makes the UI family
  OS-independent. The tab labels use Bold (`font-weight: 700`), applied on the
  bar itself for both the document and panel tab bars via
  `fonts.cpp:applyTabBarFont` (so elision metrics and weight match, and the file
  bar no longer relies on the QSS subcontrol rule alone); Medium (500) added
  only ~1.6 % ink over Regular at the 10 px tab size and SemiBold (600) ~3.7 %,
  both too subtle, Bold ~5.5 %.
  License `LICENSES/NotoSans-OFL.txt`; provenance in `assets/PROVENANCE.md`;
  verified by `tst_fonts`.
- OpenSpec **1.13.2** (`/usr/bin/openspec`). M0–M47 archived plus the
  content-named `layers-panel-controls`, `layers-filtering-search`,
  `layers-panel-chrome-fixes`, `layers-panel-row-interactions`,
  `layers-panel-control-polish`, `layers-panel-management`,
  `selection-tools-and-menu`, `psd-interop-compression`,
  `psd-opaque-preservation`, `psd-smart-object-roundtrip`,
  `smart-object-source-render`, `adjustment-payload-decode`, and
  `psd-rle-write`, `convert-to-smart-object`, `rasterize-smart-object`,
  `place-smart-object`, `replace-smart-object-contents`,
  `open-as-smart-object`, `export-smart-object-contents`,
  `photo-filter-adjustment-decode`, `gradient-map-adjustment-decode`,
  `solid-color-fill-descriptor`, `gradient-fill-layer`,
  `edit-smart-object-contents`, `image-import`, `file-drop-routing`,
    `free-transform-mode`, `psb-write`, `color-balance-adjustment-decode`,
    `pattern-fill-layer`, `layer-effects-drop-shadow`,
    `layer-effects-outer-glow`, `layer-effects-inner-shadow`,
    `layer-effects-inner-glow`, `layer-effects-stroke`,
    `layer-effects-overlays`, `layer-effects-satin`,
    `layer-effects-bevel`, `layer-effects-legacy-lrfx`, and
    `channel-mixer-adjustment-decode`, `curves-adjustment-decode`,
    `selective-color-adjustment-decode`, `layer-effects-stroke-fills`,
    `vector-mask-render`, `vector-fill-content`, `color-mode-read`,
    `depth-read`, `color-lookup-adjustment-decode`, `psd-image-resources`,
    `psd-icc-convert`, `psd-file-info`, `psd-iptc-write`, `assign-convert-profile`,
    `xmp-metadata`, `metadata-templates`, `psd-zip-write`, `color-settings`,
    `app-control-server`, `depth-preserve`, `agentic-control-vision`,
    `agentic-control-actions`, `color-mode-write-back`, `cmyk-write-back`,
    `icc-output-mode-consistency`, `indexed-write-back`, `bitmap-write-back`,
    `agentic-control-input`, `depth-color-mode-write-back`,
    `agentic-control-e2e`, `phfl-v3-xyz-decode`, `type-layer-kind`,
    `tysh-model-roundtrip`, `multichannel-duotone-read`, and
    `knko-blend-if-model`, and
    `crs-xmp-edit`, and
    `blend-if-render`, and
    `type-engine-data`, and
    `text-render-seam`, `text-rasterize-bundled`, and
    `type-rasterize-command`, and
    `type-live-composite`, and
    `rasterize-all-type`, and
    `rasterize-type-command`, and
    `text-qt-backend`, and
    `knockout-composite`, and
    `knockout-oracle`, and
    `knockout-groups`, and
    `knockout-isolated-groups`, and
    `text-shaping-rustybuzz`, and
    `text-shaping-offsets`, and
    `text-subpixel-positioning`, and
    `text-backend-drop-fontdue`, and
    `psd-tagged-block-8b64`, and
    `bit-depth-sample-model`, and
    `native-depth-adjustments`, and
    `native-depth-color-adjustments`, and
    `native-depth-composite`, and
    `native-depth-layer-content`, and
    `native-depth-remaining-adjustments`, and
    `native-depth-masks`, and
    `native-depth-save`, and
    `hdr-exposure-gamma`, and
    `magnetic-lasso`
    `edit-clipboard`, and
    `edit-clipboard-interop`, and
    `perspective-crop-and-slice`, and
    `slice-select-tool`, and
    `open-recent-menu`, and
    `crop-tool-box`, and
    `annotation-tools`, and
    `cs6-shared-widgets`, `panel-line-art-glyphs`, `info-panel-expansion`,
    `swatches-panel-expansion`, `properties-panel`, `channels-panel`,
    `info-panel-readout-grid`, `info-panel-menu-refinements`,
    `info-panel-grid-style`, and `channels-thumbnail-source`
    changes;
    canonical specs are in `openspec/specs/` as a `{domain}/{capability}` tree
   (128 specs, `validate --all --strict` green), change history under
   `openspec/changes/archive/`.
   The panel-program stage **layer styles / effects** is complete:
   `layer-effects-drop-shadow`, `layer-effects-outer-glow`,
   `layer-effects-inner-shadow`, `layer-effects-inner-glow`,
   `layer-effects-stroke`, `layer-effects-overlays`, `layer-effects-satin`,
   `layer-effects-bevel`, and `layer-effects-legacy-lrfx` cover the object-based
   effect kinds (drop shadow, outer glow, inner shadow, inner glow, stroke,
   color/gradient/pattern overlay, satin, and bevel & emboss) plus the legacy
   `lrFX` block.
- **Shared panel widgets and the CS6 panel slices** (issues #70, #71, #78, #79,
  #80, #81, ported from photorust): six capabilities landed on the panel
  foundation — `ui/shared-widgets` (the `AngleDial`, `RampSlider`, `SpectrumBar`,
  and `CurveWidget` controls, with `tst_shared_widgets`), the `ui/icon-assets`
  additions (eleven `layers.*`/`path.*` glyphs rebased onto the 24×24 grid, with
  `tst_icon_assets`), `ui/properties-panel` (read-only: the active adjustment
  layer is identified and named), `ui/channels-panel` (presentational composite
  plus RGB rows with thumbnails and local-only eye toggles), the
  `ui/info-histogram-panel` expansion (the CMYK readout and the Ruler A/L + W/H
  mode), and the `ui/color-swatches-panel` expansion (reflowing named
  `SwatchGrid`, click modifiers, footer New/Delete, and a context menu with
  Reset). The six changes are archived.
  Deferred engine work: adjustment-parameter get/set plus an edit session for the
  Properties panel (#70); a per-channel visibility model, a compositor
  channel-mask path, and channel enumeration for the Channels panel (#71);
  `.aco` swatch-library load/save (needs a `pictura-presets` crate, which does
  not exist yet).
- **Missing CS6 filter kernels** (change `port-missing-filter-kernels`, issue
  #82, ported from photorust): `pictura-filters` gained six CS6-Core kernels —
  Noise ▸ Dust & Scratches (`noise::dust_and_scratches`), Stylize ▸ Extrude /
  Tiles / Trace Contour / Wind (`stylize/{extrude,tiles,trace_contour,wind}.rs`),
  and Sharpen ▸ Smart Sharpen (`sharpen::smart_sharpen`), now including CS6's
  **More Accurate** higher-fidelity blur path and the Advanced
  **Shadow/Highlight** `TonalFade` fields (`amount`/`width` 0–100, `radius`
  1–100; `amount` 0 is a no-op), both `(inferred)` approximations. They extend the
  `Filter` enum (`filter/types.rs`) and `apply` dispatch, validate before
  mutating, leave alpha bit-identical, and are deterministic (Extrude/Tiles/Wind
  read a coordinate hash, not an RNG). Verification is property tests, since
  Adobe's kernels are closed: the new kernels are registered as no-equivalent in
  the oracle mapping table (`tests/oracle/mapping.rs`, tolerance 0) and covered
  by property tests, while Smart Sharpen's Gaussian path is byte-identical to
  Unsharp Mask and so rides the existing ImageMagick differential (tolerance 6).
  `filter_from_kind` maps six new kinds (`dust-and-scratches`, `extrude`,
  `tiles`, `trace-contour`, `wind`, `smart-sharpen`), reachable through the
  control server and the Rust engine tests; the headless self-test only ever
  exercises `add-noise`, so these have no self-test coverage. New crate-root
  enums `ExtrudeType`, `TileFill`, `ContourEdge`, `WindMethod`, `SharpenRemove`,
  and `TonalFade`.
  The unsupported `docs/06-filters/stylize-filters.md` claim that Extrude Depth
  does not apply with Solid Front Faces was corrected (CS6 Help describes the
  two controls independently and gives no exception).
  Ceilings (`ponytail:`):
  Diffuse/Glowing Edges (Filter-Gallery-only), Render ▸ Lighting Effects (GPU
  workspace), and Flame/Picture Frame (CC 2014.2, non-goals) stay unported, and
  the Filter menu entries and per-filter dialogs remain disabled stubs — their
  wiring is a separate follow-up.
- **Fibers** (change `port-photorust-fibers`, issue #224, ported from
  photorust): `render::fibers` (now `render/fibers.rs`) lays soft clumps and
  hard-edged hairs down the picture instead of a horizontal fBm field;
  Variance `0..=64` (0 = even blend), Strength `1..=64`. Clouds and Difference
  Clouds keep Kooka's seeded, two-colour, Starker model by decision.
- **Lens Flare** (change `port-photorust-lens-flare`, issue #225, ported from
  photorust): `render::lens_flare` (now `render/lens_flare.rs`) is photorust's
  size-invariant model: an inverse-square core and glow, a halo, rays or the
  Movie Prime streak, and tinted hexagonal ghosts along the axis through the
  frame centre. Filter ▸ Render ▸ Lens Flare… opens a dedicated
  `LensFlareDialog` (`lens_flare_dialog.{h,cpp}`, tested by
  `tst_lens_flare_dialog`): a 250 px proxy with the flare under a draggable
  crosshair, Brightness field + slider, and a Lens Type radio group. Lens Flare
  previews against the whole layer (#168).
- **Shear** (change `port-photorust-shear`, issue #226, ported from photorust):
  `Filter::Shear` now shifts each **row** horizontally, not each column
  vertically. `curve` is `(position, offset)` points (top row `-1`, bottom
  `1`; shift `offset × width/2`), the warp moved to `photorust::distort::shear`
  on the shared `remap` (premultiplied translucent edges), and the old
  `distort/coord.rs` vertical shift is gone. `SHEAR_MAX_POINTS = 8` is
  exported; the `shear` kind grew from 7 to 18 slots (a point count, up to 8
  `(position, offset)` pairs, and the fill) and defaults to a straight curve +
  Wrap Around. The dialog is CS6's: a 140 px curve box (click the line to add,
  drag a point out to remove, the two ends slide horizontally only, only the
  points the user placed are shown), a Hermite line that is straight between
  two points and rounded between more, `Undefined Areas` radios (`Wrap Around`
  checked), OK/Cancel to the right, and a live whole-picture preview below,
  with no Preview checkbox and no zoom controls. New `ShearCurve`/`Radio`
  controls and a `previewBelow` layout in
  `filter_param_controls`/`filter_preview_dialog`; the pane re-filters a
  320 px proxy on every change while the whole-layer canvas render waits for
  the mouse release. The Filter Gallery's options pane gets the same curve
  editor. `shear` joins `filter_preview_needs_whole_layer` (its shift is a
  fraction of the layer's half-width). Tested by the ported property tests and
  `tst_shear_dialog`.
  Oracle: still no-equivalent, re-measured against
  `-shear -26.565x0 -crop 16x16+4+0 +repage` max 255 / mean 18.4
  (WrapAround 23.4). Verified: `cargo fmt --all --check` and `cargo clippy
  --workspace --all-targets -- -D warnings` clean; `cargo nextest run
  --workspace` **2706 passed, 17 skipped**; both headless self-tests exit 0
  (475/0/0 and 510/0/0); `ctest -R '^tst_'` 41/41 (new `tst_shear_dialog`);
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` → `verify-fast: OK`
  (TOTAL 3526 passed · 18 skipped · 0 failed); `openspec validate --all
  --strict` 207/207. The change REMOVEs the old `Shear` requirement and ADDs
  `Shear row shift` (`imaging/distort-filters`) plus the `Shear dialog`
  requirement (`imaging/filter-app-ui`); no new capability.
- **Displace** (change `port-photorust-displace`, issue #335, ported from
  photorust): `Filter::Displace { map, horizontal_scale, vertical_scale, fit,
  wrap }` over a `DisplaceMap` (two 8-bit planes). Each pixel reads the
  source `(v − 128) × scale/100` px away (the contract's 128 px at 100%, not
  photorust's half image); Stretch To Fit samples the map bilinearly, Tile
  repeats it; Wrap Around / Repeat Edge Pixels use the shared `remap`. Joins
  the alpha-moving allowlist. The app keeps loaded maps in
  `filter_tools/displace_maps.rs` (the 8 most recent, process-lifetime);
  `filter_displace_map_load` reads a flattened PSD (profile ignored, Bitmap
  refused). The `displace` kind is 5 slots, the last a map id (0 = neutral).
  The dialog is CS6's (two scales, two radio groups, no preview: new
  `FilterControl::Hidden` and `FilterCommandSpec::livePreview`); after OK a
  file dialog asks for the map. Last Filter reuses the map; Last Filter
  Settings asks again. Ceilings: the map fits the layer, not the selection
  bounds; CMYK/Lab/Indexed maps read as RGB. Oracle: the first Distort filter
  with a faithful ImageMagick operator (`-compose displace`, `X = 1.275 ×
  scale`); Repeat Edge Pixels within 8 (measured max 5 / mean 0.5); Wrap
  Around differs at the seam and is not diffed. Tests: kernel units,
  `displace_maps` units, `tst_displace`. Verified: `TASK_ALLOWS_DOCS=1 bash
  scripts/verify-full.sh` → `verify-fast: OK` (TOTAL 3913 passed · 21 skipped ·
  0 failed; nextest 2922 passed / 20 skipped); `openspec validate --all
  --strict` 231/231.
- **Custom** (change `expose-custom-filter`, issue #336): `Filter ▸ Other ▸
  Custom…` exposes the existing `Filter::Custom` kernel. The `custom` kind is
  27 slots (the 5×5 weights row by row, Scale, Offset; empty = pass-through).
  The dialog adds `FilterControl::Matrix` (a 5×5 grid of `-999..=999` cells)
  and `FilterControl::Number` (a value box with no slider) for Scale
  (`1..=9999`) and Offset (`-9999..=9999`), with the usual live preview.
  Ceiling: no Save/Load (`.acf`) buttons. Tests: `filter_map` unit,
  `tst_custom_filter`. Verified: `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh`
  → `verify-full: OK` (TOTAL 3917 passed · 21 skipped · 0 failed); `openspec
  validate --all --strict` 232/232.
- **Shape Blur** (change `add-shape-blur`, issue #337): new engine (no
  photorust source). `Filter::ShapeBlur { shape, radius }` convolves each
  colour plane with a Custom Shape's antialiased coverage
  (`pictura_core::shape::custom_shape_kernel`), `2r+1` px square, rotated 180°
  so a bright pixel spreads into the shape as drawn; clamp-to-edge, integer
  exact. Kernel rows split into equal-weight runs over per-row prefix sums, so
  a pixel costs the run count, not the area (~100 ms at radius 46 on 1024²).
  `Filter ▸ Blur ▸ Shape Blur…` (after Box Blur) has Radius `5..=1000`
  (default 10, inferred) and `FilterControl::Shape`, a grid of the six
  built-in Custom Shapes; the `shape-blur` kind is 2 slots (Radius, Shape).
  `filter_map.rs`'s choice lists moved to `filter_map_choices.rs` to stay
  under the cap. Ceilings: the six app shapes, not CS6's library; no large
  Shape swatch or shape-set gear. Tests: unit (runs = direct convolution on
  every shape), ImageMagick `-morphology Convolve` differential (exact),
  kernel digests, `filter_map` unit, `tst_shape_blur`.
- **Smart Blur** (change `add-smart-blur`, issue #339): new engine (no
  photorust source). `Filter::SmartBlur { radius, threshold, quality, mode }`
  in `blur/smart.rs` is a selective Gaussian (σ = radius/3) over the disc of
  Radius, averaging only neighbours whose every channel is within Threshold
  levels; Quality (engine `Quality` Draft/Good/Best = Low/Medium/High) caps the
  disc at 3/6/12 taps a side. An edge is a pixel with a 4-neighbour past
  Threshold: Edge Only paints edges white on black, Overlay Edge white over the
  blur. `Filter ▸ Blur ▸ Smart Blur…` has Radius/Threshold `0.1..=100` (defaults
  3/25, inferred), Quality (Low) and Mode (Normal); the `smart-blur` kind is 4
  slots. ~160 ms at radius 50 High on 1024². Ceilings: Adobe's kernel is
  closed; the joint test, weights, tap budget and edge rule are inferred. Tests:
  unit (step/threshold/modes/quality), kernel digests, `filter_map` unit,
  `tst_smart_blur`.
- **Lens Blur** (change `lens-blur-filter`, issue #338): `pictura-filters` gains
  `DepthMap`/`IrisShape` and `Filter::LensBlur` with the kernel in
  `blur/lens.rs` — a per-pixel `|d − focal|/255` circle of confusion, a
  precomputed antialiased iris tap table (blades 3–8, curvature, rotation), an
  additive specular highlight, and seeded uniform/Gaussian grain; `None` is a
  uniform far depth, a mismatched map stretches bilinearly, and Lens Blur moves
  transparency (alpha allowlist) while the alpha pass drops the colour-only
  grain and specular. `Filter ▸ Blur ▸ Lens Blur…` opens CS6's dialog on the
  shared pane (15 slots; defaults from the #338 screenshot: Source None, focal
  126, Triangle (3), radius 50, curvature 33, rotation 191, brightness 44,
  threshold 93, noise 47 Gaussian, Faster), previews the whole layer, and
  always commits More Accurate. The app materializes Transparency / Layer Mask
  / Selection into a depth map cropped to the active layer
  (`filter_tools/lens_blur_depths.rs`, `filter_lens_blur_depth`); an
  unavailable source refuses the commit. No ImageMagick equivalent (mapping
  row tolerance 0); guarded by the kernel property tests, a digest entry, the
  render alpha/apron tests, and `tst_lens_blur_dialog`. Ceilings (`ponytail:`):
  the CoC ramp, iris curvature mapping, specular compositing, and grain shape
  are inferred approximations; the source combo has no named alpha channels
  and no click-to-focus preview; the gather is `O(Σ CoC area)` per pixel on
  the CPU. Verified: `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh` builds
  and passes fmt, clippy, file-size, selftest-budget, and guard;
  `openspec validate --all --strict` 232/232; the unified report is 3944
  passed · 21 skipped · 1 failed, the one failure the pre-existing local
  ImageMagick fixture mismatch noted above.
- **Liquify** (change `add-liquify`, issue #348): new engine, ported from
  photocraft's displacement field (MIT OR Apache-2.0). `distort::liquify`'s
  `LiquifyMesh` holds an inverse offset per node of a 2/4/8 px grid plus a
  freeze mask; brush dabs (`d' = q + d(q) − p`) implement Forward Warp,
  Reconstruct, Twirl, Pucker, Bloat, Push Left, Freeze/Thaw, and Kooka's own
  Mirror and Turbulence, with Alt reversals; Reconstruct amount, Restore All,
  Mask All/None/Invert All, `scaled_to`. `Filter::Liquify { mesh }` resamples
  Catmull-Rom (identity bit-exact; ~110 ms for 24 MP in release) and warps
  alpha (opacity allowlist). The app keeps one dialog session and up to eight
  kept meshes (`filter_tools/liquify.rs`); the `liquify` kind is one slot, the
  mesh id, so OK commits one filter state, Last Filter replays it and Last
  Filter Settings reloads it. `LiquifyDialog`/`LiquifyCanvas` give CS6's
  workspace: Basic (Warp, Reconstruct, Pucker, Bloat, Push Left, Hand, Zoom;
  Size/Pressure 100 from the #348 capture) and Advanced (Twirl, Mirror,
  Turbulence, Freeze, Thaw, Density/Rate/Jitter, Reconstruct…, mask options,
  View Options: Show Image, Show Mesh — Small/Medium/Large, colour, traced as
  contours of `p + d(p)` — Show Mask, Show Backdrop — All Layers or one
  layer, In Front/Behind/Blend (Difference stand-in), Opacity 50); the view is rendered at screen resolution from the
  full-resolution layer (Catmull-Rom when magnified, box levels minified) and
  re-rendered per dab rectangle; opens at 100 %, Zoom tool right-click menu;
  Load Last Mesh; W R C S B O X T F D H Z, `[`/`]`. Ceilings (`ponytail:`):
  tool strengths tuned by eye; no `.msh` files, mask sources, Show Guides,
  Pin Edges, live Reconstruct preview or GPU path; kept meshes live for the
  process; `Edit ▸ Fade` is still an app-wide stub. Tests: 19 engine units
  (+1 ignored profile), store and `filter_map` units, `tst_liquify_dialog`
  (20). Verified: `bash scripts/verify-full.sh` — unified report 4011 passed ·
  22 skipped · 0 failed, file-size and selftest-budget OK, guard OK with
  `TASK_ALLOWS_DOCS=1`; `openspec validate --all --strict` 237/237.
- **Field Blur** (change `field-blur-gallery`, issue #340): the Blur Gallery's
  first effect and its shared workspace shell. `pictura-filters::blur::field`
  (`BlurPin { x, y, blur }`, `BokehParams`, `BlurNoise`, `field_blur`,
  `MAX_FIELD_RADIUS` 500, `MAX_FIELD_PINS` 16) builds a per-pixel radius field
  from inverse-square-distance pin weights (pins may sit outside the image)
  and blends the two bracketing levels of a box-blur pyramid, so a live pin
  drag stays fast; it is deterministic, validates before mutating, joins the
  moving-kernel alpha allowlist (the alpha pass drops the colour-only
  effects), and returns the largest pin amount as its preview apron. After
  the blur, the Blur Effects pass multiplies the in-range blurred pixels by
  up to 3× (Light Bokeh) so bright highlights bloom brilliantly toward white,
  and pushes the lightened, non-clipped pixels away from their
  luminance (Bokeh Color), both gated by the Light Range window and the local
  field weight; the Noise pass adds seeded Gaussian/Uniform/Grain grain
  scaled by the weight and damped in highlights. The `field-blur` kind is a variable slot
  list (`count, x, y, blur, …, bokeh …, noise …`; the arity table's canonical
  is one centred pin plus the 13-slot effects tail), and
  `Filter ▸ Blur Gallery ▸ Field Blur…` (the later Photoshop submenu, with
  disabled `Iris Blur…` / `Tilt-Shift…` / `Path Blur…` / `Spin Blur…` stubs)
  opens `BlurGalleryDialog` + `BlurGalleryCanvas`: a document proxy with
  draggable circle pins (click adds, drag moves, the selected pin's ring
  changes its amount, each ring's white arc shows the blur amount — full at
  100 px, Delete removes, `H` hides the overlay), a Blur Tools
  panel (the selected pin's Blur 0–500 px, plus the four disabled rows), an
  Effects / Motion Effects / Noise tabbed panel (Bokeh and grain controls;
  Motion Effects is a placeholder), Preview / Cancel / OK, and one `"Filter"`
  state on commit; Last Filter / Last Filter Settings reuse the pin list.
  #341/#342 reuse the shell. Tests: engine units + a kernel digest + a
  no-equivalent oracle row, `filter_map_tests`, `tst_filter_menu`'s row, and
  `tst_blur_gallery_dialog` (pins, Effects, Noise, commit/cancel, Last
  Filter). Ceilings (`ponytail:`): the doc's disc/bokeh gather is approximated
  by the box pyramid (highlights blur softly, not disc-shaped), and the bokeh
  curves and grain model are inferred; the `M` mask view, the Motion Effects
  controls, and Iris/Tilt-Shift/Path/Spin are not shipped; the preview is a
  document proxy on the dialog's own canvas, and pins normalize to the
  document while the commit normalizes to the active layer, so a layer that
  does not cover the document previews approximately; no GPU. Verified:
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh` builds and passes fmt,
  clippy, file-size, selftest-budget, icon provenance, guard, and control;
  `openspec validate --all --strict` 237/237; the unified report is 3994
  passed · 21 skipped · 1 failed, the one failure the pre-existing local
  ImageMagick fixture mismatch noted above (nextest 2969/1/20, app self-test
  510, Qt 515).
- **Iris Blur** (change `iris-blur-gallery`, issue #341): the Blur Gallery's
  second effect, on Field Blur's shell. `pictura-filters::blur::iris`
  (`IrisPin { x, y, radius_x, radius_y, angle, squareness, feather, blur }`,
  `iris_blur`, `iris_exponent`, `MAX_IRIS_RADIUS` 10) makes each pin a
  rotated superellipse (semi-axes in buffer widths; squareness 0..100 maps to
  the exponent 2 → 20): sharp inside the feather core, a smoothstep across the
  fade band, the full amount outside the outline. Pins multiply their
  falloffs (one sharp island each) and blend their amounts by
  inverse-square distance. Field Blur's pyramid and effects passes moved
  behind a shared `variable_blur(radius_at)` with the Field digest unchanged.
  The `iris-blur` kind is `count`, 8 slots per pin, then the 13-slot effects
  tail (22 canonical), parsed with `field-blur` in `filter_map/gallery.rs`.
  `Filter ▸ Blur Gallery ▸ Iris Blur…` opens `BlurGalleryDialog` in Iris mode:
  the Iris Blur row active (Blur, default 15 px), the other rows disabled, and
  the selected pin's outline with radius handles (resize + rotate), feather
  dots, and the roundness diamond; new pins copy the selected outline. Tests:
  iris engine units, a kernel digest, a no-equivalent oracle row,
  `filter_map_tests`, the whole-layer preview row, `tst_filter_menu`, and five
  `tst_blur_gallery_dialog` cases. Ceilings: the falloff, squareness map, and
  multi-pin combine are inferred; one gallery effect per workspace; feather
  dots move together (no Alt-drag); no `M` mask view. Verified:
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` passes; `openspec validate
  --all --strict` 239/239; the unified report is 4052 passed · 22 skipped ·
  0 failed (nextest 3004/0/21, Qt 538).
- **Tilt-Shift** (change `tilt-shift-gallery`, issue #342): the Blur
  Gallery's third CS6 effect. `pictura-filters::blur::tilt` (`TiltPin { x, y,
  angle, focus_above, focus_below, fade_above, fade_below, distortion,
  symmetric, blur }`, `tilt_shift`, `MAX_TILT_EXTENT` 10): a band through each
  pin, extents in buffer heights per side (centre → solid → dashed line),
  sharp / smoothstep fade / full blur; pins combine as Iris. Distortion
  (−100..100) smears the zone below the band (both when symmetric) radially or
  tangentially between the pyramid and the effects passes — `variable_blur`
  split into `pyramid_blur` + `apply_effects` with the Field/Iris digests
  unchanged. The `tilt-shift` kind is `count`, 10 slots per pin, the 13-slot
  tail (24 canonical). `BlurGalleryCanvas` takes a `BlurGalleryMode`; the
  Tilt-Shift mode draws solid/dashed lines and rotation dots, and the row
  carries Blur, Distortion, Symmetric Distortion. Every Blur / Blur Gallery
  command now refuses an empty target (`filter_target_empty`) with a
  "Could not complete the … command because …" error before any dialog, on
  the menu, Last Filter, and Last Filter Settings paths. Tests: tilt engine
  units, a digest, a no-equivalent oracle row, mapping and empty-target Rust
  tests, `tst_filter_menu`, and the new `tst_tilt_shift` suite. Ceilings: the
  falloff, combine, and smear are inferred; one effect per workspace; no `M`
  mask view. The gallery canvas shows resize/rotate cursors over the
  Tilt-Shift lines and dots, and every gallery mode previews a ≤ 360 px draft
  while a pin, handle, line, or amount slider is dragged (~26 ms vs ~213 ms
  for the 1100 px proxy at Blur 176), the full proxy on release. The gallery
  preview now filters only the active layer over the document with it hidden
  (`filter_proxy_base` + `filter_proxy_layer`), and the gallery pyramid and
  smear blur the alpha plane (they had stopped at three planes since the
  fourth-plane alpha refactor, so layer edges never feathered); the Field,
  Iris, and Tilt-Shift kernel digests changed for that (alpha bytes only).
  The Blur Tools rows are now selectable (name), checkable, and expandable
  (arrow); each effect keeps its own pins and the ones switched on combine
  in `blur::gallery` (`Filter::BlurGallery`, per-pixel max of the effect
  fields, one effect byte-identical to its own kernel). OK commits the one
  effect's own kind, or `blur-gallery` ("Filter: Blur Gallery", reopened by
  Last Filter Settings) for several. `LightRangeSlider` moved to
  `blur_gallery_light_range.h`. Verified: `TASK_ALLOWS_DOCS=1 bash
  scripts/verify-fast.sh` passes; `openspec validate --all --strict`
  248/248; the unified report is 4126 passed · 22 skipped · 0 failed
  (nextest 3026/0/21, app self-test 510, Qt 590).
- **Picture Frame** (change `port-photorust-picture-frame`, issue #227, ported
  from photorust): `render::picture_frame` (`render/picture_frame.rs`) draws a
  decorative border over the layer: 47 frames over a shared catalogue of 22
  flower shapes and 23 leaf shapes plus ruled and moulded borders, with Basic
  (margin, size, arrangement, flower/leaf colours and sizes) and Advanced
  (Number of Lines, Thickness, Angle, Fade, Invert) controls, all procedural
  geometry (no traced Adobe asset). `Filter::PictureFrame { options:
  FrameOptions }`, the `picture-frame` kind (22 slots, the frame slot a 0-based
  picker index), joins the moving-kernel alpha allowlist, and previews the
  whole layer. Filter ▸ Render ▸ Picture Frame… opens a dedicated
  `PictureFrameDialog` (`picture_frame_dialog.{h,cpp}`, tested by
  `tst_picture_frame_dialog`): a 47-frame thumbnail picker, a picture proxy
  preview, and Basic / Advanced pages. Picture Frame is a CC 2014.2 entry
  shipped by decision, so the `imaging/render-filters` requirement was amended
  to surface it while `Flame`/`Tree` stay unsurfaced.
- **Layers panel manual-test fixes** (#378, change
  `layers-panel-manual-test-fixes`, spec `ui/layers-panel` +
  `compositing/layer-masks`): six independent `PAN-001` fixes. The Opacity
  field's label is right-aligned against its value box
  (`NumericField::setLabelAlignment`); adding a layer mask makes it the view's
  mask edit target (`layer_mask_add_path` sets `mask_edit_target`), and the
  panel derives its active-thumbnail brackets from that target so they follow
  the mask (clicking a vector-mask thumbnail clears the raster target); a group's
  expanded state survives a structural change that shifts its index path (adding,
  removing, or moving a layer) — `LayersModel` gains `namePathForPath` /
  `indexForNamePath` (same-named siblings disambiguated by occurrence) and
  `refresh` remaps expansion by the ancestor name chain, while `setData` patches
  a renamed node's name in place so a rename keeps the group expanded;
  a group's folder glyph draws no black outline and grows from half to three
  quarters of the thumbnail; the thumbnail box spans the row's full height minus
  a 2 px pad (with `letterboxedThumb` corrected for a non-square box) so a tall
  document fills the row; and the mask/vector chain glyph is drawn 2 px larger
  and rotated 90°. The panel's blend and filter combos now draw a drop-down
  caret (`assets/ui/combo-caret{,-disabled}.svg`, since Qt's stylesheet engine
  cannot draw a CSS triangle) and cycle with Up/Down while focused, and a
  focused header control keeps its focus across a refresh (`syncControls` now
  sets the final enabled state once rather than disabling then re-enabling); a
  blend-mode edit records `Blending Change: <mode name>` (`BlendMode::name` in
  `pictura-core`) instead of the generic `Blend Mode`. `LayersPanel::eventFilter`
  moved to `panels/layers_panel_events.cpp` (pure move) for size headroom.
  Checked by the new `tst_layers_panel_fixes` (11) plus updated
  `tst_layers_panel::maskEditTargetActivation` and
  `tst_mask_edit::colorsFollowTheEditTarget` and
  `pictura-core::blend_names_are_distinct_and_non_empty`. The local ImageMagick
  fixture mismatch remains the one environment failure; `openspec validate --all
  --strict` 246/246.
- **Design system** (change `ui-design-system`, issue #249): the theme's token
  table is now a public contract. `Theme::tokens(level)` returns every colour
  (including the derived shades) and the shared metrics (panel/chrome border,
  group divider, slot footprint and icon maxima, shade step) as
  `ThemeToken{key, value, color}`; `styleSheet` builds from it and the new
  `./build/pictura --headless --dump-theme` prints the same list for all four
  levels as JSON. The global QSS gained one canonical definition per primitive
  (text button, icon/toggle button, text input, numeric field, combo, check
  box, slider, panel/document tabs, menu, colour swatch, panel header) with
  rest/hover/pressed/checked/disabled/focus states; a new `accent` token
  (`#2a7fff`) replaces the drop-indicator literals, and the static per-widget
  stylesheets (preview panes, indicators, bold labels, menu padding) moved to
  object-name-scoped rules. Data-driven fills route through the new
  `panels/color_swatch_widget.{h,cpp}` (a `QPushButton` that paints its colour,
  `setColor`/`setActive`, keeps the `color` property), replacing the ten
  ad-hoc swatch stylesheets; no `setStyleSheet` call remains outside
  `theme.cpp`. `scripts/gen-styleguide.py` runs the dump and writes the
  committed `design-system.html` (token tables, metrics, primitive/state
  gallery, level switcher; idempotent). Checked by `tst_design_system`
  (palette == tokens, primitive selectors, object-name/state contract, swatch
  paint); `tst_filter_menu`, `tst_command_tree`, and `tst_retouch_tools` now
  assert the shared contract instead of per-widget stylesheets. Verified:
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh` → `verify-full: OK`; unified
  report 4065 passed · 22 skipped · 0 failed (Qt 550); `ctest -R '^tst_'`
  79/79; `openspec validate --all --strict` 244/244. Ceilings: the HTML guide
  is a static reference, not a live renderer; `QCheckBox::indicator` is the
  tokenised fill, not the Fusion tick.
- **Design system, remaining controls** (change
  `design-system-remaining-controls`, issue #417, first of four design-system
  workstreams; the predecessor `ui-design-system` was archived to promote the
  `ui/design-system` capability into `openspec/specs/`): the global QSS gained
  one tokenised definition for the controls #249 left on raw Fusion — `QGroupBox`,
  `QRadioButton`, `QProgressBar`, `QHeaderView`, and a general `QSplitter`
  handle — states per the #249 contract. The last custom-painted widgets that
  hard-coded greys now read tokens through `Theme::tokenColor(key)`:
  `CurveWidget`, `SpectrumBar`, the histogram view, and the navigator thumbnail,
  so a theme change repaints them. Two paint-only tokens (`graphFill`,
  `graphAxis`) joined `Theme::tokens`; because the stylesheet never references
  them, `ds_tokens_single_source` was relaxed to cover stylesheet-referenced
  tokens (and `tst_design_system` skips the paint-only set). `tst_design_system`
  gained the five selectors, instance/state checks, and a grab-and-sample paint
  assertion. A new `scripts/check-colour-literals.py` (wired into `guard.sh`)
  rejects theme-like opaque grey literals outside `theme.cpp`, over a baseline
  `scripts/colour-literal-allowlist.txt` (45 data, 17 pending conversion) that
  only shrinks. Verified: `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh` →
  `verify-full: OK` (TOTAL 4139 passed · 22 skipped · 0 failed; app self-test
  510); `openspec validate --all --strict` 253/253. Ceiling (`ponytail:`): the
  guard flags opaque equal-channel greys and the grey `Qt::` values, not
  chromatic or translucent literals.
- **Design system, developer browser** (change
  `design-system-developer-dialog`, issue #417, second design-system
  workstream): the design system is browsable from inside the app at
  `Help ▸ Developer ▸ Design System…` — a modeless tool window whose "stories"
  are built from the real widgets (foundations, shell chrome, the standard
  controls, composed patterns), styled by the global stylesheet. It is therefore
  1:1 with the running UI (QSS + Fusion + the custom-painted swatches and graph
  surfaces) instead of a hand-mirrored HTML approximation that drifts. The story
  list is filterable; each story shows its description and QSS selector plus a
  token readout; a brightness control previews a level live through
  `Theme::apply` and restores the user's level on close (new
  `Theme::currentLevel()`). New `cpp/design_system_dialog{,_pages}.cpp`; the
  command is declared in `command_tree.cpp` and wired in `frame_menus.cpp`;
  `tst_design_system_dialog` builds every page and asserts the curated
  primitives are present. This replaced the standalone `design-system.html`
  style guide as the review surface; the committed HTML and `gen-styleguide.py`
  are left as-is (the unbuilt snapshot/coverage-gate rework was dropped).
  Verified: `tst_design_system_dialog` 8/8; app self-test 474 passed · 0 failed ·
  0 skipped; `guard.sh` green; `openspec validate --all --strict` 253/253.
- **Filter Gallery preview UX** (change `filter-gallery-preview-ux`, issue
  #417): the gallery's zoom level is an editable drop-down showing the live
  percentage and listing CS6's presets (6 %–1600 %, Actual Pixels / Fit in View
  / Fit on Screen); the mouse wheel zooms one level over the preview; the
  thumbnails toggle uses the Lucide `panel.rightClose`/`panel.rightOpen` icons on
  the shared button surface (the old «/» glyph is gone). A parameter slider in
  the shared filter controls now focuses its value box, so Up/Down adjust the
  same parameter. A larger pan-performance improvement is filed as issue #452.
  Verified: 88/88 Qt tests, app self-test 474 passed / 0 failed, guard +
  openspec green.
- **Input-control consistency** (change `input-control-consistency`, issue
  #417): the numeric field's slider-popup arrow is now a child of the value
  editor, flush to its inner trailing edge behind a token hairline, and darkens
  while the popup is open; it shows the Lucide `chevron.down` and uses the
  default arrow cursor. `SpinArrowStyle` draws the down/up arrow for spin boxes
  and menu buttons as the tinted `chevron.{down,up}` icon. The select's
  `::drop-down` cell is flat and reserves the value's trailing space (fixing the
  chevron overlapping the value, worst on the workspace switcher, which is
  styled to match); its `::down-arrow`/`::menu-indicator` uses a per-family baked
  chevron (`assets/ui/select-chevron{,-light}.svg`) because QSS cannot tint an
  SVG — equivalent to a tint since the foreground is constant across levels.
  The tokenised checkbox regained its tick (`assets/ui/checkbox-check.svg`).
  Text inputs, editable inputs, selects, the numeric field, and the options-bar
  workspace switcher share one surface, border, focus, and disabled treatment
  and one text size, `${inputFontSize}` (app default − 1) substituted like
  `${tabFontSize}`. Verified: 88/88 Qt tests, app self-test
  474 passed / 0 failed, guard + openspec green.
- **Theme family (dark/light) + menu-bar chrome** (change
  `theme-family-light-dark`, issue #417): `Theme` gains a `ThemeFamily`
  (dark/light) with an explicit light base ramp; the token accessor, stylesheet,
  palette, and `apply` take a family, and the derived shades (hover, alternate
  row, separator-light, disabled menu text, scrollbar hover) derive toward each
  family's shade direction, so a light `#ffffff` table never washes out. The
  family persists in the session (`themeFamily`), switches from a new
  Preferences General "Theme" selector, and is toggled by the top-right menu-bar
  chrome: a sun/moon theme button plus GitHub and Discord link buttons
  (`QDesktopServices`), with new
  `assets/icons/{theme.sun,theme.moon,link.github,link.discord}.svg` (lucide
  ISC / simple-icons CC0). `Theme::currentLevel()` / `currentFamily()` back the
  Developer dialog's family+level preview. Dark values are unchanged. Verified:
  88/88 Qt tests, app self-test 474 passed / 0 failed, guard + file-size +
  colour-literal + icon-provenance green; `openspec validate --all --strict`
  255/255. `--dump-theme` also emits a `families` array (both families). The
  change is build-verified; the remaining follow-up is the light-exposed grey
  audit.
- **Lighting Effects workspace** (change `lighting-effects-workspace`, issue
  #225): `render::lighting_effects` (now `render/lighting.rs`) shades a CS6 rig
  of 1–16 Spot / Point / Infinite lights with photorust's model. A Spot is an
  ellipse whose hotspot lies toward the aimed end, and Intensity ≈ 50 is
  normal. `lighting-effects` takes 9 rig slots + 13 per light (arity table: one
  light, 22). Filter ▸ Render ▸ Lighting Effects… opens `LightingEffectsDialog`
  (`lighting_effects_dialog`, `lighting_canvas`, `lighting_rig`; tested by
  `tst_lighting_effects_dialog`). It has the options bar (17 presets + Custom,
  add-light buttons, Reset, Preview, Cancel, OK), on-canvas light controls,
  and the Properties and Lights panels. Custom preset Save/Delete and alpha
  textures are not built.
- **Filter menu wiring and dialogs** (change `filter-menu-ui`, issue #82, ported
  from photorust): the CS6 `Filter` menu is live. `cxxqt_object/filter_map.rs`
  exposes `pub(super) filter_from_kind_params(kind, &[f64])` (empty slice = the
  documented defaults; a wrong-length fixed list is refused, and variable-list
  kinds accept only their documented shapes), with `filter_from_kind`
  delegating to it; its filter parameter descriptor (`filter_map/descriptor/`)
  records every kind, including the parameterless `Blur` and `Blur More` and
  the parameterised `Oil Paint`, and
  `cpp/filter_commands.cpp` builds its rows from it. `PictureView`
  gains `apply_filter_params`, `filter_preview`, `filter_preview_cancel`,
  `filter_last_kind`/`filter_last_params`, and `filter_target_ready`. Preview
  filters a snapshot of the active layer and never records history; cancel
  restores bit-identically; commit records one `"Filter"` state and stores the
  last filter. `cpp/filter_commands.{h,cpp}` holds one row per implemented leaf,
  `cpp/filter_preview_dialog.{h,cpp}` is the shared CS6-style dialog (thumbnail
  with magnifier zoom icons and a percentage readout + Preview checkbox; Radial
  Blur drops the thumbnail), and `cpp/frame_menus_filter.cpp` wires the menu,
  `Last Filter` (`Ctrl+F`), and `Last Filter Settings` (`Ctrl+Alt+F`), marking
  every dialog entry with a trailing `…`. Filters now run on single-channel
  (Grayscale) layers too: `pictura-render::apply_filter` replicates channel `0`
  across the three working planes and writes the filtered plane back to channel
  `0`. An unlocked layer's transparency is filtered with the same kernel (a
  transparency lock keeps alpha and skips clear pixels); `apply_filter_region`
  plus `filter.preview_apron()` bound a preview to the visible section, and the dialog
  thumbnail shows that section at the canvas zoom. Every app dialog is presented
  through `runDialog` (`dialogs.{h,cpp}`) —
  non-modal with a parent input blocker and its own event loop, so KWin's
  "Dialog Parent" dim never fires (capability `ui/dialog-presentation`). Checked
  by the Qt Test suite
  `tst_filter_menu`; `commandIdForPath` centralizes the path-derived filter ids.
  Ceilings (`ponytail:`): entries with no engine kernel (Filter Gallery, Liquify,
  Vanishing Point, Lens Correction, Blur Gallery, Lens/Smart Blur, Reduce Noise,
  Glass, Diffuse Glow, Diffuse, Glowing Edges, De-Interlace, NTSC Colors,
  Digimarc) stay disabled stubs (Custom has since been exposed, #336); the alpha pass reruns the kernel on the CPU.
- PSD interop roadmap (`docs/dev/psd-support-roadmap.md`): P1 (ZIP/ZIP-prediction
  read) and P2 (opaque lossless open→save) and P2.5 are shipped. P2.5 adds a
  smart-object model and the Camera Raw Filter view on top of the preserved
  blocks: `SmartObject`/`SmartFilter` in `pictura-core`, `descriptor.rs`
  (Photoshop descriptor DOM), `smart_object.rs` (resolve `SoLd`/`SoLE`/`lnk*`),
  `smart_filter.rs` (`set_camera_raw_option` preserving object class identity),
  and `smart_writer.rs` (author `SoLd` v4 + embedded `lnk2`). It covers smart
  objects CS6→current CC by *tolerant read + byte-preserving write*, proven only
  on the synthetic fixtures `assets/test_with_smart_object0{1,2}.psd`; the Camera
  Raw settings model targets the earliest CC (ACR 8 / PV2012) `Fltr` key set,
  later-CC keys preserved, and `crs:` XMP is lifted to a typed `CrsSettings`
  view and editable in place (`crs-xmp-edit`). A CS6/earliest-CC
  fixture and the manual Photoshop reopen are deferred follow-ups.
- **Raster export** (change `raster-export`, new capability `interop/raster-export`;
  issues #60/#107, archived): `File ▸ Save`/`Save As` are
  format-aware — the output format is the save path's extension, `psd`/`psb`
  through the codec (layers preserved) and `png`/`jpg`/`tif`/`webp`/`bmp` through
  a Qt-boundary encoder (`cpp/encode_image.{h,cpp}`; `PictureView::save`
  dispatches on the suffix, and `encode_image_rgba` writes through
  `QSaveFile`+`QImageWriter` so rasters are temp+rename atomic like PSD).
  `File ▸ Export As…` and `File ▸ Quick Export as
  PNG` (and the Layers row context menu on pixel/background rows) write the
  flattened composite through `PictureView::export_image` without touching the
  document's path, dirty flag, or history; Export As is extension-authoritative
  like Save, and Quick Export prompts rather than overwrite the document's own
  `.png` source. The document remembers its import
  source format (`PictureView::output_format`) so Save As preselects it, but a
  document that needs layers (more than one layer, a group, an adjustment, or a
  type layer) defaults Save As to PSD, and Save on such a document recorded at a
  raster path reopens Save As instead of flattening silently; a `.psb` path
  forces a version-2 PSB. Open Recent, Revert, Open, drag-drop, and the control
  server all route a path through one `openDocumentAtPath` native/raster
  dispatch (previously Recent/Revert sent rasters to the PSD-only reader and
  failed silently). All file dialogs share one per-format filter set
  (`dialogs.{h,cpp}`) with uppercase patterns, a non-editable, rolling
  type-ahead "File type" combo, `All Formats` default for Open, and lowercase
  extensions on disk. Open As Smart Object uses the same filters and accepts a
  raster source by embedding its decoded layer as a smart object (the
  `place_image` recipe), matching CS6; only an unsupported file is refused. The
  dialogs are hybrid: on a native install the built-in Qt dialog's Places sidebar
  is repopulated (`fileDialogPlaces`, deduplicated by canonical path) with the
  root, mounted volumes, XDG folders, KDE `user-places.xbel` / GTK bookmark
  directories (looked up via `QStandardPaths`), and the app's recent locations;
  inside a Flatpak/Snap sandbox (`usesPortalFileDialog`) the dialogs hand off to
  the platform/portal chooser and `main()` forces
  `QT_QPA_PLATFORMTHEME=xdgdesktopportal`, so host files stay reachable (the host
  chooser owns its combo there). KIO places (`trash:/`, `remote:/`,
  `recentlyused:/`) are not representable in the built-in dialog. A file dialog
  opened without an explicit folder starts in the active document's folder, else
  the last-visited folder Qt persisted (`FileDialog/lastVisited`, read back as a
  `file://` URL rather than a path), else `~`; Export As uses the document's
  folder. The built-in
  file dialogs run without Qt's modal hint and filter the main window's input
  for their lifetime, so KWin's "Dialog Parent" effect does not dim the app
  behind them. A double-click on the empty workspace (no document open) opens
  the Open dialog (`eventFilter` on the tab pane; an app gesture, not a CS6
  behavior). A raster Revert
  reloads in place and keeps the recorded path (`open_image` no longer clears an
  existing path; a fresh import stays untitled). C++
  self-test `raster_export` (code 532) covers pixel-level reopen, alpha
  (PNG keeps, JPEG/BMP flatten), an unwritable target, empty-view refusal, the
  flatten-warning predicate, raster Revert (including the recorded path), Places
  details, portal routing, quality gating, and a raster Open As Smart Object. Ceilings
  (`// ponytail:`): raster output
  is the 8-bit sRGB display composite, and JPEG and BMP flatten alpha onto white
  (Qt's BMP writer drops alpha; the native PSD/PSB path is unchanged).
- **Native WebP codec** (change `native-webp-codec`; issue #483): WebP import and
  export no longer depend on Qt's optional `qt6-imageformats` plugin (its absence
  is why WebP files would not open). `pictura-codec` gains
  `decode_webp(bytes, budget)` (VP8/VP8L/VP8X, first frame, Rgb8 expanded to
  RGBA8888, budget checked before allocation) and `encode_webp(rgba, width,
  height)` (lossless VP8L) over the pure-Rust `image-webp` crate — the one new
  dependency. `decode_import` routes a probed WebP container to the engine and
  falls back to Qt otherwise; `save`/`export_image` route `WEBP` through a shared
  `export::encode_raster` that rescales via the boundary's new `scale_rgba`
  helper (so the percent-scale contract matches the Qt formats) and writes
  temp-file + rename. `File ▸ Export As…` gains a `WebP` format entry that
  encodes each checked target in memory through a new `webp_encode` bridge (so
  the size estimate and the write share the engine encoder), with transparency
  enabled and quality off (lossless). Oracle `webp_oracle` (magick decodes our
  lossless output exactly and we decode its lossy output within tolerance), Qt
  Test `tst_webp` (save + reopen round trip, scaled export), and `tst_export_as`
  (WebP combo entry, controls, in-memory encode) cover it. Ceiling (`//
  ponytail:`): lossless export only — no pure-Rust lossy WebP encoder exists, so
  there is no WebP quality control.
- **Pictura Raw core** (change `pictura-raw-core`, new capability `pictura-raw`):
  the Camera Raw Filter's 11 PV2012 Basic controls now round-trip and render,
  staying Photoshop's standard smart filter (`filterID 2683`, `"Camera Raw
  Filter"`) inside `SoLd.filterFX`. `pictura_core::PicturaRawSettings` is a thin alias
  of `CrsSettings`; `pictura_codec::decode_pictura_raw_settings`/`encode_pictura_raw_fltr` use
  the fixture-grounded `Fltr` keys (`Ex12` `Double`; `Temp`, `Tint`, `Cr12`,
  `Hi12`, `Sh12`, `Wh12`, `Bk12`, `Cl12`, `Vibr`, `Strt` `Long`).
  `attach_pictura_raw_filter` edits an existing filter, inserts one into a preserved
  `SoLd`/`SoLE` preserving other keys, or records it so `smart_writer` authors
  `filterFX` into a converted object's `SoLd`. `pictura_adjust::render_pictura_raw` is an
  `f32` pipeline (WB → exposure → contrast → highlights/shadows/whites/blacks →
  clarity → vibrance → saturation) and a byte-identical no-op for default
  settings; `pictura_render::apply_pictura_raw` bakes the filtered source into the layer
  proxy and records the settings (alpha/rect preserved). Ceilings (`//
  ponytail:`): behavioural parity only (no Adobe pixel oracle), clarity/contrast/
  tonal approximations, integer-typed controls round on encode, and the
  document-level `FXid`/`FEid`/`FMsk` render caches are not authored (the baked
  proxy carries the pixels). UI/menu/C++ wiring followed in change `pictura-raw-ui`: the
  modal `Pictura Raw` dialog (11 Basic controls; no tabs/filmstrip/
  histogram/preview), the `Filter > Pictura Raw…` command, and the bridge (`layer_pictura_raw_settings` prefill,
  `apply_pictura_raw_filter` convert-then-apply) record the whole raster → smart
  object → filter flow as one `"Pictura Raw"` history state.
- **Color-mode read** (roadmap P4/G2/G3, change `color-mode-read`, archived):
  `read_psd` now opens Bitmap (depth 1), Indexed, CMYK, and Lab 8-bit documents
  in addition to Grayscale/RGB and normalizes each to the working mode on load.
  Bitmap and Indexed are exact (1-bit expansion and a 768-byte palette lookup);
  CMYK and Lab are profile-free approximations marked `ponytail:`. CMYK floors
  `color * black / 255` (psd-tools/Pillow rounds, a ≤1 difference); Lab is the
  exact CIELAB(D50)→sRGB(D65) transform and matches lcms2's unoptimized transform
  within 1 LSB — psd-tools' optimized `.convert("RGB")` LUT can differ by up to
  ~20 in-gamut, so the oracle uses the exact transform, not the composite. The
  normalized document's `mode` is `Rgb`, `depth` is `Eight`, and the new
  `Document.source_mode` carries the original mode (`Some` for a normalized file);
  the Indexed palette is consumed and `write_psd` still writes the working mode,
  so open→save is a documented lossy-in-mode save. A depth-1 Bitmap layer's color
  channel is bit-unpacked like the composite. Multichannel/Duotone and all 16/32-bit
  depth stay `PsdError::Unsupported`. Four new fixtures
  (`indexed/cmyk/lab/bitmap.psd`) are compared to psd-tools/lcms2 by
  `tests/color_mode_oracle.rs`; the app shows a status-bar "Converted from …"
  notice and the C++ self-test `color_mode_open` (code 297) covers it. No new
  dependency. A depth-1 layer channel with ZIP compression is `Unsupported` (as
  for the composite); the Lab tolerance ceiling is 1 LSB against the exact
  transform. `scripts/generate-fixtures.py` regenerated `lab.psd`/`cmyk.psd`;
  `bitmap.psd`/`indexed.psd` stayed byte-stable.
- Smart-object source rendering (roadmap P3, archived
  `2026-09-19-smart-object-source-render`): `pictura-render` now depends on
  `pictura-codec` and `composite_rgba` renders an `Embedded` smart object's
  payload when the layer has no raster proxy (`composite.rs::composite_smart_source`):
  decode the payload, prefer its stored merged composite, else composite its
  layers; sample nearest-neighbour into the layer rect; `External`/`Alias`/
  `Unresolved`/empty/undecodable are no-ops. `Document.merged_composite_present`
  distinguishes a real merged composite from the codec's zero-filled placeholder
  so the layers-fallback is reachable. `Trnf`/warp and bilinear resampling are a
  `ponytail:` ceiling. A layer with a raster proxy still renders from the proxy.
- Adjustment payload decoding (roadmap P3/G8, archived
  `2026-09-19-adjustment-payload-decode`): `pictura-render::composite::decode_adjustment`
  now decodes `expA` (fixed `H3f` struct → `ExposureParams`), `vibA` (descriptor
  keys `vibrance`/`Strt` → `VibranceParams`), and `blwh` (descriptor keys
  `Rd  `/`Yllw`/`Grn `/`Cyn `/`Bl  `/`Mgnt`, `useTint`, `tintColor` 0..1 →
  `BlackWhiteParams`). `pictura-codec` exposes the descriptor DOM
  (`read_descriptor`/`write_descriptor`/`DescValue`) publicly for this. Malformed
  payloads are a no-op. Deferred because the schema is not confidently groundable
  or the op is missing: `phfl` (colour-space conversion), `mixr` (layout), `curv`
  (list/format), `selc`/`clrL`/`grdm`.
- Photo Filter adjustment decoding (roadmap P3/G8, archived
  `2026-09-19-photo-filter-adjustment-decode`): a version-2 `phfl` payload now
  decodes to `PhotoFilterParams` (R, G, B taken from the first three of the four
  `u16` colour components, `u32` density in `0..=100`, `u8` luminosity), and
  `pictura-render` exposes `encode_photo_filter` so the app can author one. The
  app maps the `photo-filter` kind to a warming layer (density 25, luminosity
  preserved) and the Adjustments panel menu offers a `Photo Filter` entry
  (`adjustment:photo-filter`). Only version 2 is decoded; a truncated payload, a
  version-3 `phfl` (CIE XYZ), a component above 255, or a density above 100 stays
  a no-op. **Curves (`curv`) is deferred**: the model is single-composite versus
  Photoshop's per-channel curves, and the legacy channel-bitmap order is
  ungrounded (no real Photoshop fixture has any adjustment key). `mixr` (layout)
  remains deferred too.
- Gradient Map adjustment decoding (roadmap P3/G8, archived
  `2026-09-19-gradient-map-adjustment-decode`): a `grdm` payload (version 1 or 3)
  now decodes to `Adjustment::GradientMap(GradientMapParams)` (16-bit colour stops
  reduced with `>> 8`, sampling a linear luminance LUT), and `pictura-render`
  exposes `encode_gradient_map` (the `* 257` inverse) so the app can author one.
  The app maps the `gradient-map` kind to a black→white layer and the Adjustments
  panel menu offers a `Gradient Map` entry (`adjustment:gradient-map`); the C++
  self-test check is code 285. The key is `grdm` — the codec whitelist previously
  misspelled it `gdrm`, so real Photoshop `grdm` blocks were unrecognized (both
  spellings are now accepted on read). Ceilings: linear-only interpolation,
  midpoint/dither/transparency stops ignored, and only RGB (no non-RGB colour
  models).
- Solid-color fill descriptor (roadmap P3/G8, archived
  `2026-09-19-solid-color-fill-descriptor`): the real Photoshop `SoCo` descriptor
  (version-16 `Clr `/`RGBC` with `Rd  `/`Grn `/`Bl  ` doubles on the 0–255 scale)
  now decodes to `Adjustment::SolidFill([r, g, b, 255])`; the 4-byte in-house form
  stays readable (length-first dispatch). `pictura-render::encode_solid_color_fill`
  and `document_ops::add_solid_fill` now author the standard descriptor, and
  `is_fill_content_layer`/`rasterize_fill_content` accept both forms through one
  decoder. Alpha ceiling: the descriptor is RGB-only, so all app callers stay
  opaque (`// ponytail:` note in `create.rs`).
- Gradient fill descriptor (roadmap P3/G8, archived `2026-09-19-gradient-fill-layer`):
  the real Photoshop `GdFl` gradient-fill descriptor now decodes to
  `Adjustment::GradientFill(GradientFillParams)` (kind Linear/Radial/Angle/
  Reflected/Diamond, angle, scale, reverse, reusing `GradientStop`), composites
  generatively over the layer rect (psd-tools' geometry), and is fill content for
  rasterize through the same `decode_adjustment`. `pictura-render::encode_gradient_fill`
  and the `Layer > New Fill Layer > Gradient…` command author a black-to-white
  Linear fill; C++ self-test codes 286/287. Ceilings: colour-noise gradients
  (`ClNs`), transparency stops, midpoint, non-linear interpolation, and non-RGB
  are ignored.
- RLE write (roadmap P3/G12, archived `2026-09-19-psd-rle-write`): `write_psd`
  now PackBits-encodes the merged composite (color + document extra channels),
  layer color channels, and the raster mask (compression 1), instead of raw.
  Preserved `Layer.raw_channels` and all unmodeled blocks stay byte-for-byte.
  The byte golden `crates/pictura-codec/tests/fixtures/default_before.psd` was
  regenerated for the RLE output (`default_document_matches_rle_golden`); the
  P2 lossless-open→save guarantee is unaffected because only engine-encoded
  channel compression changes. ZIP **write** is still missing.
- Smart-object layer actions (app, archived `2026-09-19-convert-to-smart-object`
  and `2026-09-19-rasterize-smart-object`): `Layer > Smart Objects > Convert to
  Smart Object` builds an embedded PSD source for a raster layer, keeps the proxy
  (rendering unchanged), and authors `SoLd`/`lnk2`; `Layer > Rasterize > Smart
  Object` materializes the content and drops the preserved `SoLd`/`SoLE`/`plLd`
  block plus the document `lnk*` record (`pictura-codec::remove_linked_source`).
  The embedded-source render was refactored to `render_smart_source(so, rect,
  region)` so compositing allocates only the canvas-clipped region. `File >
  Place…` (archived `2026-09-19-place-smart-object`) inserts a PSD/PSB as a
  channel-less top smart-object layer (native size at the origin, no transform
  session) that renders from its embedded source; the GPU compositor declines a
  channel-less smart-object layer so `composite_active` falls back to the CPU
  oracle. `Layer > Smart Objects > Replace Contents…` (archived
  `2026-09-19-replace-smart-object-contents`) swaps the embedded source while
  keeping the layer's geometry, clears the proxy, and drops the preserved
  `SoLd`/`lnk*` so the save re-authors the new payload. The C++ smart-object
  self-tests (277-281) live in `selftest_layers_smart_object.{cpp,h}`, taking
  `selftest_layers_controls.cpp` from the 1200 cap to 1009. `File > Open As Smart
  Object…` (archived `2026-09-19-open-as-smart-object`) opens a PSD/PSB as a new
  untitled document (path `None`, so Save cannot overwrite the source) with one
  embedded smart-object layer. `Layer > Smart Objects > Export Contents…`
  (archived `2026-09-19-export-smart-object-contents`) writes the stored payload
  byte-for-byte to a file and records no history state. Deferred: linked objects
  and the placeholder menu commands New Smart Object via Copy and Stack Mode.
- `Layer > Smart Objects > Edit Contents` (archived
  `2026-09-19-edit-smart-object-contents`) opens the embedded source as a new
  untitled editor tab; saving the editor re-embeds the edited document into the
  originating layer (`commit_smart_object_edit`, labelled `"Edit Contents"`,
  exactly one origin undo state). The implementation reuses the existing
  export/open/save/replace bridge methods through a per-session `QTemporaryDir`
  temp file (no new byte-crossing FFI). Eligibility requires an `Embedded` object
  with a non-empty payload that parses as PSD/PSB; C++ self-test codes 288/289.
- Move-tool drag start is instant: `begin_move_preview` reuses a cached base
  composite keyed by `content_revision` + topmost-layer index. The base is the
  document with the topmost layer hidden, which does not depend on that layer's
  position, so a committed move leaves it valid (`record_move` deliberately does
  not bump `content_revision`) and the next press is a pure cache hit. The
  earlier key also included the layer's clamped rect, which threw the base away
  on every drag and forced a 140–200 ms region recomposite (GPU device + layer
  upload + 64 MB clone + convert) on the next press. `applyToolPolicy` warms it
  when Move is selected or the canvas rebinds. The `move_preview_cache` self-test
  (exit 197) covers hit, byte-identical reuse, hit-after-commit, and
  miss-after-content-change; the fresh-compute path stays byte-identical.
- Drag-start latency probe: `ImageView` records the press-handler, OS
  press→move, post-handler→move, and move→paint gaps and prints a `[press-trace]`
  line to stderr when a drag start exceeds one frame (or always with
  `PICTURA_PRESS_TRACE=1`); `ToolController` logs `[move-press]` with the cache
  hit flag and `begin_move_preview` time. This is what located the replaced cache
  key above.
- Mouse-move no longer pulls the composite across the FFI:
  `PictureView::document_width()` / `document_height()` read `doc.width` /
  `doc.height` directly, so `InfoPanel::refresh()` (called on every
  `ImageView::mouseMoved`) and `PicturaMainWindow::updateStatus()` no longer pay
  a full ~64 MB `image()` copy for a size readout. `InfoPanel` still samples the
  hovered pixel through the O(1) `sample_argb`. The `document_size` self-test
  (exit 198) asserts the accessors equal `image()` and track a `resize_canvas`.
  The histogram panel still pulls `image()` because it bins the pixels; the
  document load/switch paths (`addDocument`, `refresh`) still pull it to fill the
  canvas.
- The C++ app needs **Qt6::Svg** (`Qt6Svg` CMake package) alongside the other Qt
  modules; icons and cursors render through `QSvgRenderer`.
- Image import (non-PSD Open/Place, archived `2026-09-19-image-import`):
  `File > Open` and `File > Place…` accept raster images (PNG/JPEG/GIF/BMP/TIFF/
  WebP) beside the native PSD/PSB path. Qt decodes at the app boundary to packed
  RGBA8888 (`decode_image_rgba`), so the engine stays Qt-free. A pure
  `pictura-codec::probe_image` header probe + `ImageBudget` guards the edge
  (recognized headers are dimension/allocation pre-checked, unknown containers
  deferred to Qt with a cap on the actual decoded allocation). New
  `Document::from_rgba` and `add_raster_layer_from_rgba` build engine structures
  from pixels; `place_image` reuses `convert_to_smart_object` (baked proxy,
  embedded PSD source) so Place keeps its one-undo-state contract. PSD/PSB never
  leaves the native `read_psd` path. The modified capability is
  `smart-object-layer-actions`'s Place filter; `image-import` is the new
  capability; C++ self-test `lpr_image_import` (code 290). Ceilings
  (`// ponytail:`): original-file-bytes fidelity is deferred (Export Contents
  yields the authored proxy), frame 0 only, no ICC/EXIF, and a colour image
  placed into a Grayscale document renders red-as-gray. This was Phase 1 of the
  place/drop program; OS file drag-and-drop and free transform on place have
  since shipped, completing the program.
- OS file drag-and-drop (archived `2026-09-19-file-drop-routing`): a
  `FileDropRouter` event filter on the frame routes each drop by target — a drop
  on a document `ImageView` (canvas or its surrounding space) **places** each file
  into the current document, falling back to opening tabs when no document is
  open; a drop on the tab strip, menu bar, or options bar **opens** each file as
  its own tab. PSD stays on the native `openPath`/`place_smart_object` branch and
  other images reuse the Phase 1 `open_image`/`place_image` routing; a URL-less or
  non-file drag is never consumed, so the Layers-panel internal DnD and tab
  reordering are untouched, and failures are skipped per file. C++ self-test
  `lpr_file_drop` (code 291). This was Phase 2 of the place/drop program.
- Free Transform (move/scale/rotate, archived `2026-09-19-free-transform-mode`):
  a per-`PictureView` modal session with a bounding quad, 8 handles, and a rotate
  affordance; the live preview reuses the Move-tool overlay (cached base + a
  `QTransform`-drawn layer image, no recomposite per move). Enter commits one
  `"Free Transform"` state, Escape cancels bit-identically. The engine gained
  `transform_layer` — inverse-mapped bilinear resample of every channel plane and
  the mask into the transformed bounding rect, a refusal contract (missing path,
  group, adjustment, Background, position-locked, zero-area, bad scale), and
  channel-less smart objects materialized from their payload then consumed (the
  `SoLd`/`plLd` blocks and linked-source record dropped). `Edit > Free Transform`
  (`Ctrl+T`, `edit.freeTransform`) begins a session on the current layer; a
  successful `File > Place…` or canvas drop selects the new layer and enters the
  session (cancel keeps the `"Place"` state). C++ self-test code 292. Ceilings:
  bilinear-only. This completes the place/drop program (image import + OS file
  drop + free transform).
- **Skew / Distort / Perspective** (archived `2026-09-25-free-transform-quad`):
  the engine gained `transform_layer_quad(doc, path, quad)` — the projective
  (homography) map sending the layer rect's four corners to a target quad, solved
  by an 8×8 linear system and inverted for the same bilinear resample and refusal
  contract as `transform_layer`, plus a non-finite/singular/zero-area refusal.
  The session gained a mode (`Free | Skew | Distort | Perspective`); in a
  projective mode it holds the live quad and `transform_quad` returns it, so the
  overlay and hit-test follow and the C++ preview draws the layer through the
  projective `QTransform`. Distort sets the dragged corner, Perspective moves the
  opposite corner by the negated delta (centre fixed), Skew slides one edge with
  the opposite edge fixed. `Edit > Transform > Skew|Distort|Perspective` are real
  commands, enabled like Free Transform; commit routes to `transform_layer_quad`
  and records one `"Free Transform"` state, Escape still cancels bit-identically
  (code 292). Ceilings: the gesture semantics are inferred (no Photoshop oracle);
  bilinear-only, no Warp / Puppet Warp / Content-Aware Scale.
- **Custom mesh warp engine** (change `warp-mesh`): `pictura_render::transform_layer_warp(doc, path, &WarpMesh, WarpParams)`
  resamples a layer through a tensor-product cubic Bézier surface defined by a
  row-major control net, after the options-bar row/column distortion; it reuses
  the similarity op's refusal/materialization/mask/smart-object rules (factored
  into `prepare_layer`/`write_layer`) and `integer_bbox`. Identity
  (`identity_mesh`) is bit-exact, the four corners stay fixed under an
  interior-only edit, and a degenerate/non-finite/wrong-size net is refused.
- **Preset warp styles and the Warp command** (change `transform-warp-presets`):
  `pictura_render::{WarpStyle, style_mesh(style, bend, rotate_vertical, w, h)}`
  port the 15 CS6 named preset control-net constructions from
  SethRobinson/Patchy's `generate_style_warp_mesh` (MIT, commit
  `7d14d1f6…`), which reproduces Photoshop's own bakes to ~2.4e-6 px.
  A golden control-point table (all 15 styles × five bends × both orientations,
  produced by compiling the pinned Patchy source) pins them at 1e-9; `None`/
  `Custom` produce no mesh, and `bend == 0` is the style's identity grid.
  `Edit > Transform > Warp` is now a real command: a modal `WarpPresetDialog`
  (Style, Bend, X/Y distortion, orientation) feeds the `apply_warp_preset`
  bridge, which builds the mesh from the layer rect and applies it through the
  shipped op in one `"Warp"` state (C++ self-test code 527). Ceilings: the
  interactive mesh cage / curved overlay, `View > Extras`, the custom-net drag,
  Warp Text, Puppet Warp, and Content-Aware Scale are not shipped; Patchy's
  captures are Photoshop 2026, so CS6 preset equivalence is assumed.
- **Transform options bar** (change `transform-options-bar`, issue #296): the
  Free Transform session gained a document-space **reference point** (`pivot`,
  default the source-rect centre; 3×3 presets from the live quad; canvas drag
  with a 6 screen-px snap to the nine anchors; X/Y absolute or origin-relative),
  **H/V skew** committed as the equivalent affine quad, the six-method CS6
  `pictura_render::Interpolation` (`nearest` … `bicubic-automatic`, with
  `transform_layer_with` / `transform_layer_quad_with` /
  `transform_layer_warp_with`; the legacy ops stay bilinear, so no other engine
  caller changes), and an **in-session preset warp** (the existing
  `WarpPresetDialog`; the commit applies the transform then the warp in one
  `"Free Transform"` state; box gestures are suspended while warped, and the
  preview is computed on a clone). The session-keyed options page
  (`options_bar_transform.cpp`, appended after the per-tool pages) shows the
  tool icon, show-pivot toggle, 3×3 locator, X / relative / Y, W / link / H,
  angle, H/V, interpolation, warp, Cancel, and Apply;
  `ToolController::transformChanged` fires on begin/end/gesture/setter and
  switches the bar between the page and the active tool's. The new bridge
  family lives in a secondary bridge (`impl_transform/bridge.rs`, free
  functions over `PictureView`) so the main bridge stays inside its size
  ceiling; the canvas draws the reference point (`image_view_overlays.cpp`), and
  a custom `options.warp` glyph was added. The Move tool's **Show Transform
  Controls** checkbox now shows that box on the active layer (change
  `show-transform-controls`, issue #296 follow-up): an auto-managed session
  with no reference point, handle/rotate gestures applied per release as one
  `"Free Transform"` state, inside/outside presses passing through to the
  ordinary Move drag (`"Move Layer"`), rebuilt when the active layer or its
  content rect changes, suppressed by a selection or an untransformable layer,
  and taken over by `Ctrl+T`. Ceilings (`ponytail:`): the bicubic
  kernels are inferred approximations (Adobe's are closed); the projective-mode
  angle/H/V readouts are derived from the quad; W/H setters are Free-mode only;
  the interactive warp cage stays out of scope.
- **Color Sampler, Ruler, and Note** (change `annotation-tools`, issues #6,
  #7, and #8, ported from photorust): `pictura_core::annotations` puts color
  samplers (CS6's cap of four) and notes on `Document::annotations`, undoable
  through the history snapshot; the Ruler's line is view state
  (`PictureViewRust::ruler`) with an X/Y/W/H/A/D1 readout (A negative for a
  down-right line, as CS6) and a 45° Shift snap. The tools
  (`tool_annotations.cpp`, `tool_ruler.cpp`), the always-on sampler/note
  overlay (`image_view_annotations.cpp`), the Info panel's sampler readouts, a
  real Notes panel, and the `cxxqt_object/annotations.rs` bridge land in the
  app (self-test code 535; guard 98 now probes Count). Ceilings: no PSD round
  trip for samplers or notes; no sampler Sample Size, Ruler protractor or
  Straighten, or note Author/Color.
- **Spot Healing Brush and Healing Brush** (change `healing-brush-tools`,
  issues #10 and #11, ported from photorust): `pictura_paint::healing` rebuilds
  a region from the pixels around it (Spot Healing's Proximity Match /
  Create Texture / Content-Aware, the last a patch-synthesis onion peel +
  PatchMatch refine) or from an offset source with the destination's lighting
  (Healing Brush's Poisson solve, `Transfer::Full`/`TextureOnly`); `HealStroke`
  accumulates the brush coverage mask and heals once on release through
  `heal_layer`. The `cxxqt_object/healing.rs` bridge, `tool_healing.cpp`, the
  options-bar rows (Spot Type, Healing Aligned), and the enabled J-group catalog
  rows land in the app (self-test code 536; `shift_plain` 117 now asserts the
  J cycle). Ceilings (`ponytail:`): single-threaded, Laplace/Poisson
  approximations (Adobe's biharmonic solver is closed), Sampled source only
  (no Pattern), no Sample All Layers, and no GPU path.
- **Patch** (change `patch-tool`, issue #12, ported from photorust):
  `pictura_paint::healing::patch_layer` heals the active pixel layer through a
  document-sized selection mask — Source repairs the selection from the
  dragged-to area, Destination applies the selection at the drag target,
  Transparent keeps the patched area's colour, Content-Aware rebuilds the
  selection in place and ignores the drag. `patch_selection` in
  `cxxqt_object/healing.rs` records one "Patch Tool" state; `tool_patch.cpp`
  is CS6's two-step gesture (drag outside the selection to outline, drag the
  outline to patch, with the outline previewed at the offset); the options bar
  has the combine buttons, Patch (Normal / Content-Aware), Source /
  Destination, Transparent, and a disabled Use Pattern. The J cycle is now Spot
  Healing → Healing → Patch (self-test code 537). Ceilings (`ponytail:`): no
  Adaptation, Sample All Layers, or Use Pattern; the solve runs on the GUI
  thread.
- **Content-Aware Move** (change `content-aware-move-tool`, issue #13, ported
  from photorust): `pictura_paint::healing::move_layer` copies the selection's
  pixels verbatim to the drag target and rebuilds the hole content-aware
  (Move), or copies and keeps the original (Extend). CS6's five-level
  `Adaptation` (default Medium) now sets the synthesis patch size and search
  reach — an inferred mapping; Medium is the previous fixed synthesis, so Spot
  Healing and Patch are unchanged. photorust's later-CC Structure/Color are
  not ported. `content_aware_move` (`cxxqt_object/healing.rs`) moves the
  selection with the pixels in one "Content-Aware Move" state; the
  outline-then-drag gesture is shared with Patch in `tool_region_drag.{h,cpp}`.
  Options bar: combine buttons, Mode, Adaptation, disabled Sample All Layers;
  arrow cursor with the tip as hotspot. The J cycle ends with Content-Aware
  Move (self-test code 538; guard 98 now probes Red Eye). Ceilings
  (`ponytail:`): no Sample All Layers; the solve runs on the GUI thread.
- **Red Eye** (change `red-eye-tool`, issue #14, ported from photorust):
  `pictura_paint::healing::red_eye_layer` takes the red out of pixels inside a
  document box where red dominates green and blue (Pupil Size widens the
  ratio), setting red to the green/blue level and darkening by Darken Amount;
  skin is left alone. `red_eye` (`cxxqt_object/healing.rs`) records one "Red
  Eye Tool" state; `tool_redeye.cpp` takes a dragged box (a click gets a 24 px
  box); the bar has Pupil Size and Darken Amount (50 / 50). The J cycle ends
  Content-Aware Move → Red Eye (self-test code 539; guard 98 now probes Clone
  Stamp). Behavioural parity only: CS6's detector is closed.
- **Color Replacement and Mixer Brush** (changes `color-replacement-tool` and
  `mixer-brush-tool`, issues #15 and #16, ported from photorust): the B group is
  complete. `pictura_paint::Stroke` gains `StrokeKind` / `begin_kind`: a
  `Replace` or `Mixer` stroke edits the layer at every dab (each reads what the
  last left) on the Brush's live-stroke path, so preview, commit, and cancel
  are shared. `pictura_paint::replace` matches per channel within Tolerance
  (anti-aliased taper), samples Continuous / Once / Background Swatch, limits
  Discontiguous / Contiguous / Find Edges (a per-dab flood; Find Edges stops at
  a luminance step), and blends by Hue / Saturation / Color / Luminosity, whose
  W3C helpers moved from `pictura-render` to `pictura_core::nonseparable`.
  `pictura_paint::mixer` mixes a reservoir with the tip-weighted pickup by
  Wet / Load / Mix / Flow, the load drying out and the reservoir absorbing what
  a wet brush crosses. `cxxqt_object/paint_tools.rs` begins each stroke ("Color
  Replacement Tool", "Mixer Brush Tool"); the Mixer's paint lives on the
  controller (Alt-click or a foreground choice loads it; Load / Clean after
  stroke). Bars (`options_bar_paint.cpp`): Color Replacement Mode, Sampling,
  Limits, Tolerance 30 %, Anti-alias; Mixer load swatch (Load / Clean Brush),
  the after-stroke toggles, presets, Wet / Load / Mix / Flow. The size ring and
  `[` / `]` cover all four B brushes; `shift_plain` (117) asserts the B cycle,
  `keys_shown` (116) now probes the S group (self-test codes 540, 541).
  Ceilings (`ponytail:`): no Sample All Layers or Load Solid Colors Only; the
  mixing rates and preset values are photorust's (CS6's are unstated).
- **Clone Stamp, Pattern Stamp, and History Brush** (changes `clone-stamp-tool`,
  `pattern-stamp-tool`, and `history-brush-tool`, issues #17–#19; the stamps
  ported from photorust, the History Brush original): the S group is complete.
  `Stroke::begin_source` is a Brush stroke whose colour at each pixel is read
  from a `pictura_paint::stamp::StampSource` (a document-space image at
  `destination + offset`; an off-image source leaves the pixel alone). The
  Clone Stamp snapshots `layer_surface` or, per Sample, the composite of
  `sample_scope` (Current And Below drops the layers above; Ignore Adjustment
  Layers strips adjustments under All Layers) when the stroke begins; Alt-click
  sets the source, Aligned keeps the offset across strokes, and a sourceless or
  zero-offset stroke is refused. The Pattern Stamp paints `pictura_paint::pattern`
  (eight generated seamless greyscale 64 px tiles) through `stamp::tiled`,
  Aligned pinning the tile to the document origin, unaligned to the stroke
  start. The History Brush paints the active layer's panel path from the
  `History` brush source: the oldest state by default, or a state / snapshot
  chosen in the History panel's left column (a History Brush icon marks it);
  the index follows its state as the depth limit prunes, and a source pruning
  or a post-undo capture would discard is pinned as a copy. Bridges in
  `cxxqt_object/paint_tools.rs` ("Clone Stamp", "Pattern Stamp", "History
  Brush"), handlers in `tool_stamps.cpp`, bars in `options_bar_stamp.cpp` (Size,
  Hardness, Mode, Opacity, Flow, plus Aligned / Sample / Ignore Adjustment
  Layers or the pattern picker / Aligned / disabled Impressionist); the three
  join the size ring and `[` / `]`. Fixed on the way: the Brush stroke read an
  alpha-less layer (an opened image's transparency-locked Background) as
  transparent, so it took no paint and soft edges did not blend; it now reads
  as opaque. Self-test codes 542–544; `shift_plain` (117) asserts the S cycle
  and the lone Y member, `keys_shown` (116) now probes the Y group, and guard
  98 probes the Art History Brush. Ceilings (`ponytail:`): no Clone Source
  panel, overlay, or source crosshair; built-in patterns only, no
  Impressionist; the History Brush matches the layer by panel path and has no
  merged snapshots, Erase to History, or Fill > History.
- **Brush and Clone Source panels** (change `brush-and-clone-source-panels`,
  issue #17 follow-up): the Clone Stamp bar's Toggle the Brush panel / Toggle
  the Clone Source panel buttons show or hide the two new panels
  (`PicturaMainWindow::togglePanel`); `Window > Panels > Brush` (F5) and `Clone
  Source` are live. Both sit in the hidden overflow group. `BrushPanel` is the
  Brush Tip Shape page (round tip presets, Size, Flip X / Y, Angle, Roundness
  with an indicator, Hardness, Spacing, and a preview painted by
  `brush_tip_preview`); the controller now holds Roundness / Angle / Spacing /
  flips (`brushTipChanged`), and every paint-tool begin takes one `PaintTip`
  (Brush and Pencil pass them through `begin_paint`; a single flip mirrors the
  angle, exact for the elliptical tips). `CloneSourcePanel` edits five
  controller slots (`CloneSource`: source, offset + anchor, W / H, angle,
  flips); Offset shows destination minus source. `StampSource::transformed`
  maps through the inverse scale-then-rotate about the anchor with
  premultiplied bilinear sampling. The Brush panel menu lists the spec's
  entries (disabled); Clone Source's carries only Close / Close Group. Self-test
  codes 545, 546. Ceilings (`ponytail:`): no dynamics, Brush Presets, sampled /
  bristle / erodible tips, or velocity spacing; no source overlay or frame
  controls; clone slots are controller-wide, not per document.
- **Eraser and Art History Brush** (changes `eraser-tool` and
  `art-history-brush-tool`, issues #21 and #20; the Eraser's background-colour
  erase ported from photorust's erase mode, the Art History Brush original):
  the Y group is complete. `pictura_paint::eraser::begin_erase` starts a Brush
  stroke that erases an ordinary layer to transparency (Clear: coverage ×
  Opacity multiplies alpha down) and paints the background colour on the
  Background, an alpha-less layer, or a transparency-locked layer; Mode is
  Brush / Pencil / Block (a hard square of 16 *screen* pixels at full strength,
  so its document side is round(16 / zoom) and never under one pixel,
  `StrokeConfig::square`); Erase to History,
  or Alt at the press, paints the
  History Brush source back. `pictura_paint::art_history` scatters strokes over
  the Area per dab, coloured from the source at their start, running along the
  source's edges for the Style's length / wander / curl (CS6's ten style
  names; the table is a design choice), gated by Tolerance (skip pixels within
  it of the source), seeded by the history index; `Stroke::begin_art_history`
  refuses 16/32-bit documents. Bridges `begin_eraser` ("Eraser") and
  `begin_art_history_brush` ("Art History Brush") in
  `cxxqt_object/paint_tools.rs`; handlers `tool_eraser.cpp` and
  `tool_stamps.cpp`; bars in `options_bar_stamp.cpp` (Eraser: tip, Mode,
  Opacity / Flow greyed for Block, Erase to History; Art History: tip, greyed
  Normal Mode, Opacity, Style, Area 50 px, Tolerance 0 %). Both join the size
  ring and `[` / `]`; the Eraser's Block mode draws a square cursor of the
  block's own size instead, resized with the zoom; Alt on the Eraser keeps the
  paint cursor. Self-test codes
  547, 548; `shift_plain` (117) asserts Shift+Y and E and probes the G group,
  `keys_shown` (116) now probes the E group, and guard 98 probes the Background
  Eraser. Ceilings (`ponytail:`): the Art History Brush paints only as the
  pointer moves, with Normal mode only; Tolerance defaults to 0 % (the spec's
  unverified 100 % would paint almost nowhere); both match the source layer by
  panel path.
- **Brush Preset picker** (change `brush-preset-picker`, issue #77, ported from
  photorust): every brush bar shows a tip button (the engine-drawn tip with its
  size) instead of Size / Hardness fields; a click opens `BrushPresetPicker`
  (`panels/brush_preset_picker.*`): preview, Size 1–5000 px and Hardness (box +
  slider each; the Size slider's first half is 1–100 px, the second climbs
  geometrically to 5000, `brush_size_scale`), the preset name, and photorust's
  44-entry CS6 default set with `brush_dab_preview` thumbnails. A preset writes
  size, hardness, roundness, angle, spacing, and `BrushDynamics` into the
  controller. `StrokeConfig` gained dab dynamics (count, scatter, size / angle /
  roundness jitter, seeded; none paints as before) for the Paint stroke;
  `PaintTip` carries them and Brush / Pencil now begin through `begin_brush`.
  `stroke.rs`'s tests moved to `stroke/tests.rs`. Self-test code 549;
  `lpn_brush_resync` (307) and `lpn_paint_percent` (325) read the picker.
  Ceilings (`ponytail:`): no cog menu, Use Sample Size, or libraries; sampled
  tips are approximated with scatter and jitter; per-dab tools (Color
  Replacement, Mixer, Art History) ignore the dynamics.
- **Background Eraser and Magic Eraser** (changes `background-eraser-tool` and
  `magic-eraser-tool`, issues #22 and #23, ported from photorust's
  `core/src/erase.rs`): the E group is complete. `replace.rs`'s match and
  per-dab flood are shared free functions (`match_strength`, `reachable`).
  `StrokeKind::BackgroundErase` runs `eraser::BackgroundEraser`: each dab
  samples under the crosshair (Continuous skips cleared pixels, Once, or
  Background Swatch), limits the match (Discontiguous / Contiguous / Find
  Edges), keeps a protected foreground, and multiplies alpha down with an
  always-soft match; it overrides Lock Transparency. `eraser::magic_erase`
  erases through a document-sized mask at Opacity (or blends toward the
  background colour on an alpha-less / transparency-locked layer);
  `antialias_mask` softens the wand's edge. Bridges `begin_background_eraser`
  ("Background Eraser") and `magic_erase_at` ("Magic Eraser",
  `pictura_select::magic_wand` over the layer or, with Sample All Layers, the
  composite); both turn the Background into a layer first (`ensure_alpha`).
  Handlers in `tool_eraser.cpp`, bars in `options_bar_erase.cpp` (Background:
  tip, Sampling, Limits, Tolerance 50 %, Protect Foreground Color; Magic:
  Tolerance 32, Anti-alias, Contiguous, Sample All Layers, Opacity). The eraser
  self-checks live in `selftest_erasers.cpp` with the shared
  `selftest_paint_fixture.h`; codes 550, 551; guard 98 and `keys_shown` (116)
  now probe the G group. Ceilings (`ponytail:`): no edge colour extraction or
  pen-pressure controls; the selection does not limit the Magic Eraser.
- **Gradient and Paint Bucket** (changes `gradient-tool` and
  `paint-bucket-tool`, issues #24 and #25, ported from photorust's
  `core/src/gradient.rs` and `core/src/bucket.rs`): the G group is complete.
  `pictura_paint::fill` is the per-pixel pass both share (a Brush mode through
  a document-sized selection mask; Lock Transparency keeps alpha; locked
  pixels, Clear under a transparency lock, and non-8-bit-RGB documents are
  refused); the stroke's per-pixel blend is now the shared `blend_pixel`.
  `pictura_paint::gradient` interpolates stops in straight alpha, draws the
  five styles along the drag with the ramp's ends extended past it, dithers
  the quantisation (not the position, so hard steps stay hard), and ships
  fifteen built-in gradients (the first two and Transparent Stripes follow the
  foreground / background). `pictura_paint::bucket::flood` matches every
  channel including alpha (so empty pixels are not black, unlike the Magic
  Wand), 4-connected; `fill` lays the foreground or a built-in pattern tiled
  from the document origin. Bridge `cxxqt_object/paint_tools/fills.rs` (a
  child of `paint_tools`, since `cxxqt_object.rs` sits at its size ceiling):
  `draw_gradient` ("Gradient") and `bucket_fill_at` ("Paint Bucket", layer or
  All Layers composite, `antialias_mask`). Handlers `tool_fills.cpp` (the drag
  axis reuses the canvas ruler line; Shift snaps to 45°); bars
  `options_bar_fill.cpp` (Gradient: sample + preset menu redrawn on
  `ToolController::colorsChanged`, five style buttons, Mode, Opacity, Reverse,
  Dither, Transparency; Paint Bucket: Fill, pattern picker, Mode with Clear,
  Opacity, Tolerance 32, Anti-alias, Contiguous, All Layers). Checked by the
  Qt Test suite `tst_fill_tools` (first self-test codes 552, 553, now
  retired); `keys_shown` (116) probes the O group.
  Ceilings (`ponytail:`): no Gradient Editor or noise gradients; Mode is the
  Brush modes only; the fill stays within the layer's rectangle.
- **Blur** (change `blur-tool`, issue #26, ported from photorust's
  `core/src/focus.rs`): `pictura_paint::focus::BlurBrush` is a per-dab engine
  (like the Mixer Brush) that moves each covered pixel toward its 3×3 Gaussian
  average by Strength × tip coverage — premultiplied so a layer edge softens
  without a dark rim, read from a snapshot so a dab does not smear, and
  cumulative so dwelling deepens the blur. Mode is CS6's cut-down list
  (Normal, Darken, Lighten, Hue, Saturation, Color, Luminosity; the
  non-separable ones via `pictura_core::nonseparable`); Lock Transparency keeps
  coverage. `Stroke::begin_blur` takes an optional document-space composite
  (Sample All Layers) and refuses 16/32-bit documents. Bridge `begin_blur`
  ("Blur") in `cxxqt_object/paint_tools.rs`; handler `tool_blur.cpp`; bar in
  `options_bar_paint.cpp` (tip, Mode, Strength 50 %, Sample All Layers). Blur
  joins the size ring and `[` / `]`. Checked by the Qt Test suite
  `tst_retouch_tools` (first self-test code 554, now retired). Ceiling
  (`ponytail:`): no pressure-driven Strength.
- **Sharpen, Smudge, Dodge, Burn, and Sponge** (changes `sharpen-tool`,
  `smudge-tool`, `dodge-tool`, `burn-tool`, `sponge-tool`, issues #27–#31,
  ported from photorust's `core/src/focus.rs`, `smudge.rs`, and `tone.rs`): the
  Blur and O groups are complete. `pictura_paint::focus` now carries both focus tools
  (`Focus::{Blur, Sharpen}`, `FocusOptions` with Protect Detail, `FocusBrush`,
  `Stroke::begin_focus`); Sharpen reflects a pixel through its 3×3 average,
  clamped to the neighbourhood's range with Protect Detail. The Mode enum is
  `RetouchMode`, shared with `pictura_paint::smudge::SmudgeBrush`, which lays
  down the patch picked up at the previous dab (a dab-sized patch, not a layer
  copy) and picks up the result; Finger Painting starts it loaded with the
  foreground. `pictura_paint::tone::ToneBrush` (`Tone::{Dodge, Burn,
  Sponge}`) tones each pixel once per stroke: Dodge and Burn by Range
  (Gaussian on luminance), Exposure, and Protect Tones (a luminance shift that
  keeps the colour and cannot clip); the Sponge scales the distance from grey
  (Desaturate / Saturate) by Flow, eased by Vibrance. The
  per-dab constructors share `Stroke::begin_retouch` and refuse 16/32-bit.
  Bridges `begin_focus` ("Blur" / "Sharpen"), `begin_smudge` ("Smudge"),
  `begin_tone` ("Dodge" / "Burn" / "Sponge"). The six tools share
  `tool_retouch.cpp` and
  `options_bar_retouch.cpp` (`tool_blur.cpp` and the Blur page in
  `options_bar_paint.cpp` folded in); each keeps its own options
  (`ToolController::retouchOptions(ToolId)`, `toneOptions(ToolId)`). Qt Test
  suite `tst_retouch_tools` (all six); the self-test budget is back to 511
  now that the Gradient, Paint Bucket, and Blur checks live in Qt Test. Guard
  98 now probes Pen, `keys_shown` (116) the out-of-scope 3D K group, and
  `shift_plain` (117) presses P as the unimplemented key. Ceilings
  (`ponytail:`): no Airbrush or pressure controls; a Sample All Layers smudge
  reads the composite as it was at the press.
- **Pen, Freeform Pen, Add Anchor Point, Delete Anchor Point, and Convert
  Point** (changes `pen-tool`, `freeform-pen-tool`, `add-anchor-point-tool`,
  `delete-anchor-point-tool`, `convert-point-tool`, issues #32–#36, ported from
  photorust's `core/src/path.rs`): the P group is complete. New module
  `pictura_core::path`: `VectorPath` / `Subpath` / `PathPoint` (absolute
  in/out handles, smooth flag) in `f64` document pixels, with append / drag
  handles (Alt: out only) / close / finish / resume-from-endpoint, de Casteljau
  `insert_anchor`, `delete_anchor`, `set_corner`, `drag_new_handles`,
  `move_handle`, hit tests, `flatten`, and Douglas-Peucker
  `simplify_freehand` / `add_freeform`. `Document::work_path` holds the Work
  Path and rides the history snapshot, so path edits undo. Bridge
  `cxxqt_object/paths.rs` (states "New Work Path", "Add Anchor Point", "Close
  Path", "Delete Anchor Point", "Convert Point", "Freeform Pen"; live drag steps
  record nothing until release). One handler, `tool_pen.cpp`, for all five;
  the overlay (`image_view_paths.cpp`) shows the path, anchors (the last placed
  solid), handles, and the Rubber Band / Freeform trail while a P-group tool is
  active. Bars in `options_bar_pen.cpp`: Pen Auto Add/Delete (on), Rubber Band
  (off); Freeform Curve Fit (2 px). The Paths panel (`panels/paths_panel.cpp`,
  replacing its placeholder) lists the Work Path as one italic row with a
  thumbnail once it has an anchor, refreshed with the other panels. Enter / Esc / Ctrl-click / a tool switch
  end the Pen's subpath open. P / Shift+P now cycle only the lettered members
  (`Toolbox::cycleGroup(group, key)`; Alt-click still cycles all). Qt Test
  suite `tst_pen_tools`. Guard 98 now probes Horizontal Type and `shift_plain`
  (117) presses T as the unimplemented key. Ceilings (`ponytail:`): the Work
  Path is not read from or written to the PSD path resources; no saved paths or
  Paths panel commands (fill, stroke, selection, menu), Shape / Pixels mode, path operations, Magnetic Pen, or path-to-endpoint
  connect; Freeform keeps corners only (no curve fit).
- **Horizontal Type, Vertical Type, Horizontal Type Mask, and Vertical Type
  Mask** (changes `horizontal-type-tool`, `vertical-type-tool`,
  `horizontal-type-mask-tool`, `vertical-type-mask-tool`, issues #37–#40,
  ported from photorust's `core/src/psd/text_write.rs` and its shell's type
  entry): the T group is complete and Kooka can now *create* type layers.
  `pictura_core::TypeSpec` describes point type (`\r` between lines, family,
  pixel size, RGBA, justification, orientation, anti-aliasing, the click);
  `TypeTool` gains `vertical` (`Ornt`). New module `pictura_codec::type_write`
  (`author_type_tool`): the `TySh` view with a `TxLr` text descriptor and a
  complete EngineData dump (run lengths covering the string), plus a "no warp"
  descriptor, checked by reading it back. New module `pictura_render::type_layer`:
  `type_placement` (lines `1.2 × size` apart, click on the first baseline;
  vertical: columns right to left, click on the first column's top centre),
  `vertical_layout`, `render_type`, `type_mask` (document-sized coverage), and
  `add_type_layer` (rendered pixels + authored `TySh` + the CS6 type locks,
  named after the first line, 8-bit RGB only). Read-side fixes: EngineData
  `FillColor` is alpha-first and now renders so (`TextStyle::rgba()`; black text
  had rendered red), multi-line `TySh` text splits at `\r`, and `Txt ` drops
  Photoshop's trailing NUL. Bridge `cxxqt_object/type_tools.rs`
  (`type_preview_rect` / `type_preview_rgba`, `type_commit_layer` — one
  "Horizontal Type" / "Vertical Type" state, the layer made active and selected
  via `ToolController::layerCreated` — and `type_commit_mask` — one "Horizontal
  Type Mask" / "Vertical Type Mask" state through `apply_selection_labeled`).
  One handler, `tool_type.cpp`, for all four: a click opens a session that
  edits like a text field (`type_text_edit.{h,cpp}`, photorust's caret /
  anchor model; caret stops from `pictura_render::type_caret_stops`): click /
  drag / Shift-click / double-click, arrows, Home / End, Ctrl word steps,
  Backspace / Delete, Ctrl+A / C / X / V. An application key filter takes the
  keys (letter shortcuts go quiet; other Ctrl chords still fire); Enter adds a
  line / column, Ctrl+Enter / keypad Enter / Commit / a click away / a tool
  switch commit, Esc / Cancel discard. `image_view_type.cpp` draws the engine-rendered preview and
  caret, or the red mask tint with the letters cut out. `options_bar_type.cpp`:
  Toggle Text Orientation, font family, size (px), anti-aliasing (None /
  Sharp), alignment, Cancel / Commit (enabled while typing). Host fonts:
  `type_fonts.cpp` rebuilds the face Qt resolves for the family from
  `QRawFont` tables and registers it (`pictura_render::register_font`, module
  `fonts.rs`), so layout, caret, and render use it (Liberation Sans otherwise)
  and `TySh` records its PostScript name; with a type layer selected and no
  session, a bar change re-sets the layer (changed field only, one "Edit Type
  Layer" state, `type_update_layer`), and selecting a type layer loads its
  settings into the bar. The Layers panel
  draws every type row's thumbnail as a dark T on a white card (CS6), not its
  pixels. Clicking a visible type layer with Horizontal / Vertical Type reopens
  it (`type_layer_at`, `type_layer_spec` — origin follows a moved layer — and
  `replace_type_layer`; bridge `type_edit_begin` / `type_edit_cancel` /
  `type_commit_edit`): hidden while retyped in its own orientation and
  settings, re-set in place as one "Edit Type Layer" state; Esc or blank text
  restores it. Free Transform and Skew of a type layer re-set it as type:
  `TypeSpec::matrix` (the `TySh` transform's linear part) takes the map via
  `transform_type_layer` (hooked into `transform_layer` /
  `transform_layer_quad` for affine quads), and `render_mapped` rasterizes each
  glyph outline through it (`BundledRasterizer::rasterize_mapped`), so the text
  stays sharp and editable. Qt Test suite `tst_type_tools`. Guard 98 now probes Path Selection and `shift_plain` (117)
  presses A as the unimplemented key. Ceilings (`ponytail:`): a variable font's
  named instance renders as its default instance; Warp Text is still absent, on-path type handles one subpath, and area type clips a bounding-box layout; no input-method composition; a reopened layer is
  retyped in one style, its own `TySh` replaced by the authored one, and its
  Layers row shows it hidden while retyped; vertical type is upright
  on a fixed cell; no font style, colour swatch, or Crisp / Strong / Smooth;
  8-bit RGB documents only; the mask tint covers the canvas; Distort /
  Perspective and a masked type layer still resample pixels; the live
  transform preview is the resampled raster until commit.
- **Clipping masks** (change `clipping-masks`, issue #63, from photorust's
  `core/src/compositor.rs`): the compositor now honours `Layer::clipping`
  (`composite_clipping.rs`): a base and the clipped layers above it composite
  as one isolated group — base at full opacity, clipped layers confined to the
  base's coverage (its content alone: Fill 0 and effects do not narrow it),
  the group composited with the base's opacity and mode. Every sibling loop
  (document root, pass-through and isolated groups) routes through it; the GPU
  path declines clipped layers. `layer_ops/clipping.rs`: Create / Release
  Clipping Mask (CS6 rules), wired to the Layer menu (Ctrl+Alt+G) and the
  Layers panel's Alt-click on the line between rows. Checked against
  psd-tools' compositor (`clipping_oracle`, within 2 levels) and
  `tst_layers_panel::clippingMasks`. Ceiling (`ponytail:`): inside the group a
  clipped layer's mode sees only the base and the clipped layers below it.
- **Rotate View** (change `rotate-view-tool`, issue #59, implemented fresh —
  photorust has none): `ImageView` turns the canvas about its centre; zoom and
  offset stay in the unrotated view frame, the paint applies one rotation
  (`viewRotation()`), overlays draw in the same frame, and only input mapping
  (`widgetToImage`, new `imageToWidget`, wheel anchor, drag panning) undoes it.
  `tool_rotate_view.cpp`: drag (compass with a red needle to the document's
  top), Shift 15° steps, Esc resets; `options_bar_rotate.cpp`: Rotation Angle,
  dial, Reset View, synced by `ToolController::setViewRotation` /
  `viewRotationChanged`. `shift_plain` (117) now presses K as the
  unimplemented key. Qt Test `tst_rotate_view`. Ceilings (`ponytail:`): no
  Rotate All Windows or gestures; the angle is the single canvas widget's, so
  shared by tabs; Navigator rectangle and offset clamping stay axis-aligned.
- **Save for Web & Devices** (change `save-for-web`, issue #61, from photorust's
  `SaveForWebDialog.cpp` / `GifWriter.cpp`): `pictura_codec::web` quantizes
  (Perceptual / Selective / Adaptive median-cut approximations, Restrictive,
  Black and White, Grayscale; Diffusion / Pattern / Noise dithering; matte and
  transparency) and encodes GIF89a (hashed LZW, interlace, transparency) and
  WBMP; JPEG / PNG-8 / PNG-24 go through Qt. `save_for_web_dialog` offers
  Original / Optimized / 2-Up, CS6 presets, per-format settings, Image Size,
  and size / 56.6 Kbps time; File > Save for Web & Devices… (Ctrl+Alt+Shift+S)
  is wired. ImageMagick oracle `web_oracle` decodes our GIF / WBMP exactly;
  Qt Test `tst_save_for_web`. Ceilings (`ponytail:`): no 4-Up, slices / HTML,
  colour-table editing, Lossy, Web Snap, JPEG Blur, metadata choice, PNG
  interlacing; settings last for the session.
- **Align and Distribute layers** (change `align-distribute-layers`, issue #64,
  from photorust's `Document::align_layers` / `distribute_layers` and
  `MainWindow` Move options): `layer_ops/align.rs` lines layers up by the box
  round their non-transparent pixels — Align against the union of the selected
  layers (two or more) or a selection's bounds (one is enough), Distribute three
  or more at even steps between the outermost. A position-locked layer or the
  Background holds still but counts toward the union; `crop::offset_layer` is
  the shared rect / store / mask shift. Layer > Align, Align Layers To
  Selection, and Distribute (were inert) act on the Layers panel's selection,
  each one state ("Align Top Edges", …). The Move tool gained an options bar
  (`options_bar_move.cpp`) with CS6's six Align and six Distribute buttons
  (aligning to the selection when there is one) and twelve Lucide icons. Unit
  tests ported from photorust plus Qt Test `tst_align_distribute`. Ceilings
  (`ponytail:`): groups, adjustments, and fill / shape layers are skipped; no
  Auto-Select, Show Transform Controls, or Auto-Align Layers on the Move bar.
- **Character, Paragraph, and Glyphs panels** (change `type-panels`, issue #65,
  from photorust's panels): the Character panel edits the Type tools' shared
  `TypeOptions` (family, size, colour, None / Sharp) and the Paragraph panel
  their alignment (top / centre / bottom for vertical type), both following
  outside changes; attributes the type model lacks (style, leading, kerning,
  tracking, scale, baseline shift, justification, indents, spacing, Hyphenate)
  are shown disabled. The Glyphs panel — **not a CS6 panel** (Photoshop CC
  2015), ported on the owner's request — lists the code points a font has in
  sixteen Unicode blocks and inserts one at the type caret through the new
  `ToolController::insertText`, switching the family only when it lacks the
  glyph. The three share one hidden group; Window > Character / Paragraph /
  Glyphs toggle it, Type > Panels > Character / Paragraph open it, and the Type
  bar gained CS6's panel toggle. The column's minimum width is unchanged
  (`no_hscroll_off` held at 333 px). Qt Test `tst_type_panels`.
- **Color Range** (change `color-range-dialog`, issue #67, from photorust's
  `ColorRangeDialog` / `wand.rs`): `pictura_select::color_range` now takes a
  `ColorRangeSelect` (Sampled Colors, six hue bands that exclude greys, three
  tonal bands) with Fuzziness 0–200 and Invert, replacing the unused
  sampled-only Chebyshev ramp. Select > Color Range… opens a non-modal dialog
  (Select, Fuzziness, swatch + eyedropper, Invert, greyscale mask preview);
  while its eyedropper is down `ToolController::setCanvasSampler` routes canvas
  presses to it under the eyedropper cursor. OK selects, or intersects a live
  selection (CS6's refine), as one "Color Range" state; unavailable at 32 bpc.
  Qt Test `tst_color_range`. Ceilings (`ponytail:`): no Skin Tones, Detect
  Faces, Localized Color Clusters, Out Of Gamut, plus / minus eyedroppers,
  preview modes, or Save / Load.
- **Print** (change `print-dialog`, issue #66, from photorust's `PrintDialog`):
  File > Print… (Ctrl+P) opens CS6's dialog — page preview (destination paper,
  portrait / landscape, Show Paper White), Printer Setup (CUPS queues plus a
  Linux **Save as PDF** destination, copies, Print Settings…), printer-managed
  colour — and prints the flattened sRGB image fitted to the printable area.
  Done / Print keep the settings for Print One Copy, which prints to the last
  printer or opens the dialog. Links **Qt6::PrintSupport** (system Qt / CI base
  install; no crate). Qt Test `tst_print` writes a real PDF. Ceilings
  (`ponytail:`): no Photoshop-managed colour, proofing, Position and Size,
  Print Selected Area, or marks.
- **Properties panel editing** (change `properties-panel-editing`, issue #70,
  from photorust's `PropertiesPanel`): `pictura-render`'s `adjustment_params`
  describes thirteen adjustments' controls and edits them in place — patching
  only the parameter's bytes or descriptor item, so unmodelled data survives;
  Curves re-encodes its points. The panel builds its controls from that page,
  edits the canvas live, records one "Modify … Layer" state per gesture, and
  carries Clip to Layer / Reset / Toggle Visibility / Delete; other layers get
  a read-only summary (post-CS6, ported on request). Its pages scroll so the
  panel column keeps its size (self-test 172 / 191 unchanged). Qt Test
  `tst_properties_panel` extended. Ceilings (`ponytail:`): no Presets menu,
  mask page, Previous State; Monochrome, Colorize / ranges, B&W tint, Gradient
  Map, and Color Lookup not editable.
- **Image > Adjustments** (change `image-adjustments-menu`, issue #83, from
  photorust's adjustment dialogs): the submenu and Image > Auto Tone / Contrast
  / Color were disabled stubs. Fourteen entries open a dialog (the Properties
  panel's descriptor controls, now `panels/adjustment_controls`, plus Preview /
  OK / Cancel) on CS6's defaults (`default_adjustment_block`); Invert,
  Desaturate, Equalize (new `Adjustment::Equalize`), and the Auto commands
  apply at once. They run through the Filter menu's active-layer path (an
  `ActiveOp`): layer checks, selection masking, a preview re-applied from the
  original pixels, an exact Cancel, one state named for the adjustment;
  `apply_adjustment_region` leaves transparency alone. CS6 shortcuts (Levels
  Ctrl+L, Curves Ctrl+M, Hue/Saturation Ctrl+U, Color Balance Ctrl+B, Invert
  Ctrl+I, …). Qt Test `tst_image_adjustments`. Ceilings (`ponytail:`): Color
   Lookup, Shadows/Highlights, HDR Toning, and Replace Color stay
   stubs; no dialog presets, eyedroppers, histograms, or Load / Save.
- **Image > Adjustments > Match Color** (change `add-match-color`, issue #352,
  the last CS6 destructive adjustment; lifted that stub): a Reinhard-style Lab
  mean/std transfer in `pictura-adjust::match_color`
  (`MatchColorParams`/`MatchColorStats`, local f64 CIELAB conversion — Adobe's
  kernel and space are closed, approximation marked in code). Statistics are
  computed by the `image_match_color` bridge over whole layers/composites
  (never the preview crop), alpha- and selection-weighted
  (`match_color_stats`); the modal `MatchColorDialog` lists the other open
  documents (Source), their pixel layers or Merged (Layer), the three CS6
  selection checkboxes (a new `ignore_selection` flag on
  `apply_op_active_region` applies outside the selection), Neutralize
  (gray-world), Luminance / Color Intensity 1–200/100 (intensity 1 is exactly
  gray), Fade 0–100 (right reduces, 100 identity — resolves the doc's open
  question in favour of the Help-sourced direction), live Preview, and
  Save/Load Statistics as an in-tree binary `.sta` payload (Adobe's format is
  undocumented). Dialog-only: no PSD encoding, native-depth apply refuses.
  The row is RGB-only (`document_mode`). Qt Test `tst_match_color`;
  `tst_image_adjustments` now asserts the row is live. Ceilings
  (`ponytail:`): no Info-panel eyedropper feedback while the dialog is open;
  cross-profile statistics are unconverted working composites.
- **Channels panel** (change `channels-panel-live`, issue #71, from photorust's
  `ChannelsPanel`): rows follow the document's mode (the opened file's — RGB,
  Gray, CMYK, Lab, Multichannel, Indexed, Bitmap, Duotone) with CS6's
  Ctrl+number labels, then the alpha channels. The RGB eyes hide channels on
  the canvas live (`ImageView::setChannelMask`: one multiply for two visible, a
  greyscale copy for one; the composite eye shows all; the last stays); the
  mask follows the active document. Footer: Load Channel as Selection, Save
  Selection as Channel, New / Delete Channel (one state each). Qt Test
  `tst_channels_panel` reads canvas pixels. Ceilings (`ponytail:`): eyes for
  CMYK / Lab / other source modes are disabled (channels derived from RGB
  working data); no alpha overlays or channel-targeted edits.
- **Ctrl-click channel selection** (#438, change
  `channels-ctrl-click-selection`): Ctrl-clicking a Channels-panel row now loads
  it as the selection. `ChannelsPanel::eventFilter` gains a `MouseButtonPress`
  branch that resolves the row and calls `loadRowSelection`, which routes an
  alpha row through the existing `load_selection("Alpha N")` and a colour or
  composite row through the new `PictureView::load_channel_selection(row, mask)`.
  The engine builds the selection from the sRGB composite with the row
  thumbnails' grayscale derivation (`color_rows`/`channel_value`): the composite
  row is its luminance over the visible channels (hidden RGB channels zeroed, the
  `mask` from the panel), each colour row its own plane (RGB planes, CMYK /
  Multichannel naive inks, otherwise luminance), committed as one "Load
  Selection" state. A plain click still only moves the current row. The footer
  Load button reuses `loadRowSelection`. Tests: three Rust derivation units and
  `tst_channels_panel::ctrlClickLoadsChannelSelection` plus an alpha Ctrl-click.
  Ceiling (`ponytail:`): no Shift/Alt add/subtract/intersect. Verified: nextest
  3182 passed / 23 skipped, app self-test 473, Qt 100/100, `openspec validate
  --all --strict` 272/272.
- **Shape options bar** (change `shape-options-bar`, owner follow-up on #48
  from a CS6 screenshot): all six shape tools share CS6's bar — Mode, Fill,
  Stroke (width, type pop-up with Align), W / link / H, disabled path
  operation / alignment / arrangement stubs, the geometry gear (Unconstrained
  / Square|Circle|Defined Proportions / Fixed Size, From Center; Polygon star
  and smoothing; Line arrowheads), the tool's field (Radius / Sides / Weight /
  grid Shape picker with a gear menu), and Align Edges
  (`shape::align_edges`). `pictura_render` `shape_style.rs`: Fill None is
  Fill opacity 0, the stroke an authored solid `lfx2` `FrFX` Stroke effect
  (CS6 uses `vstk`; approximation), `resize_shape` about the top-left.
  `tool_shape.cpp` mirrors the active shape layer into `ShapeOptions`
  (`ToolController::shapeOptionsChanged` re-reads every page) and applies bar
  edits back ("Change Shape Fill" / "Change Shape Stroke" / "Resize Shape").
  Tests: `tst_shape_tools::optionsBar`,
  `psd_tools_reads_a_shape_stroke_and_no_fill`. Ceilings (`ponytail:`):
  Gradient / Pattern, Dashed / Dotted, path operations, shape-set load / save
  are disabled; widths are px, not pt; no Proportional geometry.
- **Line and Custom Shape** (changes `line-tool`, `custom-shape-tool`, issues
  #47–#48, ported from photorust's `core/src/shape.rs`): the U group is
  complete. `pictura_core::shape` became a directory: `shape/line.rs` (the
  filled line of Weight, Shift snapping to 45°, optional Start / End
  arrowheads with Width / Length % of the weight and Concavity — CS6 options
  photorust lacks, approximated geometry, defaults 500 % / 1000 % against the
  doc's unsourced 100 %) and `shape/custom.rs` (photorust's six generated
  shapes, stretched to the box, Shift keeping the designed proportions,
  `custom_shape_preview` silhouettes). `ShapeSpec` gains the weight,
  arrowheads, and custom index; `shape_custom_{count,name,preview}` feed the
  options-bar picker. Options bar: Line Weight + Arrowheads pop-up, Custom
  Shape picker. A Line click draws nothing; a Custom Shape click opens
  "Create Custom Shape". Custom shape layers are named "Shape N"; neither
  kind is live. Guard 98 now probes Object Rotate. Ceilings (`ponytail:`): no
  `.csh` libraries / shape sets / Define Custom Shape / Defined Size; the
  heart is a 48-corner polygon; overlapping arrowheads on a short line are
  not clamped.
- **Live shapes** (change `live-shapes`, owner follow-up on #43–#45, from
  Photoshop **CC 2015** screenshots — a deliberate CC extension;
  `docs/03-tools/shape-tools.md` still scopes live shapes out of CS6 parity):
  Shape-mode Rectangle / Rounded Rectangle / Ellipse layers carry a `vogk`
  block (`pictura_codec::{encode,decode}_live_shape`, layout inferred from
  psd-tools / ag-psd, both of which decode it; Polygons are not live, as in
  CC 2015). `paths.rs`'s `path_set_layer_target` points the path calls at the
  active shape layer's outline (decoded from `vmsk` by
  `decode_vector_mask_paths`, written back with `set_layer_shape_paths` and a
  recomposite) for Path / Direct Selection and the shape tools, and at the Work
  Path for the Pen group. A whole-component move keeps a shape live (its box
  moves); any other edit makes it a regular path. Direct Selection prompts
  first ("turn a live shape into a regular path", Yes / No / Don't show again,
  saved as `confirmLiveShapeToPath` in `state.json`); the conversion records
  no state, so undoing the next edit restores the live shape. A click with a
  shape tool opens Create Rectangle / Rounded Rectangle (four radii) / Ellipse /
  Polygon (sides, Smooth Corners, Star, Indent Sides By, Smooth Indents) with
  From Center (`shape_dialogs.{h,cpp}`); `shape::outline_in_box` places it.
  Shape tools draw the active shape layer's anchors; Layers rows of shape
  layers show a rendered thumbnail and the `layers.kindShape` corner badge;
  the Paths panel lists "<Layer> Shape Path". Tests: `tst_shape_tools::
  liveShapes` / `::createDialogs`, `psd_tools_reads_an_authored_live_shape`.
  Ceilings (`ponytail:`): no Properties-panel live editing, a prompted drag
  ends at the (modal) prompt, Alt-drag copy / Delete convert silently, live
  reshapes recomposite the whole document per move. Note: psd-tools and ag-psd
  are not installed on the dev machine by default, so their oracles self-skip
  there; CI's `oracles` job runs them.
- **Shape tools** (changes `rectangle-tool`, `rounded-rectangle-tool`,
  `ellipse-tool`, `polygon-tool`, issues #43–#46, ported from photorust's
  `core/src/shape.rs` and its shell's `CanvasView` shape drag): Rectangle,
  Rounded Rectangle, Ellipse, and Polygon are enabled (Line and Custom Shape
  stay off). `pictura_core::shape::outline` turns a drag into one closed
  `path::Subpath` (curves stay cubic: rounded corners and the ellipse are
  quarter-arc Beziers, where photorust flattened to points); Shift squares the
  box by its longer side (the Polygon snaps its turn to 15°), Alt grows it from
  the press; the Polygon is centred on the press, first corner under the
  pointer, Sides 3–100 (default 5); Rounded Rectangle Radius 0–1000 px
  (default 10, as CS6 ships it; `shape-tools.md` says 0, unsourced), clamped to
  half the shorter side with coincident arc anchors merged.
  `shape::coverage` rasterises an outline (even-odd, 4 sub-scanlines with
  fractional span ends). New `pictura_codec::encode_vector_mask` /
  `decode_vector_mask` author and read a version-3 `vmsk`;
  `pictura_render::add_shape_layer` inserts a `SoCo` fill layer named
  `"<Tool> N"` above the active layer, cut by an authored `vmsk` — checked by a
  composite + PSD round-trip test and a psd-tools oracle
  (`vector_mask_oracle`, run with psd-tools installed). Bridge `cxxqt_object/shapes.rs`: `shape_outline`
  (preview), `shape_add_layer` (Shape), `shape_add_path` (Path, via
  `VectorPath::add_subpath`), `shape_fill_pixels` (Pixels, through the fills
  bridge's `apply` and `bucket::fill`, inside the selection); each records one
  `"<Tool> Tool"` state (label approximated). One handler, `tool_shape.cpp`:
  dashed live preview (`PathOverlay::preview`; in Path mode over the Work
  Path), Shift / Alt read live, a click draws nothing. `options_bar_shape.cpp`:
  Mode (Shape / Path / Pixels, `ShapeOptions`, kept in sync across the four
  pages), Radius, Sides. Qt Test suite `tst_shape_tools`. Guard 98 now probes
  Line and `shift_plain` (117) presses R as the unimplemented key. Ceilings
  (`ponytail:`): no click-to-size dialog, Stroke, gradient / pattern Fill,
  geometry pop-up (Fixed Size, Proportional, Star, Smooth Corners), path
  operations (Shift / Alt at press do not combine), Align Edges, or Pixels
  blend mode / opacity / anti-alias toggle.
- **Path Selection and Direct Selection** (changes `path-selection-tool`,
  `direct-selection-tool`, issues #41–#42, ported from photorust's
  `core/src/path.rs` and its shell's `CanvasView::pathSelectPress`): the A
  group is complete. `pictura_core::path` gains `move_anchor` (handles carried
  by the same delta), `move_subpath`, `duplicate_subpath`, `remove_subpath`,
  `hit_subpath` (segment or lone anchor within the radius, else inside a
  closed subpath, even-odd, topmost first), and `subpath_bounds` (flattened
  curve bounds). Bridge `cxxqt_object/paths.rs` adds the matching calls plus
  `path_commit_drag` (states "Drag Path", "Duplicate Path Component", "Drag
  Anchor Point", "Drag Direction Point") and `path_remove_subpath` ("Delete
  Path"). One handler, `tool_path_selection.cpp`, for both: Path Selection
  selects a component whole (anchors solid), drags it, Alt-drags a copy;
  Direct Selection drags an anchor (solid, the component's others hollow) or a
  handle of the selected component (smooth points stay collinear, Alt breaks),
  a segment click selects the component, Alt-click selects it whole; Delete /
  Backspace removes a whole-selected component (`removeLassoPoint`). The
  overlay builder moved to `workPathOverlay` (`path_overlay.h`, defined in
  `tool_pen.cpp`); `PathOverlay` gains `anchorsSolid` and `bounds`. Path
  Selection's bar: Show Bounding Box (off, `PenOptions::showBoundingBox`).
  Qt Test suite `tst_path_selection_tools`. Guard 98 now probes Rectangle and
  `shift_plain` (117) presses U as the unimplemented key. Ceilings
  (`ponytail:`): no segment drags, marquee, Shift-click multi-selection, arrow
  nudges, path operations / alignment / arrangement, Constrain Path Dragging,
  or drag to another document; the selection is dropped on a tool switch.
- **Count (Extended)** (change `count-tool`, issue #9, ported from photorust):
  `pictura_core::annotations` gains `CountGroup` (name, eye visibility, colour,
  marker size 1–10, label size 8–72, its own numbered marks) on
  `Document::annotations`, with an active group and add/remove/select; at least
  one group always exists and the last is not deletable. The `tool_count.cpp`
  handler adds/moves/deletes marks in the active group, and the Count options
  bar (`options_bar.cpp`) has the running total, the group dropdown, the eye /
  new-group (name dialog) / delete-group buttons, Clear, the colour swatch, and
  the marker and label size fields. The colour swatch opens a full
  Photoshop-style `ColorPickerDialog` (`color_picker_dialog.{h,cpp}`, ported
  from photorust): colour field + axis ramp, new/current compare, HSB/RGB/Lab/
  CMYK/hex fields, and Only Web Colors. The same picker's `ColorPicker` widget
  is embedded inline in the Color panel (field + ramp), and the toolbox
  foreground/background swatches open the dialog too. The canvas overlay
  (`image_view_annotations.cpp`) draws every visible group's marks as numbered
  discs in the group's colour and sizes, the disc growing with the label so a
  larger Label Size never clips the number. Covered by the `healing_tools`
  self-test (536). Ceilings (`ponytail:`): no
  Measurement Log, automatic counting, or PSD persistence; the picker's
  screen-sampling eyedropper, Add to Swatches, and Color Libraries are not
  wired; marks and groups are session document state undone through history.
- **Crop tool box** (change `crop-tool-box`, review on PR #98, ported from
  photorust): the Crop tool places a canvas-sized box with shield, thirds, and
  eight handles; move / ratio-locked resize; Enter, double-click, or Apply
  commits; Escape or Cancel resets; options bar ratio presets and Delete
  Cropped Pixels (`pictura_render::delete_cropped_pixels`, default on; live /
  native-depth layers keep their pixels). Self-test code 534. The remaining CS6
  crop options are #100–#105.
- **Crop straighten + surface parity** (change `crop-straighten`, issue #252):
  CS6's rotate-crop — the box stays axis-aligned while the content/layers spin
  about the box centre and the canvas grows to the rotated bounding box
  (`pictura_render::rotate_document_in`, reusing `pictura_ops::rotate_in`; the
  pivot is the box centre at the moment the angle is set). The crop has a
  `none → preview → active` model: Modern starts with a centered, ratio-fitted
  dashed preview box, Classic starts boxless, a drag draws a new box, a click
  adopts the preview, and Escape/Cancel return to no box. A Classic drag moves
  the box; a Modern drag pans the composite under a box fixed on screen. The
  canvas frame follows the box at every angle (padding with the backdrop or the
  transparency checkerboard); the first box is clamped and its press corner is
  pinned. The options bar's `W`/`H` are unit-less ratio values (pixels plus a
  resolution only in `W x H x Resolution`, defaulting to the document resolution,
  72 ppi fallback), `Ratio` is the first/default entry, the spirit-level button
  arms a draw-a-horizon straighten line that activates the crop, and Cancel/Apply
  show only while a box is active. The crop rotate cursor takes one of eight
  corner/edge orientations, and every tool cursor built on the workspace arrow
  fills it black. Edit Undo/Redo routes through the crop session;
  `Image > Image Rotation > Arbitrary` reuses the rotation primitive.
- **#252 UX-testing audit pass** (change `crop-straighten`, issue #252): nested
  layers are now paint/fill/filter targets — the active-layer path resolves to
  the exact leaf by walking the tree (`document/layer-nesting`), a group is not
  an edit target, and ancestors contribute only at composite time. The Layers
  panel gained a hover hand cursor, single-click lock removal, smaller locks, a
  narrower blend select, mask/vector thumbnails adjacent to the image thumbnail
  with link glyphs, active-thumb brackets, aspect-correct mask thumbs, folder-row
  sizing, a responsive visibility toggle, and a chevron on an empty nested group.
  Channels gained a hover cursor, a Ctrl select-all overlay, and Layers-style
  rows. Every active tool icon now has a separator; the Move bar gained
  Auto-Select, Show Transform Controls, and a three-dots `Align To:`
  Selection/Canvas menu; the Eyedropper gained Sample Size, Sample scope, and a
  sampling ring. Canvas Space/middle pan suppresses overlays and switching tools
  preserves zoom/position. The Info panel shows one hint per line (capitalised,
  2px smaller) with an unclipped icon menu; the Histogram gained a default
  `All Channels` RGB overlay; the footer uses Lucide chevrons; and a marquee's
  first Alt subtracts while a second Alt during the same drag mirrors the pivot.
  The Transform options bar and Select and Mask are tracked separately
  (#296, #297). Self-test `crop_tool` (534) and the new Qt Test suites cover the
  surface; the `cxxqt_object.rs` `#252` declarations moved to a `layer_path`
  bridge to stay under the file-size ceiling.
- **Open Recent** (change `open-recent-menu`, issue #62, ported from photorust):
  File > Open Recent is rebuilt each time it opens (a file opened this session
  appears at once), lists files by name with the path as tooltip plus Clear
  Recent File List, and reopens PSD/PSB or images through the File > Open
  routing (self-test code 533).
- **Perspective Crop and Slice** (change `perspective-crop-and-slice`, issues
  #3 and #4, ported from photorust): `pictura_render::perspective_crop` warps
  every pixel layer, mask, and extra channel so a user quad becomes the canvas,
  reusing Free Transform's `solve_homography`/`bilinear`, and refuses live type,
  smart objects, vector masks, and retained 16/32-bit samples.
  `Document::slices` holds user slices (undoable through the history snapshot);
  `resolve_slices` derives the auto slices. The tools, their canvas overlays
  (`image_view_overlays.cpp`), and a `cxxqt_object/crop_group.rs` bridge land
  in the app (self-test code 532); Slice Select (issue #5, change
  `slice-select-tool`) selects, moves, resizes, and deletes user slices.
  Ceilings: no slice export or PSD slices-resource round trip; no Perspective Crop options; a Background
  fills white outside the source.
- **Magnetic Lasso** (change `magnetic-lasso`, issue #2, ported from
  photorust): `pictura_select::EdgeMap` is a Sobel edge-cost field (Contrast)
  with a corridor-bounded Dijkstra live wire (Width); `tool_magneticlasso.cpp`
  fastens on click and automatically by Frequency, closes on the first point /
  double-click / Enter, peels points back with Delete, and cancels with Escape,
  committing through the shared lasso path. A second cxx-qt bridge
  (`cxxqt_object/magnetic.rs`) caches the field per gesture, so the size-capped
  `cxxqt_object.rs` does not grow; the options bar now sizes to the active page
  only (self-test code 530). Ceilings: Sobel magnitude only (no Adobe
  cost-function parity), no Alt temporary-tool switching, Caps Lock width ring,
  Stylus Pressure, or fastening-point markers.
- **Edit clipboard** (changes `edit-clipboard` and `edit-clipboard-interop`,
  issue #88, ported from photorust): Cut, Copy, Copy Merged, Paste, Paste
  Special ▸ Paste in Place / Paste Into / Paste Outside, Clear, and Purge ▸
  Clipboard are real commands. `pictura_render::Clip` keeps only the selection
  bounding box (RGBA + coverage); `copy_layer`/`copy_merged`/`clear_layer`/
  `paste_clip` do the work, and a second cxx-qt bridge
  (`cxxqt_object/clipboard.rs`) exposes them as free functions over
  `PictureView` with one process-wide clipboard, so the size-capped
  `cxxqt_object.rs` does not grow. The shell mirrors the clip to the system
  clipboard (`Clip::masked_rgba` out, `Clip::from_rgba` in for another
  application's image, placed at the origin). Cut/Clear/each paste is one
  history state, Copy/Purge none; locks refuse Clear/Cut (self-test code 529).
  Ceilings: Paste in Place has no default shortcut (menus.md gives
  `Shift+Ctrl+V` to Paste Into, unlike CS6), Paste Into makes a raster (not
  vector) mask, a Background clears to white (no background swatch), no Export
  Clipboard preference, and a platform that reports our own clipboard write
  asynchronously loses the Paste in Place origin.
- **Mask editing** (change `paint-on-layer-mask`, issue #108, archived
  2026-10-09): a plain click on a mask thumbnail makes that raster mask the
  view's edit target (a focus border on the thumbnail; a layer-thumbnail click
  or another row's selection clears it), and the Brush/Pencil, `Edit ▸ Fill`,
  and destructive filters then write the mask's coverage instead of the layer's
  pixels. `pictura_render::{mask_document, write_mask_back}` expose a mask as a
  mask-sized grayscale document (coverage in the colour channels, opacity
  locked) so the existing stroke/fill/filter engines run unchanged; the
  foreground/fill colour becomes its Rec.601 luma (black hides, white reveals).
  Every operation keeps its normal one-step label ("Brush"/"Pencil", "Fill",
  "Filter"); a filter preview snapshots and restores the coverage. Verified by
  `mask_edit` engine tests, the filter-preview bridge tests, `tst_mask_edit`,
  and `tst_layers_panel::maskEditTargetActivation`. Ceilings (`ponytail:`): the
  adapter makes a mask-sized four-plane copy per operation (a dedicated
  single-plane mask stroke is the follow-up), filter previews filter the whole
  mask rather than the viewport section, Dissolve/Behind mask paints are
  approximations, and vector masks are not paint targets.
- PSB **write** (roadmap P5/G9, archived `2026-09-19-psb-write`): `write_psd`
  now emits a version-2 PSB when `Document.is_psb` is set or either dimension
  exceeds 30 000, and a new `write_psb` always forces a PSB; both share one
  container writer and accept dimensions up to 300 000, above which the write
  returns `PsdError::Unsupported`. The PSB container widens the layer-and-mask
  section length, the layer-info length, and each per-channel data length to
  `u64`, and the RLE scanline byte-count table entries to `u32`; the global
  layer-mask info length stays `u32`. The bundled correctness fixes it required:
  PSB "big key" additional-layer-info blocks (`lnk2`/`lnk3`/`lnkE`,
  `Lr16`/`Lr32`/`Layr`, `LMsk`, `Alph`, `FMsk`, `PxSD`, `pths`,
  `Mtrn`/`Mt16`/`Mt32`, `cinf`, `extd`/`extn`, `artd`, `FXid`/`FEid`/`FELS`)
  carry an 8-byte length in a PSB (`common::is_psb_big_key`, mirroring
  psd-tools' `_BIG_KEYS`); per-layer tagged blocks declare an even length with
  the pad inside it, while document-level blocks declare the exact length and are
  padded externally to 4 (`reframe_document_extra` re-frames a preserved
  document-level block to the output container's width); and `iOpa` is written as
  a 4-byte `[fill, 0, 0, 0]` block. `Document` gained `is_psb` (set by the
  reader, false for new documents) and the reader gained the matching big-key
  widths. Proven by the psd-tools oracle: an authored-smart-object PSB, a
  reframed PSD→PSB big key, and odd/non-multiple-of-4 block framing all decode.
  Ceilings: an `8B64` document-level signature is normalized to `8BIM` on
  re-frame; a small PSB is preserved as a PSB via `is_psb`; and opening a
  >30 000 PSB through the app is still capped by the import probe budget
  (`probe.rs`, 30 000), a distinct follow-up. A per-layer `8B64` tagged block
  now reads instead of erroring (change `psd-tagged-block-8b64`; psd-tools
  accepts both signatures) and is likewise normalized to `8BIM` on write.
- Color Balance adjustment decode (roadmap P3/G8, archived
  `2026-09-19-color-balance-adjustment-decode`): `blnc` is now a recognized
  adjustment key; `pictura-render` exposes `decode_color_balance` (nine `i16`
  shifts plus a luminosity byte, each in `-100..=100`, pad ignored) and
  `encode_color_balance` (clamps to range). The app maps kind `color-balance` to
  the neutral Photoshop default (all-zero shifts, preserve-luminosity on) and the
  Adjustments panel offers `Color Balance`; C++ self-test `lpr_color_balance`
  (code 293). The engine op (`Adjustment::ColorBalance`) already existed — only
  PSD decode/encode and app wiring were added. Remaining P3 adjustment keys:
  `curv`, `mixr`, `selc`, `clrL`, and version-3 `phfl`.
- Pattern fill layers (roadmap P3/G8, archived `2026-09-19-pattern-fill-layer`):
  the `PtFl` pattern-fill descriptor and the document `Patt`/`Pat2`/`Pat3`
  pattern library are decoded (`pictura-codec::decode_patterns`), pattern-fill
  layers composite as tiled content (`pictura-render::composite_pattern_fill`)
  and are rasterizable, and `PtFl` joined `ADJUSTMENT_KEYS` so a real block is
  recognized instead of dropped to `extra_blocks`. Missing or unsupported
  patterns fall back to a grey placeholder; authoring (`Layer > New Fill Layer >
  Pattern…`) is deferred. The change brought security hardening: `pictura-codec`'s
  `inflate` now bounds ZIP/Deflate channel decompression to the expected plane
  size (a crafted pattern/channel can no longer balloon memory), and pattern
  parsing has a mode-derived channel-count gate and a pixel cap. Remaining P3:
  `curv`, `mixr`, `selc`, `clrL`, `phfl` v3, layer effects (`lfx2`/`lrFX`), text,
  vector masks.
- Layer effects — Drop Shadow (roadmap P3/G6, archived
  `2026-09-19-layer-effects-drop-shadow`): the object-based effects `lfx2`
  descriptor is now decoded for a **Drop Shadow**
  (`pictura-render::layer_effects::decode_drop_shadow`), and a layer with an
  enabled present shadow composites it behind its content, offset by
  `Distance`/`Angle`, with a `Spread` dilate, a Gaussian `Size`, and the shadow
  colour/opacity/blend, shaped by the layer mask. The matte follows pixel alpha,
  fill coverage (`SoCo`/`GdFl`/`PtFl`), or a channel-less smart source. The GPU
  rejects the document and falls back to the CPU
  (`GpuError::UnsupportedLayerEffect`). A psd-tools-authored `drop_shadow.psd`
  fixture proves decode + round-trip + rendering. Ceilings: only Drop Shadow
  (other effects, legacy `lrFX`, group/adjustment-layer effects, `Scale
  Effects`, knock-out application, and a GPU shader are deferred); the
  global-light resource (id 1037) is not decoded (the stored local angle is
  used); a canvas-filling layer at the maximum `Size` is O(canvas·size). The
  change also did a pure move: the blend-mode math moved out of `composite.rs`
  into `blend.rs` and the source-over blend was factored into a `blend_parts`
  helper (behaviour unchanged), keeping `composite.rs` under the cap.
- Layer effects — Outer Glow (roadmap P3/G6, archived
  `2026-09-19-layer-effects-outer-glow`): the object-based `lfx2` **Outer Glow**
  (`OrGl`) is now decoded and composited — exterior matte (`1 − matte`),
  `Spread` (`Ckmt`) dilate then Gaussian `Size`, colour/opacity/blend (default
  Screen, `#FFFFBE`, opacity 75, size 5, Softer), composited behind the content
  through the same bbox/early-out pipeline as Drop Shadow; the GPU rejects an
  enabled+present glow (`UnsupportedLayerEffect`) and falls back to CPU; a
  psd-tools-authored `outer_glow.psd` fixture proves decode/round-trip/render.
  Ceilings: `Precise` technique renders as `Softer`; range/contour/noise/jitter/
  anti-alias/gradient-mode glows and a GPU shader are deferred; the global-light
  resource is still not decoded.
- Layer effects — Inner Shadow (roadmap P3/G6, archived
  `2026-09-19-layer-effects-inner-shadow`): the object-based `lfx2` **Inner
  Shadow** (`IrSh`) is now decoded and composited — an interior matte
  (`M · blurred(offset, erode)` confined to the content), `Distance`/`Angle`
  offset, `Choke` (`Ckmt`) erode then Gaussian `Size`, colour/opacity/blend
  (default Multiply, black, 75), composited above the content; the GPU rejects an
  enabled+present inner shadow (`UnsupportedLayerEffect`) and falls back to CPU;
  a psd-tools-authored `inner_shadow.psd` fixture proves decode/round-trip/
  render. `composite.rs` now runs a below-content and an above-content effects
  pass (drop shadow/outer glow below, inner shadow above); the split was verified
  byte-identical to the single-pass composite. Ceilings: `knocks_out`
  (`layerConceals`) is decoded but inert; contour/noise/anti-alias, the
  global-light resource, and a GPU shader are deferred.
- Layer effects — Inner Glow (roadmap P3/G6, archived
  `2026-09-19-layer-effects-inner-glow`): the object-based `lfx2` **Inner Glow**
  (`IrGl`) is now decoded and composited — an interior matte confined to the
  content, `Source` Edge (`glwS` typeID `IGSr`, `SrcE`) or Center (`SrcC`),
  `Choke` (`Ckmt`) erode then Gaussian `Size`, colour/opacity/blend (default
  Screen, white, 75), composited above the content; a `(size 0, choke 0, Edge)`
  pair is a strict no-op. The GPU rejects an enabled+present inner glow
  (`UnsupportedLayerEffect`) and falls back to CPU; a psd-tools-authored
  `inner_glow.psd` fixture proves decode/round-trip/render. The change also did a
  pure move: `layer_effects.rs` split into `layer_effects/{mod,shadows,glows}.rs`
  (drop shadow / inner shadow in `shadows`, outer/inner glow in `glows`, shared
  helpers in `mod`), verified behavior-preserving. Ceilings: `Center` is a
  complement-of-edge approximation, linear choke, `Precise` renders as `Softer`,
  contour/noise/anti-alias ignored, and the legacy `IGsr` typeID is accepted
  leniently.
- Layer effects — Stroke (roadmap P3/G6, archived
  `2026-09-20-layer-effects-stroke`): the object-based `lfx2` **Stroke** (`FrFX`)
  solid-colour stroke is now decoded and composited as a band at the content
  edge — `Position` Outside/Inside/Centre (`Styl` typeID `FStl`), `Sz  ` size
  `1..=250`, colour/opacity/blend (default Normal, black, 100), composited above
  the content; gradient/pattern strokes are deferred (a non-solid `FrFX` decodes
  `None`); the GPU rejects an enabled+present solid stroke and falls back to CPU;
  a psd-tools-authored `stroke.psd` fixture proves decode/round-trip/render; the
  new `layer_effects/strokes.rs` holds the kind. Ceilings: gradient/pattern
  fills, contour/anti-alias/overprint, and the integer max/min band (psd-tools
  uses a doubled-radius edge mask) are deferred.
- Layer effects — Color/Gradient/Pattern Overlay (roadmap P3/G6, archived
  `2026-09-20-layer-effects-overlays`): the object-based `lfx2` overlay effects
  are now decoded and composited — **Color Overlay** (`SoFi`), **Gradient
  Overlay** (`GrFl`), and **Pattern Overlay** (`patternFill`) into typed params.
  Each fills the layer's own content coverage: the source (solid colour;
  gradient via the shared gradient geometry, layer-rect anchored when
  `align_with_layer` and canvas otherwise; pattern via the document pattern
  library, grey placeholder when the id is absent) is gated by the masked
  content matte `M` and composited **above** the content with the effect's own
  blend mode and `M · opacity/100` alpha. Overlay defaults: absent `Angl` → 0,
  absent `Type` → Linear. The GPU rejects an enabled+present overlay
  (`UnsupportedLayerEffect`, no shader) and falls back to the CPU; psd-tools
  authored `{color,gradient,pattern}_overlay.psd` fixtures prove
  decode/round-trip/render; the new `layer_effects/overlays.rs` holds the three
  kinds. The change also extracted the shared `fill.rs` helpers
  `gradient_params_from_desc`/`pattern_params_from_desc` (a pure move), which
  tightens the strict `GdFl` `Type`/`GrdF` typeID checks and rejects an `f64`
  that overflows `f32` (no existing fixture regressed). Ceilings: gradient
  `Ofst`/noise/`Dither`, pattern rotation (`Angl` decoded but not applied); the
  remaining kinds — bevel and the legacy `lrFX` block — are deferred.
- Layer effects — Satin (roadmap P3/G6, archived
  `2026-09-20-layer-effects-satin`): the object-based `lfx2` **Satin** (`ChFX`)
  is now decoded and composited — an interior directional band from the blurred
  content matte (`|B(x−dx, y−dy) − B(x+dx, y+dy)|`, offset by `Distance`/`Angle`),
  optionally inverted, tinted colour/opacity/blend (defaults Multiply, black, 50,
  angle 19, distance 11, size 14), confined to the content and composited above
  it; the GPU rejects an enabled+present satin (`UnsupportedLayerEffect`) and
  falls back to CPU; a psd-tools-authored `satin.psd` fixture proves
  decode/round-trip/render; the new `layer_effects/satin.rs` holds the kind.
  Ceilings: contour (`MpgS`)/anti-alias/global-light ignored, one `M` confinement
  (libpsd squares it), and the rounding differs from libpsd by ≤1 px.
- Layer effects — Bevel & Emboss (roadmap P3/G6, archived
  `2026-09-20-layer-effects-bevel`): the object-based `lfx2` **Bevel & Emboss**
  (`ebbl`) is now decoded and, for the **Inner + Smooth** slice, composited — a
  height field from the blurred content matte lit from `Angle`/`Altitude`
  produces highlight/shadow intensities (tinted by `hglM`/`hglC`/`hglO` and
  `sdwM`/`sdwC`/`sdwO`), confined to the content and composited above it; other
  styles/techniques decode but render a no-op; the GPU rejects an enabled+present
  renderable bevel (`UnsupportedLayerEffect`); a psd-tools-authored `bevel.psd`
  fixture proves decode/round-trip/render; the new `layer_effects/bevel.rs` holds
  the kind. Ceilings: chisel techniques, the Outer/Emboss/Pillow/Stroke styles,
  contour/texture, and the global-light resource.
- Effect blend-mode fix (cross-cutting, in `2026-09-20-layer-effects-bevel`): all
  `lfx2` effect blend modes (`Md  `, and a bevel's `hglM`/`sdwM`) now decode the
  `BlnM` descriptor vocabulary (`Nrml`/`Mltp`/`Scrn`/`Ovrl`/…) via a shared
  `effect_blend_mode`; previously they used the layer-key decoder, so real
  Photoshop effect modes silently fell back to defaults. This required
  regenerating the effect fixtures' goldens (`drop_shadow`, `outer_glow`,
  `inner_shadow`, `inner_glow`, `color`/`gradient`/`pattern_overlay`, `satin`;
  `bevel` is new) — non-effect fixtures are unchanged.
- Layer effects — legacy `lrFX` (roadmap P3/G6, archived
  `layer-effects-legacy-lrfx`): the legacy `lrFX` block (Photoshop 5.0–6.0; CS
  writes it for compatibility) is now decoded into the existing typed effect
  model (`DropShadow`/`InnerShadow`/`OuterGlow`/`InnerGlow`/`BevelEmboss`/
  `ColorOverlay`) in the new `layer_effects/legacy.rs`, and rendered through the
  shipped `lfx2` renderers. A single resolver `decode_layer_effects` prefers
  `lfx2` and falls back to `lrFX` — never both, so no effect is double-applied.
  `lrFX` uses the layer blend vocabulary (`BlendMode::from_psd_key`), unlike
  `lfx2`'s capitalized `BlnM`; gaps (technique/soften/altitude/knockout) take
  the typed defaults. The GPU rejects a renderable legacy effect through the same
  resolved set (`UnsupportedLayerEffect`). A psd-tools-authored
  `legacy_effects.psd` fixture proves decode, round-trip and render; new
  `layer_effects/legacy.rs` test module. Ceilings: only the classic `lrFX`
  record set (no satin/stroke/gradient/pattern record), contour/noise/anti-alias,
  and the shadow `blur` width ambiguity (psd-tools u32 vs libpsd u16+u16).
- Channel Mixer adjustment decode (roadmap P3/G8, archived
  `channel-mixer-adjustment-decode`): the `mixr` block now decodes into the
  existing `Adjustment::ChannelMixer` (no `pictura-adjust` change) and composites
  through the adjustment-layer path, and `pictura-render` exposes
  `encode_channel_mixer`. The layout is grounded on **ag-psd** because psd-tools'
  `ChannelMixer` reads only the red row; a committed `channel_mixer.psd` fixture
  (monochrome + non-monochrome) is proven by an ag-psd oracle plus a psd-tools
  partial check. The app gains a neutral-identity `channel-mixer` kind and
  Adjustments panel row. The adjustment self-test checks moved (pure) into
  `selftest_layers_adjustments.cpp`, where `lpr_channel_mixer` takes code 294.
  Ceilings: version 1 only; the non-monochrome gray row is read by ag-psd but
  ignored by the decoder; the two reserved bytes per channel are ignored; no
  Adobe pixel parity claim.
- Layer effects — Stroke gradient/pattern fills (roadmap P3/G6, archived
  `2026-09-20-layer-effects-stroke-fills`): the `lfx2` Stroke (`FrFX`) now
  decodes and composites **gradient** (`PntT` `GrFl`, `Grad`) and **pattern**
  (`PntT` `Ptrn`, `Ptrn`) fills in addition to solid, reusing the shipped overlay
  gradient/pattern samplers over the stroke band; the pattern link key is `Lnkd`
  (with `Algn` fallback); the GPU rejects a renderable gradient/pattern stroke;
  two psd-tools-authored fixtures `stroke_gradient.psd`/`stroke_pattern.psd`
  prove decode/render. Ceilings: the aligned-gradient edge clamp and the
  layer-rect pattern anchor are marked approximations (Photoshop's exact
  stroke-fill extent is ungrounded); the solid render path is unchanged.
- Curves adjustment decode (roadmap P3/G8, archived `curves-adjustment-decode`):
  `curv` now decodes/encodes into an extended `CurvesParams` (composite `points`
  plus optional per-channel `red`/`green`/`blue` curves), applied per-channel
  then composite. The layout is grounded on **ag-psd** with a psd-tools partial
  check; a committed `curves.psd` fixture proves decode. The legacy `is_map`
  bitmap form and the duplicate `Crv ` v4 section are ignored. The
  per-channel-then-composite order is a marked assumption (not
  Photoshop-verified). Ceilings: point counts `2..=14`; no pixel-parity claim.
- Selective Color adjustment decode (roadmap P3/G8, archived
  `selective-color-adjustment-decode`): `selc` now decodes/encodes into a new
  `Adjustment::SelectiveColor` (relative/absolute, nine ranges), implemented from
  libpsd's integer CMYK algorithm; the block layout is grounded three ways
  (libpsd plate 0 reserved + nine named ranges; ag-psd; psd-tools framing); a
  committed `selective_color.psd` fixture is proven by psd-tools + ag-psd
  oracles; the app gains a neutral `selective-color` kind/panel row. Ceilings:
  profile-free CMYK round-trip, the all-zero early-return deviation from libpsd,
  no Photoshop pixel parity, no GPU shader.
- **Color Lookup adjustment decode** (roadmap P3/G8, archived
  `2026-09-22-color-lookup-adjustment-decode`): `clrL` now decodes to a new
  `Adjustment::ColorLookup(ColorLookupParams)` (kind plus an optional parsed
  `Lut3d`), closing the whitelisted adjustment-key set. A `3DLUT` whose embedded
  `LUT3DFileData` is a `.CUBE` is parsed (`pictura_adjust::parse_cube`, red index
  fastest) and sampled trilinearly by a new `pictura_adjust::lut` kernel;
  abstract-profile, device-link, non-`.CUBE`, and malformed payloads are no-ops.
  `pictura-render` exposes `decode_color_lookup`/`encode_color_lookup`/
  `identity_cube`; the app maps kind `color-lookup` to an identity-cube layer
  (neutral composite) with an Adjustments-panel `Color Lookup` row (C++ self-test
  code 455). The committed `color_lookup.psd` fixture is proven by psd-tools and
  the ag-psd oracle. Ceilings (`// ponytail:`): `dataOrder`/`tableOrder` are
  metadata only, a non-default `DOMAIN_MIN`/`DOMAIN_MAX` is treated as `0..1`,
  and there is no Adobe pixel-parity claim. Remaining P3 adjustment key at the
  time: version-3 `phfl` (now shipped, below).
- **Version-3 Photo Filter decode** (roadmap P3/G8, change
  `phfl-v3-xyz-decode`): `decode_photo_filter` now accepts version 3 as well as
  version 2. The v3 payload is three big-endian `u32` CIE XYZ values, a `u32`
  density, and a `u8` luminosity flag (offsets 2/6/10, 14, 18 — layout grounded
  on psd-tools). XYZ is read as 16.16 fixed-point relative to D50 and converted
  with the same profile-free matrix as Lab document read
  (`pictura_codec::xyz_d50_to_srgb_u8`, shared with `lab_to_rgb`).
  `encode_photo_filter` stays version 2, so open→save of a v3 layer re-emits v2
  with the decoded colour. Truncated payloads, versions other than 2/3, and
  density `> 100` remain `None`. Ceilings (`ponytail:`): the 16.16 scale and
  D50 white are unproven without a CS6-authored v3 fixture; out-of-gamut
  components clip. This closes the last deferred adjustment key (G8); remaining
  P3 is the text (`TySh`) kind. Proven by unit tests
  (`phfl_decodes_version_three`, v2 still green, encoder round-trip); no
  app/UI change, no new dependency.
- **Type-tool model** (roadmap P3/G6, change `tysh-model-roundtrip`): a layer's
  preserved `TySh` now decodes into a derived `pictura_core::TypeTool` view
  (6×`f64` transform, `Txt ` string, four `i32` bounds, raw text/warp
  descriptor bytes). `encode_type_tool` rebuilds framing-only for an edited
  view; unmodified open→save still re-emits `extra_blocks` bytes. Malformed
  `TySh` leaves the view `None` without failing the document; kind detection
  stays presence-only (`type-layer-kind`). Ceilings (`ponytail:`): bounds are
  `i32` per psd-tools (Adobe table says "4 * 8"); EngineData styles/fonts are
  now decoded (`type-engine-data`); no glyph rasterization or Type tool.
  Proven by synthetic TySh unit tests (decode, encode round-trip, open→save,
  malformed). No app UI change, no new dependency.
- **EngineData text styles** (roadmap P3, change `type-engine-data`, archived):
  `pictura_codec::parse_engine_data` decodes the `Txt ` descriptor's `EngineData`
  (`tdta`) blob into a typed `EngineValue` tree (bounded depth/token/byte caps;
  UTF-16BE strings with escapes; MacRoman names; malformed → error, never a
  panic). `TypeTool.fonts` holds the font-set names and `TypeTool.style`
  (`pictura_core::TextStyle`: resolved font, size, fill colour, tracking,
  justification) the first run's effective values with the style/paragraph
  defaults applied. Proven by a synthetic text-layer EngineData
  fixture (`tests/fixtures/engine_data.bin`) and a psd-tools differential
  oracle, plus unit tests. Ceilings (`ponytail:`): first run only, no per-run
  layout or font-file resolution, EngineData2 (`TEXT_ENGINE_DATA`) not decoded,
  no glyph rasterization. No app UI change, no new dependency.
- **Text render seam** (roadmap P3, change `text-render-seam`, archived): the
  ADHD architecture decision for live text — the Qt-free engine owns a
  deterministic layout pass, and glyph rasterization sits behind a POD port.
  `pictura_core::layout_lines(lines, params) -> TextLayout` positions
  already-shaped glyphs (advance scaling by `size/units_per_em`, 1/1000-em
  tracking, leading, left/center/right alignment inside a `wrap_width`) with
  pure `f32` arithmetic and no font parsing; `TextAlign::from_justification`
  maps the EngineData paragraph byte. `RasterRequest`/`GlyphMask`/`trait
  Rasterizer` are the font- and Qt-free seam (a test backend ships);
  `FontPolicy` and `TextProvenance` (requested/resolved family, font content
  hash, backend+version) make substitution auditable. Ceilings (`ponytail:`):
  justification variants beyond 0/1/2 map to Left; automatic line-break/wrap
  is out of scope (the host supplies lines); no rasterizer backend, font, or
  pixels yet. Proven by layout/port/provenance unit tests. No app UI change,
  no new dependency.
- **Bundled text rasterizer** (roadmap P3, change `text-rasterize-bundled`,
  archived): `pictura-render` now depends on `fontdue` 0.9.4 and bundles
  `LiberationSans-Regular.ttf` (SIL OFL 1.1, license + provenance in
  `crates/pictura-render/assets/`). `BundledText::shape_line` maps chars to
  glyph ids + device-pixel advances; `BundledRasterizer` implements the core
  `Rasterizer` port via `rasterize_indexed` (guarded against the out-of-range
  panic); `BundledText::provenance` records the requested→Liberation-Sans
  substitution with a non-crypto font hash. `render_text_layer(doc, path)`
  shapes/lays out a type layer's `TypeTool` text+style, paints the glyph
  coverage in the fill colour into fresh `0/1/2/-1` channels, removes `TySh`,
  and clears `type_tool`; a refusal (no style, zero area, nothing painted)
  leaves the document unchanged. Ceilings (`ponytail:`): no warp/rotation,
  the file name is a substitution. Proven by six render unit tests. The app
  `Rasterize Type` command and the optional Qt backend are deferred follow-ups.
  Adds the `fontdue` dependency (pure Rust, no C deps, keeps the engine
  Qt-free) and the bundled font asset. **The command now reaches it**
  (`type-rasterize-command`): `PictureView::rasterize_layer` tries
  `render_text_layer` first and records `Rasterize Type`, else falls back to
  `rasterize_fill_content`; `Layer > Rasterize > Layer` needs no C++ change. A
  dedicated `Rasterize Type` menu entry and `Rasterize All Layers` covering type
  remain follow-ups.
- **Live type composite** (roadmap P3, change `type-live-composite`, archived):
  the shape/layout/paint body is now `render_text_buffer` (RGBA, layer-rect
  local; parsed from a process-wide `OnceLock` so compositing does not reparse
  the ~400 KB font each frame). `composite_layer` renders a **proxy-less** type
  layer live: a non-group layer with `type_tool` and no channel `0` composites
  the buffer over the canvas-clipped region with its opacity/mask/blend
  (`composite_type_source`, mirroring `composite_smart_source`); a type layer
  with a stored raster proxy is unchanged. The GPU declines a proxy-less type
  layer (`GpuError::UnsupportedText`) to the CPU oracle. Ceiling: axis-aligned
  placement at the layer rect, first-run style. Proven by three
  compositor tests plus the six renderer tests. No app change. **Rasterize All
  Layers now covers type** (`rasterize-all-type`): the engine
  `rasterize_all_fill_content` was renamed and widened to
  `rasterize_all_layers(doc)`, dispatching each flattened layer to the
  fill-content baker or `render_text_layer`; `PictureView::rasterize_all_layers`
  calls it, so one document-wide command bakes fills and type in one history
  state. **`Layer > Rasterize > Type` is now a real command**
  (`rasterize-type-command`): `PictureView::rasterize_type` materializes a type
  layer (else returns false with no history) and `layer_is_type` gates the menu
  entry, which was a disabled leaf; the C++ self-test `lpr_rasterize_refuse`
  (238) now asserts the type command refuses a plain pixel layer. **The Qt
  backend renders it** (`text-qt-backend`): `pictura-render::materialize_text_rgba`
  replaces a layer's channels from a packed RGBA buffer (wrong size/missing →
  false, no mutation); `crates/pictura-app/cpp/render_text.{h,cpp}` draws the
  text with `QFont`/`QPainter` into an RGBA `QImage` (system family resolution,
  justify, word wrap); `PictureView::rasterize_type` tries Qt first and falls
  back to the bundled `render_text_layer` on an empty render. `render_text_layer`
  now delegates its tail to `materialize_text_rgba` (behavior unchanged). C++
  self-test `lpr_qt_text` (code 525) rasterizes "Hi" and asserts non-empty
  coverage. Live compositing (`composite_type_source`) stays bundled, so goldens
  are unchanged. **Harfbuzz-grade shaping now ships** (`text-shaping-rustybuzz`):
  `shape_line` shapes each line with `rustybuzz` (a pure-Rust HarfBuzz port,
  `rustybuzz = "0.20"`, no C), so the font's `kern`/GPOS applies — `AV` at 48 px
  advances 60.47 instead of 64.03 — and provenance records
  `rustybuzz/swash`. `ShapedGlyph` now also carries the shaper's `x_offset`/
  `y_offset` (`text-shaping-offsets`), applied by `layout_lines` (glyph at
  `pen + x_offset`, `baseline - y_offset`, pen advanced by the advance alone) and
  painted per glyph — so GPOS mark positioning carries through; plain Latin has
  zero offsets, so existing output is byte-identical. **Subpixel positioning now
  ships** (`text-subpixel-positioning`): the bundled glyphs rasterize with
  `swash` (pure-Rust, honors a fractional offset via `Render::offset`), so
  `RasterRequest.subpixel_x/y` is honored and `paint_layout` floors the pen and
  passes the fraction; `'A'`@48px places identically to the old fontdue mask, a
  `0.0` vs `0.5` offset changes the coverage/width, and a `12.5` pen differs from
  `12.0`/`13.0`. **The backend is one pure-Rust stack** (`text-backend-drop-fontdue`):
  `fontdue` is removed — glyph count, character lookup, and ascent now come from
  `ttf-parser` (reached through `rustybuzz`) — leaving `rustybuzz` (shape),
  `swash` (raster), and `ttf-parser` (metrics), with no C.
  Remaining: text warp and a Qt live-composite path.
- **Multichannel and Duotone read** (roadmap P4/G2/G3, change
  `multichannel-duotone-read`): header modes 7 and 8 now open. Duotone normalizes
  like grayscale, retains the plane and `color_mode_data` (the undocumented
  duotone spec), and an unchanged flat document saves back as mode 8. The
  **Duotone Options block is now decoded** (`pictura_codec::parse_duotone`,
  change `duotone-spec-parse`): version, plate count, up to four ink color/name/
  13-point-curve/override records, dot gain, and the 0/1/4/11 overprint colors,
  grounded on the Adobe spec and `psdparse`. Rendering the duotone into the
  composite stays open (Photoshop's edit model is single-channel gray and the
  multi-ink compositing is unpublished).
  Multichannel opens only for header channel counts 1 or 3 at depth 8: 1 channel
  maps like grayscale; 3 channels map as profile-free CMY→RGB (`255-x`, marked
  ungrounded). Unchanged flat Multichannel saves back as mode 7 with retained
  plates; edited/layered falls back to RGB. Other channel counts stay
  `Unsupported`. Ceilings: Multichannel layer color ids are not converted;
  no plate layout or duotone curve is invented on edit.
- **Advanced-blending model** (roadmap G6/G7, change `knko-blend-if-model`):
  `knko`/`clbl`/`infx` consume into `Layer.knockout` / `blend_clipping` /
  `blend_interior` (defaults None/true/true; write only non-defaults) and
  `blending_ranges` parse into a typed `BlendIf` view (composite + per-channel
  black/white ranges, big-endian `u16`; empty/malformed → `None`, raw kept).
  `encode_blend_if` rebuilds the body for an edited view; unmodified open→save
  still re-emits raw ranges. Ceilings: Adobe labels `knko` a boolean while 0/1/2
  is accepted per psd-tools; the render is now in `blend-if-render`. Proven by
  seven codec unit tests (decode, defaults, round-trip,
  ranges, encode). No app UI, no new dependency.
- **Blend If compositing** (roadmap G6, change `blend-if-render`, archived):
  `composite.rs::blend_if_factor` gates a layer's CPU blend weight by its typed
  `BlendIf` view — the composite source range on the source pixel's gray, the
  composite destination range on the running backdrop's gray, and each
  per-channel group (`i < 3`) on that channel. A full `(0, 65535)` range is
  inactive, so a document without Blend If composites byte-identically. The GPU
  path declines a non-default layer (`GpuError::UnsupportedAdvancedBlending`)
  and falls back to the CPU oracle. Ceilings (`ponytail:`): Rec.601 composite
  gray and a hard 0/1 gate (no feather/split); channel groups assumed R,G,B in
  order; no Photoshop oracle. Proven by seven render unit tests plus the GPU
  parity suites (a real Vulkan device ran them).
- **Knockout punch-through** (roadmap G6/G7, change `knockout-composite`,
  archived): a top-level, non-bottom layer whose `Layer.knockout` is `Shallow`
  or `Deep` composites its content against the document background (the bottom
  layer, or transparency when it is the bottom) instead of the running backdrop,
  and replaces the running backdrop at its covered pixels, so the intermediate
  layers are punched through. `Canvas` gained an optional coverage mask that
  `blend_parts` sets; `composite_knockout` renders the layer into a background
  clone and copies only covered pixels back. The GPU declines a non-`None`
  knockout (`GpuError::UnsupportedAdvancedBlending`). Ceilings (`ponytail:`):
  the mechanism is inferred from the documented shape-against-the-stopping-point
  rule (no Photoshop oracle); clipping bases are not resolved; `Transparency
  Shapes Layers` is not applied; the bottom layer is assumed to be the
  background.
  Proven by render unit tests; `Knockout::None` and knockout-free documents
  compose byte-identically. **Independently verified** (change `knockout-oracle`,
  archived): a committed `knockout.psd` (red Background, green layer, half-fill
  blue with `knko = Deep`) is diffed against psd-tools' own compositor (which
  documents its deep knockout as "Verified against Photoshop") by
  `crates/pictura-render/tests/knockout_oracle.rs`, tolerance 1 (psd-tools
  floors 8-bit, the CPU rounds: 126 vs 127); the reference is regenerated by
  `scripts/psd_knockout_reference.py`. Authoring the fixture surfaced a real
  nuance now marked: psd-tools (and Photoshop) only treat a channel-less
  **Background** as the deep-knockout target, so a document whose bottom layer
  carries a transparency channel would knock to transparency, while the CPU's
  bottom-layer assumption targets its content. **Pass-through groups** (change
  `knockout-groups`, archived): the base threads through a group whose blend is
  Pass Through with full opacity and no mask, so a knockout child punches
  through the group to the document background — verified by a
  `knockout_group.psd` fixture in the same oracle; **an isolated group** (change
  `knockout-isolated-groups`, archived) stops the knockout at the group's own
  initial backdrop instead — its child punches through the group's earlier
  children only, and the group then composites over the layers below, verified by
  a `knockout_isolated_group.psd` fixture (red Background, yellow layer, isolated
  group of green + half-fill blue) whose psd-tools reference is `(126,127,128)`
  (the CPU gives `(127,127,128)`), discriminating the isolated base from the
  document background. No Photoshop pixel dump exists (the manual reopen stays
  deferred). **Shallow now stops at the group's entry backdrop** (change
  `knockout-shallow-base`, archived): the compositor threads two bases (`deep` =
  document background; `shallow` = the initial backdrop of the applying
  compositor) and selects by `knko`, so a `Shallow` child of a pass-through group
  no longer punches through the layers below the group while `Deep` still does;
  at the root the two coincide. Verified by a new
  `knockout_shallow_group.psd` fixture (red Background, yellow layer,
  pass-through group of green + half-fill `knko = 1` blue) against psd-tools
  **>= 1.19**, whose compositor is the first to discriminate Shallow; CI now
  pins `psd-tools>=1.19`. The knockout entry points moved to
  `crates/pictura-render/src/composite_knockout.rs` (keeping `composite.rs`
  under its cap). Ceiling: clipping-mask knockout bases remain unresolved — the
  compositor does not composite clipping masks at all and psd-tools cannot
  discriminate them.
- **Image-resource parsing** (roadmap P6/G5, archived
  `2026-09-22-psd-image-resources`): `pictura_codec::decode_image_resources`
  parses the preserved image-resource section into typed
  `ImageResource { id, name, data }` records and exposes the well-known ids
  (`ICC_PROFILE` 1039, `XMP_METADATA` 1060, `EXIF_DATA_1` 1058, `EXIF_DATA_3`
  1059, `IPTC_NAA` 1028). Parsing is tolerant: an unrecognized signature or a
  truncated block returns the records so far and never panics, and `write_psd`
  still re-emits the raw section byte-for-byte. A committed
  `image_resources.psd` (EXIF + XMP) is proven against psd-tools. Ceiling: the
  ICC profile is exposed but not yet applied (assign/convert and a save-side
  resource rewrite are the follow-up). No app change, no new dependency.
- **Embedded ICC profile applied on read** (roadmap P6/G5, archived
  `2026-09-22-psd-icc-convert`): an RGB document whose image resources carry a
  non-sRGB ICC profile (resource 1039) is converted to the sRGB working space on
  load — the composite and every layer's color channels, relative colorimetric —
  via a new `pictura-codec` dependency on `pictura-color` (lcms2). The original
  bytes are recorded in `Document.source_icc` and resource 1039 is dropped from
  the preserved resources so a save is not mis-tagged (`encode_image_resources`
  re-emits the rest byte-for-byte). A Grayscale document, an sRGB/undecodable
  profile, or one lcms2 cannot transform is left untouched. Ceilings
  (`// ponytail:`): "is sRGB" is a `Description`-contains-"srgb" heuristic, the
  intent is fixed at relative-colorimetric, and only RGB is converted. The
  fixture profile is synthesized (`psd_icc_rgb.icc` from `Profile::adobe_rgb()`;
  no Adobe file), proven against an independent PIL/lcms2 conversion, and the app
  shows a status "Converted from ICC profile …" notice. The C++ link now needs
  `-llcms2` (the staticlib does not propagate it). Still open: a File Info
  surface and a user-facing Assign/Convert Profile command.
- **Read-only File Info with decoded EXIF/IPTC/XMP** (roadmap P6/G5, archived
  `2026-09-22-psd-file-info`): `File > File Info…` (previously a disabled menu
  leaf) now opens a read-only dialog with Camera Data (EXIF), IPTC, and Raw Data
  (the raw XMP packet) categories. `pictura-codec` gains dependency-free
  `parse_exif` (TIFF/IFD walker: `Exif\0\0`-prefixed or bare TIFF, IFD0 + Exif
  sub-IFD, ASCII/SHORT/LONG/RATIONAL/UNDEFINED; a multi-value tag decodes as raw
  bytes), `parse_iptc` (IPTC-IIM `0x1C` record stream), and
  `read_metadata(&Document)` gathering resources 1058/1059, 1028, 1060. Defensive
  bounds: a per-IFD entry cap and a total cloned-value budget (a hostile resource
  cannot amplify memory), stop-and-return on malformed input, no panic. Proven by
  the byte-stable `metadata.psd` fixture against the independent `exiftool`
  decoder (self-skipping when absent) and a self-test that opens an inline
  EXIF+IPTC+XMP PSD and checks the dialog rows. Ceilings (`// ponytail:`): IFD0 +
  Exif sub-IFD only (no GPS), XMP is raw text (no field parsing), metadata is
  read-only, and the app decodes the three groups with separate bridge getters.
  Still open: editing/templates, XMP field extraction, sidecars, assign/convert
  profile.
- **IPTC editing and write-back** (roadmap P6/G5, archived
  `2026-09-22-psd-iptc-write`): the File Info dialog's IPTC page now edits the
  six core fields (Object Name, By-line, Copyright Notice, Caption/Abstract,
  Credit, Source); OK applies only the changed fields as one `File Info` undo
  state and marks the document dirty. `pictura-codec` gains
  `frame_image_resource` (the block framer), `Iptc::set`/`remove`,
  `encode_iptc`, and `set_iptc_fields(&mut Document, &[(u8,u8,String)])`, which
  re-frames resource 1028 and reassigns `Document.image_resources`. Safety:
  every other resource keeps its raw bytes, a section that does not decode
  losslessly is left untouched (no unframed bytes dropped), an IIM value longer
  than the `u16` length is truncated so no later record is lost, clearing an
  absent/empty field is a no-op, and unchanged fields record no undo state.
  Proven by `iptc_write_oracle.rs` (edit → `write_psd` → re-read + `exiftool`)
  and self-tests `file_info_metadata` / `file_info_iptc_edit` (456/457).
  Ceilings (`// ponytail:`): XMP is not edited (no IIM↔XMP sync), only the six
  core fields are editable, no EXIF editing or metadata templates, and IIM is
  written as UTF-8.
- **Assign / Convert Profile** (roadmap P6/G5, archived
  `2026-09-22-assign-convert-profile`): `Document.document_icc` holds the ICC
  bytes of the working profile the stored pixels are in (`None` = sRGB, so a
  freshly opened doc is unchanged and read-normalisation is untouched).
  `pictura-codec::assign_document_profile` retags only (rewrites
  `image_resources` with a framed 1039, preserving any unparsed tail), and
  `convert_document` transforms `doc.composite` and every layer's color channels
  (including group children) then retags; `buffer_to_srgb` converts the final
  composite to sRGB for display, so `doc.composite`/layer bytes stay in document
  space and a save emits a consistent 1039. The app wires `Edit > Assign
  Profile…` / `Edit > Convert to Profile…` (`profile_dialog`, bridge
  `assign_profile`/`convert_profile`), each one undo step, via the existing
  whole-document history. Proven by codec unit tests, `icc_oracle` /
  `profile_assignment_oracle` (lcms2 + 1039 bytes), bridge replay tests in
  `tests_impl.rs`, and self-test 458 (`profile_assign_convert`). Ceilings
  (`// ponytail:`): RGB 8-bit only, three built-in profiles only, fixed
  relative-colorimetric intent (no BPC/dither/flatten), and no Color Settings
  policy layer (`WF-011`).
- **XMP parse and edit** (roadmap P6/G5, archived `2026-09-22-xmp-metadata`):
  `pictura-codec/src/xmp.rs` parses resource 1060 into a fixed typed property
  set (title, creator, description, subject, rights, credit, source, headline,
  marked) via a bounded, entity-free, non-recursive scanner, and
  `patch_xmp` edits a property by byte-span, copying every unmanaged byte
  verbatim (unknown namespaces/properties/wrapper); a packet it cannot safely
  rewrite, or a managed property wrapping a comment, fails closed.
  `set_xmp_fields` frames 1060 (creating a minimal packet when absent) through
  the same lossless-prefix + preserved-tail pattern as `set_iptc_fields`, and
  `set_file_info_fields` writes the six IPTC-Core fields to BOTH XMP and IIM so
  the channels agree (scalar = `x-default`/first item governs the no-op; a
  non-primary alternative collapses — ceiling). File Info gains a read-only
  Description category; editing the IPTC page syncs XMP+IIM as one `File Info`
  undo state. Proven by codec unit tests, `metadata_oracle` (exiftool, ran) and
  self-tests 456/457/459. Ceilings (`// ponytail:`): nine managed properties
  only, raw packet read-only, EXIF not editable.
- **Metadata templates** (roadmap P6/G5, archived `2026-09-22-metadata-templates`):
  `pictura-codec/src/xmp/serialize.rs` serializes the nine managed properties to
  a standalone packet and `metadata/template.rs` adds
  `MergeMode { Append, Replace, KeepOriginalReplaceMatching }`, `export_template`
  (managed fields only) and `apply_template` (patches in place; Append fills
  empty only, Replace overwrites and clears template-omitted fields, Keep
  Original overwrites only template-defined fields). Lists merge as full
  `rdf:Seq`/`rdf:Bag` via `patch_xmp_values`; the six shared fields route through
  `set_file_info_fields` so XMP and IIM agree. File Info gains Export/Apply
  Template controls + a mode combo (dialog stays open; callbacks call the bridge
  `export_metadata_template`/`apply_metadata_template`), apply = one
  `Metadata Template` undo state, export = no mutation. Proven by codec unit
  tests, `metadata_oracle` (exiftool, ran, all three modes), self-test 460.
  Ceilings (`// ponytail:`): nine managed properties only, no sidecars/template
  folder/batch, `xmp.rs` is near the 1200-line cap.
- **PSD ZIP write and compression preservation** (roadmap P3/G12, archived
  `2026-09-22-psd-zip-write`): `pictura-core::Compression` (`Rle` default, `Raw`,
  `Zip`, `ZipPrediction`) and `Document.{composite,layer}_compression` record the
  kind read; `write_psd` emits it via `zip_scanlines` (zlib, with a reversible
  per-row delta for ZIP-prediction) and `channel_stream`, for the composite,
  document extra channels, layer color channels, and raster mask. A constructed
  document stays RLE and byte-unchanged; a document with no merged composite now
  writes no image-data section, so a maximize-compatibility-off file round-trips
  equal; only surviving layer records set the layer kind. Proven by codec unit
  tests and the psd-tools write oracle (ran) across raw/RLE/ZIP/ZIP-prediction.
  Ceilings (`// ponytail:`): mixed per-channel kinds within a category normalize
  to the first seen; no user compression choice, no per-channel fidelity.
- **Color Settings / incoming-profile policy** (roadmap P6/G5, archived
  `2026-09-22-color-settings`): `pictura_color::Policy { Preserve (default),
  Convert, Off }` (codes 0/1/2) and `pictura_codec::read_psd_with(bytes, policy)`
  decide how an RGB document's decodable non-sRGB embedded profile is honoured:
  Preserve keeps the pixels + resource 1039 and sets `Document.document_icc` (the
  canvas converts it for display via `buffer_to_srgb`); Convert is the old
  normalisation; Off drops 1039 and stays untagged. An untransformable profile
  (e.g. CMYK on a normalised-RGB doc) is unchanged under every policy.
  `read_psd` stays `Convert` for existing callers (incl. nested smart-object
  reads). The app persists the policy as an application preference (default
  Preserve), pushes it to the view before opening, displays the first frame
  through the profile conversion, and `Edit > Color Settings…` is a real dialog
  (sRGB working space, policy combo). Proven by codec unit tests + `icc_oracle`
  (PIL, ran) and self-tests 461/462. Ceilings (`// ponytail:`): sRGB working
  space only, RGB policy only, no `.csf`, no mismatch/missing dialogs.
- **Agentic control server** (archived `2026-09-22-app-control-server`, plan
  `docs/dev/mcp-agentic-control-plan.md` P0/P1): `./build/pictura --control
  [--control-socket PATH] [--state-home DIR]` starts a `QLocalServer` on a
  per-user Unix socket (owner-only, default
  `$XDG_RUNTIME_DIR/pictura-control.sock`) speaking newline-delimited JSON on
  the GUI thread. Methods `status`, `get_pixel`, `list_layers`, `list_commands`,
  `dispatch_command`, `document`, `edit`, `set_unsaved_policy`; errors
  `bad_request`/`unknown_method`/`invalid_param`/`no_document`/`not_implemented`/
  `refused`/`io_error`/`internal`. `CommandRegistry::describe()` added. Control
  mode is non-interactive by construction: unsaved policy is non-interactive/
  Discard, `document` uses the non-dialog entry points (a pathless `save` →
  `invalid_param`), and `dispatch_command` refuses modal-opening commands (a
  hand-maintained denylist, `// ponytail:`) with `refused`, so no request blocks
  on a modal. Socket is owner-only (Qt mode 0700), no TCP, no eval, oversize
  requests rejected. Proven by the `mcp_control` self-test block (codes 463+).
  **Vision methods** shipped (archived `2026-09-23-agentic-control-vision`):
  `screenshot` (`scope` window/canvas/document, `max_dim` default 1280, PNG as
  base64 with source dimensions), `ui_tree` (capped widget tree with
  window-local rects, `max_depth` default 12, `max_children` 64), and
  `layer_thumbnail` (`index`, `size` default 64). Caller-supplied image sizes are
  capped at the trust boundary (thumbnails 1024, screenshots 4096) because the
  engine allocates `size²` and `max_dim` would otherwise bypass the response-size
  guard; the vision self-test block is codes 483–492. **Engine actions**
  shipped (archived `2026-09-23-agentic-control-actions`): `set_tool` (a
  string→`ToolId` lookup accepting the `status` name or label), `selection`
  (all/deselect/rect/ellipse/lasso/quick/wand → `{has_selection,count,bounds}`),
  `filter`/`adjustment` (kind-only, fixed seed; unknown → `invalid_param`,
  locked/hidden target → `refused`), `layer_op` (name/opacity/visible/blend/
  fill/lock/color/move/translate/delete/duplicate/add → `{ok,layers}`), and
  `set_gpu_compute`; action self-test block codes 493–517. **Input synthesis**
  shipped (archived `2026-09-23-agentic-control-input`): `pointer`
  (click/dblclick/move/drag/scroll in `window` or `image` space; image
  coordinates map as the exact inverse of `ImageView::widgetToImage`,
  `w = image*zoom + offset`) and `key` (a sequence such as `Ctrl+Z`/`Shift+F2`
  parsed to a modifier set and key, sent as a `QKeyEvent` press/release to the
  focused widget else the frame), both via `QApplication::sendEvent` with no
  `Qt6::Test`/private API. Qt 6.11 routes a non-spontaneous key press through the
  shortcut machinery (a `ShortcutOverride` is sent first), so a shortcut fires in
  an active window and a key no shortcut claims reaches the widget handler; a
  window-scoped shortcut matches only while the window is active, so the
  in-process self-test (pre-activation) asserts the widget path with the Move
  tool's arrow nudge and the shortcut path is left to the live `verify-control.sh`
  (P5). Input self-test codes 522–524. **Live e2e** (archived
  `2026-09-23-agentic-control-e2e`): `scripts/verify-control.sh` launches a
  throwaway `pictura --headless --control` on a temp socket, drives
  status→rect-select→`add-noise`→a 64-pixel diff (changed inside the selection,
  none outside)→undo→screenshot over the real newline-JSON protocol, and exits
  non-zero on a mismatch or a missing binary; a trap kills the process and
  removes the socket. It is the only check that exercises
  `QLocalServer`/`QLocalSocket` and a separate process (the in-process block
  calls `dispatch` directly); a subset-of-the-active-layer selection makes a
  mask-ignoring filter fail. Wired into `verify-full.sh` after the build and into
  the CI `qt-headless` job; `verify-fast.sh` stays build-free. Ceilings:
  `pictura-mcp` stdio frontend (P4), the P5 client registration/long-form docs,
  native/global shortcuts, and `filter` caller seed/params remain.
- **Bit-depth preservation** (roadmap P4/G4, archived `2026-09-23-depth-preserve`):
  a 16/32-bit **Grayscale or RGB** document no longer downgrades to 8-bit on
  save. `read_psd` retains the decoded source-depth samples of the composite
  color planes, document extra channels, and every layer channel (stored on
  `Document.source_planes` / `Layer.source_channels`, keyed by channel id in
  canonical order); the 8-bit engine model is unchanged. `write_psd` writes the
  header at the source depth and, per plane, re-encodes the retained samples at
  the recorded compression when the plane is unchanged (its narrowing still
  equals its 8-bit bytes) and widens the 8-bit plane (`v*257` at 16, scaled to
  `[0,1]` at 32) when it changed. Compression is depth-aware: PackBits stays
  byte-wise with a native row stride, ZIP-with-prediction uses the depth-specific
  forward predictor (inverse of the read-side undo). An 8-bit or constructed
  document is byte-identical; a mode the read path converts (CMYK/Lab) still
  saves 8-bit (the app notice reports that case); the retained copy is dropped on
  a scale/rotate/flip (`// ponytail:` an unmodeled channel cannot be resampled)
  and kept on a pure translation. Proven by the `psd-tools` write oracle
  (composite + layer channels, all four compressions, both depths, and a grouped
  document whole-`Document`-equal). The retained store is now sample-typed
  (`bit-depth-sample-model`): `SourcePlanes`/`SourceChannels` hold `u8`/`u16`/
  `f32` samples with the shared `Samples::narrow_to_u8`/`widen_from_u8` rules and
  `PixelBuffer<T = u8>` is generic, so 16/32-bit planes are decoded to native
  values instead of raw PSD bytes (byte-exact `to_bytes`/`from_bytes` round-trip;
  no pixel-behavior change). The tonal adjustment family now applies at native
  depth too (`native-depth-adjustments`): `pictura_adjust::apply_native` runs
  `Invert`/`Desaturate`/`Levels`/`Curves`/`BrightnessContrast`/`Exposure`/
  `Posterize`/`Threshold`/`GradientMap` on a `Samples` store in a unit-domain
  `f64` kernel (the 8-bit `apply` is byte-identical; ImageMagick oracle unmoved),
  and a depth-16 edit saved and re-read keeps its low bits instead of the
  widen-on-edit `high*257`. The color-preserving family now applies natively too
  (`native-depth-color-adjustments`: `HueSaturation`, `Vibrance`, `ColorBalance`,
  `BlackWhite`, `PhotoFilter`, `ChannelMixer`, `SelectiveColor`), still
  byte-identical on u8 (Selective Color keeps its 8-bit integer pipeline, a
  marked ceiling). The CPU compositor now gets it too
  (`native-depth-composite`): for a 16/32-bit document an adjustment layer is
  applied to the `f32` canvas through `apply_native` instead of the former
  `to_u8`/`from u8` round-trip, `Unsupported` adjustments fall back to the 8-bit
  path, and `pictura_render::composite_native(doc)` emits the canvas at the
  source depth (`U16`/`F32`, `None` for 8-bit); the 8-bit composite is
  byte-identical. A high-depth RGB/Grayscale pixel layer also composites from
  its retained native samples (`native-depth-layer-content`; a channel missing
  from the store falls back to its 8-bit plane, and a converted Lab/CMYK read
  keeps the 8-bit path since its store is source-mode planes). `Auto` and
  `ColorLookup` are in the native set too (`native-depth-remaining-adjustments`:
  native histogram/stretch for `Auto`, trilinear `ColorLookup` on unit values;
  u8 stays byte-identical), so the only native-unsupported adjustments are the
  generative fill kinds. A high-depth layer also gates on its native `-2` mask
  plane (`native-depth-masks`; unit-space raster×vector, falling back to the
  exact 8-bit `mask_alpha` when there is no native plane). A dirty save of a
  16/32-bit RGB/Grayscale layered document now re-emits its composite color
  planes from `composite_native` (`native-depth-save`:
  `pictura_render::refresh_native_composite` splices native color planes into the
  retained store and refreshes `doc.composite`; a clean, no-layer, or
  converted-mode save is untouched, so the codec depth oracles stay
  byte-identical). The **Exposure & Gamma** HDR tone-map operator ships as a
  pure linear-light function (`pictura_adjust::hdr_toning::exposure_gamma`,
  change `hdr-exposure-gamma`), and the open `hdr-conversion` change now wires
  it into a real 32→16/8 conversion
  (`pictura_render::document_ops::convert_depth_exposure_gamma`): for a native
  32-bit Grayscale/RGB document it tone-maps the retained `f32` color planes,
  quantizes to the output depth (`Samples::to_u16` at 16, `narrow_to_u8` at 8),
  rebuilds `source_depth`/`source_planes` so a save writes the converted depth,
  and clears every layer's 32-bit store; the app adds the
  `Image > Mode > 16/8 Bits/Channel` commands (real ids, gated on
  `document_depth_bits() == 32`, which now reports the retained source depth)
  and a two-field Exposure/Gamma `HdrConversionDialog` plus a `convert_depth`
  bridge. Ceilings: only Exposure & Gamma is grounded (the Method combo and
  Local Adaptation/Equalize Histogram/Highlight Compression have closed
  kernels); the ranges beyond Exposure 0 / Gamma 1.0 are inferred; an
  already-edited 32-bit document was clamped to `[0, 1]` by `emit_native`, so
  only a clean open tone-maps values above white; 32-bit Lab/CMYK are refused;
  layered documents tone-map the merged composite and drop per-layer native
  stores; a moved/edited layer channel now keeps a resampled (or re-anchored)
  native store (`native-depth-edit-preserve`), while unmodeled raw streams,
  merge/flatten/via-copy/rasterize, and a true working-depth model stay open;
  retained samples cost
  2×/4× while open and the write path clones each retained plane.
- **Native-store edit preservation** (roadmap P4/G4, open change
  `native-depth-edit-preserve`): the destructive geometry ops now keep a
  layer's retained native samples instead of widening them on save. Move/
  translate offset `Layer.source_channels.rect` with the layer bounds; canvas
  resize/crop offset-blit `Document.source_planes` and update its dimensions;
  `transform_layer`/`transform_layer_quad`/`transform_layer_warp` resample every
  stored plane through the same plane map as the 8-bit channels (`-2` via the
  mask map) and derive each stored 8-bit channel from the resampled plane's
  `narrow_to_u8()` so the writer's equality gate holds; `Image > Image Size`
  resamples the layer and composite stores; rotation/flips remap them exactly
  (index permutations, no resampler) and swap `source_planes`' recorded
  dimensions. New `document_ops/native_store.rs` (offset blit / native resize /
  exact remap) and `layer_ops/transform_native.rs` (inverse-map bilinear /
  mesh warp) hold the `u8`/`u16`/`f32`-dispatched kernels; the `u8` arm delegates
  to the byte-identical 8-bit kernels, so an 8-bit document is unchanged.
  Ceilings: `crates/pictura-codec/src/write.rs` still requires the 8-bit working
  model (`doc.depth == Eight`) and the store is a retention overlay, not the
  edit representation; unmodeled `raw_channels` streams still drop on a
  resample; merge/flatten/via-copy/rasterize build 8-bit content with no store;
  a source-mode (Lab/CMYK) store is resampled but its working 8-bit channel is
  not derived from it; GPU paths unchanged.
- **Lab write-back** (roadmap P4/G2, archived `2026-09-23-color-mode-write-back`):
  an 8-bit Lab PSD no longer converts to RGB on save. `read_psd` retains the
  pre-normalization Lab color planes (composite and every layer color channel,
  recursing into groups) in the same store as `depth-preserve`; `write_psd`
  writes header mode Lab, re-emitting the retained planes byte-for-byte when
  unchanged (an 8-bit Lab re-encode would drift saturated colors up to ~19 LSB)
  and converting the working RGB with a profile-free `rgb_to_lab` inverse when
  edited (approximate, `// ponytail:`). RGB/Grayscale/constructed documents are
  unchanged; CMYK/Bitmap/Indexed and a 16/32-bit Lab source still save the
  working mode (the app notice reports Lab only for an 8-bit Lab source). The
  change also fixed a pre-existing read bug: the color-mode normalization now
  recurses into layer groups, so nested layers in a grouped Lab/CMYK/Bitmap/
  Indexed document convert correctly. Proven by the `color_mode_oracle`
  (lcms2/psd-tools, ran) and grouped/16-bit unit tests. Ceilings: CMYK/
  Bitmap/Indexed write-back (CMYK has no profile-free inverse; needs a
  color-management decision), Multichannel/Duotone, and an edited Lab plane's
  approximation.
- **CMYK write-back** (roadmap P4/G2, archived `2026-09-23-cmyk-write-back`): an
  8-bit CMYK PSD now saves back as CMYK (header mode 4, four color planes).
  `read_psd` retains the pre-normalization CMYK color planes (composite and every
  layer, recursing groups) in the same store; `write_psd` re-emits them
  byte-for-byte when unchanged, else converts the working RGB with `rgb_to_cmyk`,
  an **exact** right-inverse of `cmyk_to_rgb` (no-black `K=255`), so an edited
  pixel reads back to exactly the edited RGB. The 4-channel layout adds the extra
  color channel to the header count, the document-extra offset (`4 + i`), and the
  synthesized per-layer `(C,M,Y,K)` channels. Bitmap, Indexed, and a 16/32-bit
  CMYK source still save the working mode; the app notice reports a
  CMYK-preserving save only for an 8-bit source. Proven by the `color_mode_oracle`
  (mode 4, four channels, retained planes byte-identical; lcms2/psd-tools ran) and
  unit tests (edited plane, layer, extra channel). Ceilings: an edited CMYK pixel
  uses a fixed no-black convention, not color management; a 16/32-bit CMYK source
  still writes RGB (now untagged — see the ICC consistency bullet); Bitmap
  write-back and Multichannel/Duotone remain open.
- **Indexed write-back** (roadmap P4/G2/G3, archived
  `2026-09-23-indexed-write-back`): an **unchanged** 8-bit Indexed PSD now saves
  back as Indexed (header mode 2, one index channel, the 768-byte palette).
  `read_psd` retains the palette in a new `Document.source_palette` and each
  index plane (the composite's and every pixel layer's, recursing groups) in the
  existing source store, while `color_mode_data` stays cleared (the "palette is
  consumed" contract is unchanged). `write_psd` emits Indexed only when every
  retained plane still expands through the palette to the current working RGB;
  the header mode is document-wide, so an edited composite or layer, or an added
  color layer, falls back to writing the working RGB. No RGB-to-palette
  quantization is invented (a no-oracle path, like the Lab/CMYK edits). A file
  with no merged composite still writes Indexed (palette + layer index channels,
  no image-data section). The mode notice reports an Indexed-preserving save;
  ceiling: it does not consult the edit, so an edited Indexed document still
  claims "saved as Indexed". Proven by the `color_mode_oracle` (mode 2, palette
  and index planes byte-identical; psd-tools ran) and unit tests (grouped layer,
  raster mask, edited composite/layer fallback, no-merged-composite, added empty
  layer, malformed-input typed error). Ceiling: Bitmap write-back and
  Multichannel/Duotone remain open.
- **Bitmap write-back** (roadmap P4/G2, archived `2026-09-23-bitmap-write-back`):
  a **flat, unchanged** depth-1 Bitmap PSD now saves back as Bitmap (header mode
  0, depth 1, one 1-bit channel) at its source compression (Raw or RLE). `read`
  retains the raw packed composite plane (store depth `BitDepth::One`), and
  `write_psd` borrows it directly (bypassing the `width*height` widening path).
  `depth_of(One)` now yields 1 so the writer finds the packed plane; `depth_bits(1)`
  stays unmapped so a Bitmap read's `source_depth` is unchanged, and
  `output_depth` only honors a 16/32-bit source depth (an inconsistent hand-built
  depth-1 store is a typed error, so `write_psd` never panics). Scope is the flat
  case: an edited Bitmap, a layered Bitmap, or one with an extra channel writes
  the working RGB (a per-channel packed store plus a depth-1 layer path is the
  ceiling; no RGB→1-bit threshold is invented). The mode notice reports a
  Bitmap-preserving save; ceiling: the edit and the flat-only fallback are not
  consulted. Proven by the `color_mode_oracle` (Raw and RLE round trips, packed
  plane byte-identical, psd-tools mode 0) and unit tests (edited/layered/
  extra-channel fallback, typed-error guard). Ceiling: Multichannel/Duotone
  remain open (no natural RGB mapping / spot-ink spec).
- **ICC profile matches the output mode** (roadmap P4/P6, archived
  `2026-09-23-icc-output-mode-consistency`): a 16/32-bit CMYK or Lab source is
  normalized to RGB but keeps its CMYK/Lab resource `1039` (the profile cannot
  build an RGB transform, so both the Convert and Preserve read paths leave it).
  `write_psd` now drops a framable `1039` whose ICC data-space signature (header
  bytes 16..20) does not match the output header color mode (`RGB `/`GRAY`/`CMYK`/
  `Lab `), at the single resource-emit choke point, so the saved file is never
  mis-tagged. A matching profile, a profile too short to carry the signature,
  every other resource, and the unparsed tail re-emit byte-for-byte; the section
  is returned unchanged when nothing mismatches, so existing files write
  identically. The read-time `Preserve` policy is untouched. Tests cover a
  read→write 16-bit CMYK and Lab source (Convert and Preserve), Grayscale keeping
  its `GRAY` profile, and the CMYK/RGB cases. Ceiling: a `1039` the parser cannot
  frame (behind an unrecognized signature) is preserved, not filtered. The later
  `depth-color-mode-write-back` change makes a 16/32-bit CMYK/Lab save in its
  source mode, so that path's profile now matches and is kept; the guard remains
  the general invariant against any mode/profile mismatch.
- **16/32-bit CMYK/Lab write-back** (roadmap P4/G2/G4, archived
  `2026-09-23-depth-color-mode-write-back`): a 16/32-bit CMYK or Lab PSD now saves
  back in its source color mode **and** source depth; before, it saved 8-bit RGB
  (both lost). `read_psd` retains the native Lab/CMYK color planes (composite and
  every layer; the native layer store already existed) and `write_psd` re-emits an
  unchanged plane byte-identically, else re-encodes the working RGB with the
  profile-free 8-bit inverse and widens (Lab approximate, CMYK exact). The
  unchanged test runs in the narrowed-retained domain: two helpers
  (`composite_retained_8`/`layer_retained_8` in `color_mode.rs`, since `write.rs`
  is at the size cap) narrow the retained native plane before comparison, because
  comparing it to `rgb_to_lab`/`rgb_to_cmyk` fails (Lab ±1 LSB, `rgb_to_cmyk`
  forces `K=255`). A short retained plane is a typed error, not a panic. The app
  notice keeps the mode-preserving wording when the depth is retained. Proven by
  the `color_mode_oracle` (16- and 32-bit CMYK/Lab, psd-tools mode 4/9 at source
  depth) and `tests/depth.rs` (composite + layered Lab/CMYK byte-identity, edited
  widening, typed-error guard). Ceilings: editing stays 8-bit (the typed store
  ships — `bit-depth-sample-model` — but ops are not yet ported to native depth);
  a mode with no retained native store (8-bit Lab/CMYK) and
  Indexed/Bitmap save as before; Multichannel/Duotone remain open.
- `vmsk` vector masks (roadmap P3, archived `vector-mask-render`): now decode
  into a derived `Layer.vector_mask` view (raw block preserved and re-emitted)
  and clip the layer through `mask_alpha`, combined with the raster mask by
  multiplication and honoring the invert/disable flags. Fill rule is even-odd by
  default; non-zero is read only from the ag-psd marker. A committed
  `vector_mask.psd` fixture is proven by psd-tools + ag-psd oracles and GPU
  parity; new `vector.rs` in `pictura-core`, `vector_mask.rs` in
  `pictura-codec` and `pictura-render`. Ceilings: open paths / `vscg` / `vsms`
  deferred, non-union operations collapsed, no AA / Feather / Density, layer
  effects gated with content (Photoshop's "Vector Mask Hides Effects" off by
  default is a documented divergence), no resize transform, fixed 16-segment
  flattening, no Photoshop pixel parity.
- `vscg` vector fill content (roadmap P3, archived `vector-fill-content`): now
  decodes and renders as a generative fill clipped by the layer's vector mask (or
  layer bounds), reusing the shipped `SoCo`/`GdFl`/`PtFl` decoders (the fill
  descriptor's classID is `null`; `solidColorLayer`/`gradientFillLayer`/
  `patternFillLayer` are the stroke content, not the fill); the adjustment-block
  fill takes strict precedence (an undecodable block is a no-op regardless of
  `vscg`); a committed `vector_fill.psd` fixture is proven by psd-tools + ag-psd
  oracles; a channel-less vector layer falls back to CPU on the GPU. Ceilings:
  vector stroke (`vstk`) deferred, `vogk`/`vsms` deferred, noise gradients,
  boolean ops beyond union, no AA, no authoring/UI, rasterize stays
  adjustment-block-based.
- **16/32-bit depth read** (roadmap P4/G4, archived `depth-read`): `read_psd`
  now opens 16-bit and 32-bit Grayscale/RGB/CMYK/Lab documents and **normalizes
  them to 8-bit on load** — `source_depth` records the original
  (`Some(Sixteen)` / `Some(ThirtyTwo)`), the written document is uniformly
  8-bit (`write_psd` keeps writing 8-bit, a documented lossy-in-depth save), and
  the app shows a status-bar "Converted from 16-bit"/"…32-bit" notice. The
  narrowings match psd-tools: 16→8 is `sample >> 8`, 32→8 is
  `clamp(trunc(sample * 256), 0, 255)`; the row stride, RLE byte counts, and
  ZIP-with-prediction are depth-aware (16 a per-`u16` running sum mod 2^16; 32 a
  four-byte-plane unshuffle then byte-wise delta). **Every** channel is narrowed,
  including layer color/`-1`/`-2`, unmodeled spot/selection channels (re-wrapped
  as an 8-bit raw stream instead of re-emitting source-depth bytes), and
  document-level extra channels. Bitmap/Indexed at 16/32 stay unsupported. New
  flat fixtures `rgb16.psd`/`rgb32.psd` are proven by a psd-tools oracle (0
  tolerance); `scripts/generate-fixtures.py` is byte-stable and no existing
  fixture/golden changed. Ceilings (all `ponytail:`): no true 16-bit sample model
  (depth is not preserved), the 32-bit path is display-referred so HDR clips at
  1.0 with no tone map, and there is no write-side re-encode to the source depth.
- **Refine Edge / Select and Mask** (change `refine-edge`, issue #297, from the
  #252 UX pass; new capability `tools/refine-edge`, and `tools/select-menu`
  MODIFIED): `pictura-select::refine` refines a selection's coverage against the
  composite. `refine(mask, image, settings)` runs CS6's documented order —
  edge-band re-estimation (a Chamfer boundary band, `smart_radius` narrowing
  where the image edge is hard), then Smooth (reused majority filter), Feather
  (reused Gaussian), Contrast (a mid-toned S-curve that pushes partial coverage
  toward 0/255), and Shift Edge (a signed-distance re-threshold, negative
  inward) — with a bit-identical identity at defaults and `InvalidParams` /
  `SizeMismatch` guards. `decontaminate(image, alpha, amount)` returns a new
  buffer, lerping soft-edge colour toward nearby fully-opaque foreground by
  `(1 − α)·amount`, leaving α 0/255 pixels untouched. The app bridge
  (`cxxqt_object/impl_selection/refine.rs`) exposes available / preview / apply;
  `RefineEdgeDialog` (`refine_edge_dialog.{h,cpp}`) is the CS6 layout (View Mode,
  Show Radius / Original, Smart Radius + Radius, Smooth / Feather / Contrast /
  Shift Edge, Decontaminate + Amount, Output To) with a live proxy preview.
  `Select ▸ Refine Edge…` (`Ctrl+Alt+R`) and the selection tools' `Select and
  Mask…` options-bar button both open it; OK records one "Refine Edge" state,
  Output To maps to the selection, the active layer's mask, or `layer_via_copy`
  (New Layer / New Layer with Layer Mask), and Decontaminate forces a colour
  output (the in-place targets are disabled). Tested by `tst_refine_edge` (7
  cases) and the engine's property tests. Ceilings (`ponytail:`): no interactive
  Refine Radius / Erase Refinements brush strokes or their local band override,
  no view-mode keyboard shortcuts (F/X/P/J), no Properties-panel mask entry, and
  the surface/edge model is a behavioural approximation of a closed algorithm.
- **Canvas right-click context menus** (#306, change
  `canvas-right-click-context-menus`, archived into `ui/canvas-context-menus`):
  `ImageView::contextMenuRequested` routes a right-click to the active tool's
  `ToolHandler::onContextMenu`, which may also fire over the pasteboard (the
  Zoom menu); with no tool menu the pasteboard falls back to the
  canvas-background colour menu. Move lists the layers under the pointer (Auto
  Select off) plus Isolation Mode; Eyedropper offers Sample Size/Sample; Crop
  offers Delete Cropped Pixels/Reset Crop only while an active box exists; the
  paint tools open the non-grabbing Brush Preset picker (an in-window overlay,
  not `Qt::Popup`, so the brush ring keeps tracking and the first press paints).
  Tested by `tst_canvas_context_menus` (21 cases, Qt Test). Menus build per
  right-click and delete on close; `BrushToolHandler` finalises an open stroke on
  deactivate.

## Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace          # preferred; cargo test --workspace is the fallback
cargo test --workspace --doc           # doctests (nextest does not run them)
cmake -S . -B build -G Ninja -DCMAKE_EXE_LINKER_FLAGS=-fuse-ld=lld && cmake --build build --parallel
./build/pictura --headless --self-test
./build/pictura --headless --self-test crates/pictura-codec/tests/fixtures/two_layers.psd
bash scripts/guard.sh
openspec validate --all --strict
```

## Code health (LOC guardrail)

Every source file is under the **LOC hard cap** (code 1200, tests 1400; target
<800; `AGENTS.md` rule). `scripts/check-file-size.sh` is the guard, `scripts/file-size-allowlist.txt`
the exception list, and `scripts/verify-fast.sh` runs it. A completed
code-splitting pass (`docs/dev/refactor-code-splitting.md`) took the sixteen
over-cap files down to one by pure moves — build, byte-identical self-test
stderr, the Rust suite, and the specs green throughout:

- **C++ app TUs** — `panels/panel_group.cpp` → `panel_group{,_menu,_test}.cpp`;
  `panels/layers_panel.cpp` → `layers_panel{,_actions,_menu,_test}.cpp` +
  `layers_panel_internal.h`; `panels/panel_column.cpp` →
  `panel_column{,_drag,_iconic,_menu,_test}.cpp` + `panel_float.cpp` +
  `panel_column_internal.h`; `frame.cpp` →
  `frame{,_columns,_session,_menus,_build,_test}.cpp`. New TUs are registered in
  `CMakeLists.txt`.
- **Self-test** — the `--self-test` block moved out of `main.cpp` into
  `selftest.cpp` / `selftest.h` (`int runSelfTest(QApplication&, bool, const
  QString&, PicturaMainWindow&, PictureView*, const QImage&, bool, int)`).
  `main.cpp` is now the startup path only (7320 → 180 LOC) that calls
  `runSelfTest`; the self-test lines live in `selftest.cpp`.
- **cxx-qt bridge** — `cxxqt_object.rs` is a Rust-2018 root that keeps the
  cxx-qt bridge and shared private helpers, with concern submodules under
  `src/cxxqt_object/`: `impl_{core,layers,selection,transform,paint,history,filters}.rs`,
  `helpers.rs`, `helpers_composite.rs`, `tests.rs`, `tests_impl.rs`. The root +
  directory layout keeps the generated-header path and the 14 C++ includes
  untouched.
- **Engine crates** — `pictura-filters` (`artistic/filters.rs` →
  `artistic/filters/{effects,brush,common,tests}.rs`; `lib.rs` → `filter.rs`),
  `pictura-render` (`lib.rs` → `composite.rs` + `tests/`; `gpu.rs` →
  `gpu/{mod,backend,shader}.rs`; `gpu_filter.rs` →
  `gpu_filter/{mod,plan,resources}.rs`; `document_ops/layer_ops.rs` →
  `document_ops/layer_ops/{mod,create,paths,properties,tests}.rs`),
  `pictura-adjust` (`lib.rs` → `types/common/apply/tonal/color/auto/tests.rs`),
  `pictura-codec` (`lib.rs` → `error/common/read/write/tests.rs`), and the
  integration tests (`tests/oracle.rs` → `tests/oracle/`;
  `tests/gpu_parity.rs` → `tests/gpu_parity/`).

One file remains allowlisted: `crates/pictura-app/cpp/selftest.cpp` (7212 LOC).
It was extracted whole first to protect the verification oracle; subdividing it
by self-test section is a deliberate later step, out of this pass.

`crates/pictura-codec/tests/oracle.rs` is at **1398 LOC**, just under the 1400
test-LOC cap, and must be split (the `tests/oracle/` submodule directory
already exists) before any further oracle growth.

## Crates

| Crate | Responsibility |
|---|---|
| `pictura-core` | Document/Layer/Channel/Mask/BlendMode(27+pass)/AdjustmentData; no deps |
| `pictura-codec` | PSD/PSB read/write: composite + layer channels raw/RLE/ZIP/ZIP-prediction, layers, masks, adjustment keys, document channels; unknown blend key degrades to Normal; absent composite tolerated |
| `pictura-color` | ICC profiles (sRGB/AdobeRGB/ProPhoto), convert/assign, intents, BPC |
| `pictura-adjust` | 15 destructive adjustments (`apply`) |
| `pictura-filters` | blur/sharpen/noise + stylize/other + pixelate + distort + render filters (`Filter` + `apply`); seeded filters; `artistic` module (15 CS6 Artistic filters with shared `reduce`/`noise`/`texture` helpers) + the four remaining families — Brush Strokes, Sketch, Texture, Oil Paint (29 filters, same shared helpers) |
| `pictura-select` | selection coverage mask, boolean/modify ops, wand, color range; `Selection::{rect,ellipse,polygon}` rasterizers + `CombineMode`/`combine_with` |
| `pictura-ops` | image resize (Nearest/Bilinear/Bicubic), canvas size (9 anchors), rotate/flip + arbitrary rotation; ImageMagick oracle |
| `pictura-render` | CPU compositor (27 blend modes, groups, masks, adjustment layers) + GPU compositor (default backend: 26 GPU modes + 5 adjustment layers, `Backend`/`composite_active`/`composite_region_active` (dirty-rect composite, byte-identical to the sub-rect); GPU-native 8-bit data path; 2-D compute dispatch so large documents like 4000² composite on the GPU instead of falling back to the CPU above the old ~4.19 MP 1-D workgroup ceiling) + GPU filter path (`filter_gpu_available`/`apply_filter_active`, byte-exact CPU-parity kernels — the nine convolution kernels plus the M28 heavy window/effect kernels Surface Blur, Maximum, Minimum, Median, Custom 5×5, Oil Paint; same 2-D dispatch for >4.19 MP) + PSD adjustment encode/decode + `apply_filter` (layer filter gated by mask, GPU-accelerated when `gpu_enabled`) + `document_ops` (document resize/canvas/orientation/crop/layer-translate; `translate_layer_active` composites through the active backend, `translate_layer_rect` shifts a layer rect (and mask) without recompute, CPU `translate_layer`/`recompute` remain the oracle; re-exports `Anchor`/`Resample`) |
| `pictura-testkit` | golden compare/hash + `pictura-diff` CLI |
| `pictura-paint` | dab-splatting brush/pencil stroke engine — tip coverage, spacing, flow/opacity, paint modes; depends on `pictura-core` |
| `pictura-app` | cxx-qt `PictureView` QObject + Qt C++ shell: `commands` (command registry + full documented CS6 menu tree), `frame` (`PicturaMainWindow`: menu bar, tabbed document area with a `PictureView`+`ImageView` per document, file lifecycle New/Open/Save/Save As/Revert/Close/Close All/Exit, status bar, docks, screen modes), `theme` (Fusion dark palette, 4 brightness levels), `session` (XDG state store), real Layers/History/Navigator/Color/Swatches/Info/Histogram panel docks replacing the debug dock (backed by the document and history models, with a frame-owned `ColorState` fed by the Eyedropper), tool layer (`tools`/`toolbox`/`options_bar` — Move/Marquee/Lasso/Quick Selection/Crop/Eyedropper/Hand/Zoom/Brush/Pencil), paint bridge (`begin_paint`/`paint_dab`/`end_paint`/`cancel_paint`/`is_painting`) with a live paint options bar, zoom/pan, GPU demo |

## Milestones done

- **M0/M0.5** — cxx-qt↔Qt6 walking skeleton, PSD codec, harness, CI, GPU spike.
  GPU: offscreen wgpu→readback→QImage works; QRhi imports the wgpu VkDevice/image;
  on-screen present via `QRhiWidget` is blocked (`crates/pictura-app/GPU-INTEROP-NOTES.md`).
- **M1** — layer/channel/mask model + layered PSD round-trip (+psd-tools oracle).
- **M2/M2.5** — CPU compositor (27 modes) + ImageMagick oracle; GPU compositor
  (22 modes, Δ0); app renders the layer stack.
- **M3** — ICC color management (+ ImageMagick oracle).
- **M4 (A/B/C)** — 15 adjustments; adjustment layers (decoded: `nvrt`,`post`,
  `thrs`,`brit`,`levl`,`hue2`; others preserved-only); app adjustment UI.
- **M5 (A/B/C)** — selection math (+ IM morphology oracle); selection↔PSD
  channels; selection-masked adjustments in the app.
- **M6** — `pictura-filters`: blur (Gaussian/Box/Motion/Radial/Average/
  Blur(+More)/Surface), sharpen (Sharpen(+More)/Edges/Unsharp Mask), noise
  (Add Noise seeded/Median/Despeckle). ImageMagick oracle: Gaussian/Box/Median
  exact, USM ±6; Motion classified no-equivalent (IM kernel is one-sided). 47
  filter tests. OpenSpec change `m6-filters`, tasks checked.
- **M6-C** — filter integration: `pictura-render::apply_filter(layer, filter,
  mask)` (destructive, selection/mask-confined, alpha + layer meta preserved) and
  the app `apply_filter(kind)` command + dock control. Headless self-test proves
  confinement (`changed_inside=12 changed_outside=0`). OpenSpec change
  `m6-filter-integration`, tasks checked.
- **M7** — Stylize + Other filters: `Maximum`, `Minimum`, `Offset`, `High Pass`,
  `Custom` (Other); `Emboss`, `Find Edges`, `Solarize` (Stylize). ImageMagick
  oracle exact (Δ0) for Maximum/Minimum/Offset-wrap/Custom/Solarize; Emboss,
  Find Edges, High Pass, Offset-fill classified no-equivalent. App filter kinds
  added. OpenSpec change `m7-stylize-other`, tasks checked.
- **M8** — Pixelate filters: `Mosaic`, `Crystallize`, `Facet`, `Fragment`,
  `Mezzotint`, `Pointillize`, `Color Halftone`. Oracle: Mosaic exact (block
  average when the cell divides the dimensions); the other six no-equivalent
  with recorded deltas; seeded filters reproducible. App filter kinds added.
  OpenSpec change `m8-pixelate`, tasks checked.
- **M9** — Distort warps: `Twirl`, `Pinch`, `Spherize`, `Ripple`, `Wave`
  (single-image inverse-mapping with bilinear resampling; alpha untouched; Wave
  seeded, `repeat_edge` selectable). All five classified no-equivalent against
  the closest ImageMagick operator (Twirl closest at Δ124 vs `-swirl 45`);
  measured deltas recorded. App filter kinds added. OpenSpec change
  `m9-distort`, tasks checked.
- **M10** — `pictura-ops` image operations: `resize` (Nearest/Bilinear/Bicubic),
  `resize_canvas` (9 anchors, grow/shrink), orientation (`rotate90_cw/ccw`,
  `rotate180`, flips, `rotate_arbitrary`). ImageMagick oracle: Nearest and the
  exact right-angle rotations/flips and 3-channel canvas are exact (Δ0); Bicubic
  uses the publicly documented Photoshop `cubic(0, 0.75)` kernel and matches
  `-filter cubic -define filter:b=0 -define filter:c=0.75` (tol 1); Bilinear is
  exact on upscale but no-equivalent downscale; `rotate_arbitrary` measured on the central
  region (max 8 at 30°, 45° no-equivalent). OpenSpec change `m10-image-ops`,
  tasks checked; app/document integration deferred.
- **M11** — Distort filters, part 2: `PolarCoordinates`, `Shear`, `ZigZag`,
  `OceanRipple` (inverse-mapping warps with bilinear resampling; alpha
  untouched; Shear `fill` selectable, Ocean Ripple seeded). All nine Distort
  filters are now implemented, and all four new ones are classified
  **no-equivalent** against their closest ImageMagick operators with measured
  deltas (Polar Δ189/58 and 194/59 vs `-distort Polar`/`DePolar`; Shear Δ255/18
  vs `-shear`; ZigZag Δ170/14 vs `-swirl`; Ocean Ripple Δ227/54 vs `-wave`).
  Shear's curve range check (`x`/`y` in `-1..=1`) added. App filter kinds added.
  OpenSpec change `m11-distort2` MODIFIED the canonical `distort-filters` spec
  (7 MODIFIED + 4 ADDED); tasks checked.
- **M12** — Document operations: document-level resize/canvas/rotate/flip in
  `pictura-render`; reuses `pictura-ops`; structural oracle via psd-tools +
  composite consistency + exactness identities. OpenSpec change
  `m12-document-ops`, archived.
- **M13** — Image ops app UI: `PictureView` commands `resize_image` /
  `resize_canvas` / `rotate_doc` / `flip_doc` (validate → doc op → clear
  selection → recomposite), dock "Image" section (spin boxes + resample and
  9-anchor combos + orientation buttons), and self-test doc-op checks
  (exact remap + rejection + selection-clear assertions, exit codes 19–21).
  `pictura-render` re-exports `Anchor`/`Resample`. OpenSpec change
  `m13-image-ops-ui` (capability `image-ops-app-ui`), archived.
- **M14** — Undo/Redo: snapshot history in `crates/pictura-app/src/history.rs`
  (two-stack, depth 20 = CS6 default, doc+selection clones); capture wired
  into all eleven mutating commands (pre-state clone on success only);
  `open()` resets. `PictureView::undo`/`redo`/`can_undo`/`can_redo`/
  `history_depth`; dock buttons + Ctrl+Z/Ctrl+Y; self-test proves bit-exact
  undo/redo, redo invalidation, open reset (exit codes 22/23).
  OpenSpec change `m14-undo-history` (capability `edit-history`), archived.
- **M15** — Render filters: `Clouds`, `DifferenceClouds`, `Fibers`,
  `LensFlare` in `pictura-filters/src/render.rs` (seeded lattice value noise,
  Difference blend, x-elongated fibers, additive lens flare with
  `LensType`); all four classified no-equivalent (closed Adobe models) and
  verified by 13 property tests. App kinds + combo + self-test confinement
  check (exit 24). Lighting Effects and Scripted Patterns remain future.
  OpenSpec change `m15-render-filters` (capability `render-filters`),
  archived.
- **M16** — App shell foundation: replaced the M0 debug window with a
  CS6-shaped frame. New `pictura-app` C++ units: `commands` (declarative
  registry: stable id, path, label, shortcut, enablement, dispatch),
  `command_tree` (full documented CS6 tree — 523 documented leaves plus 21
  implemented commands and 47 separators; unimplemented leaves disabled), `frame`
  (`PicturaMainWindow`: menu bar, central canvas, status bar with view-options
  popup, dock registration with duplicate `objectName` rejection, screen modes
  `F`/`Shift+F`, canvas colour `Space+F`, `Tab`/`Shift+Tab` hide-all),
  `theme` (Fusion + dark palette, four brightness levels, `Shift+F1`/`F2`),
  `session` (atomic `QSaveFile` XDG state, schema-versioned). Bridge gains
  `has_document()` for enablement. `main.cpp` shrinks to startup + the self-test
  call (`selftest.cpp`);
  new self-test checks (exit codes 25–32) cover menu order, dispatch/inertness,
  no-document enablement, brightness, screen-mode cycle, session round-trip,
  duplicate panel rejection, and hide-all. OpenSpec change `m16-app-shell`
  (capabilities `command-registry`, `workspace-persistence`; MODIFIED
  `application-shell`), archived.
- **M17** — Document lifecycle and multi-document tabs. Rust bridge
  `PictureView` gains `new_document(width,height,mode,depth,background)`
  (8-bit Grayscale/RGB, white/transparent only), `save(path)` (atomic
  temp+rename through `write_psd`), `is_dirty()`, `file_path()`, plus
  `path`/`dirty` state; dirty is set at all 11 mutating-command
  history-capture sites and cleared by open/save. C++ shell: `new_document_dialog`
  (New Document dialog), `dialogs` (`askUnsaved` Save/Discard/Cancel with a
  non-interactive test policy), `frame` rewritten around a `QTabWidget` document
  area (one `PictureView`+`ImageView` per document, active-document targeting
  for all menu handlers/dock/status, tab titles with a `*` modified marker,
  File New/Open/Save/Save As/Revert/Close/Close All/Exit handlers, recent-files
  list persisted in the session store), `session` gains a bounded `recent`
  list, and `commands.h`/`command_tree.cpp` file-lifecycle leaves become
  implemented. `main.cpp` starts empty, opens the CLI path into a document, or
  creates a scratch document for the GPU smoke test; self-test exit codes 33–37
  cover New + white fill, Save As/open pixel round-trip, dirty set/cleared, tab
  switching, and close prompt Cancel/Discard. Verified: `cargo` suite clean
  (401 tests, 0 failed, 1 ignored — unchanged), fixture and no-argument
  self-tests exit 0, `guard.sh` OK. OpenSpec change `m17-document-lifecycle`
  (capabilities `document-lifecycle`, `document-tabs`; MODIFIED
  `application-shell`), archived.
  **Bug fixed (document tab reorder):** the document `QTabWidget` was movable
  but nothing connected `QTabBar::tabMoved`, so dragging a tab left `docs_` in
  the old order while `tabs_->currentIndex()`, `viewAt` and `removeDocument`
  indexed the new one — the wrong document became active, closed, or returned.
  `frame.cpp` now connects `tabMoved(from,to)` to `docs_.move(from,to)` (guarded
  by valid indices); Qt keeps the dragged-to-current tab current, so the active
  document is unchanged. Regression: self-test exit **196**
  `doc_tab_reorder aligned=1` (`PicturaMainWindow::reorderDocumentsForTest`),
  which also exercises `viewAt`/`documentName`/`activeDocumentIndex` after the
  move.
- **M18** — Toolbox and core tools. Engine: `pictura-select` gains
  `Selection::{rect,ellipse,polygon}` coverage rasterizers and
  `CombineMode {New,Add,Subtract,Intersect}` + `combine_with`; `pictura-render`
  document ops gain `crop_document` (reuses the M12 canvas offset; clamps and
  shifts layers/masks/channels) and `translate_layer` (shifts the topmost pixel
  layer's rect). Rust bridge: `select_rect`/`select_ellipse`/`begin_lasso`/
  `lasso_add_point`/`end_lasso`/`quick_select`/`crop`/`translate_layer`/
  `sample_argb`/`selection_bounds`, with dirty/history capture on the new
  mutating ops. C++ shell: `image_view.{h,cpp}` gains pointer signals
  (`mousePressed/moved/released` in image coordinates), `setPanEnabled`, and an
  overlay polygon; new `tools.{h,cpp}` (`ToolId {Move,Marquee,Lasso,
  QuickSelection,Crop,Eyedropper,Hand,Zoom}`, `SelectionMode`, `ToolController`),
  `toolbox.{h,cpp}` (Tools dock, letter shortcuts), `options_bar.{h,cpp}`
  (context-sensitive options bar); `frame.{h,cpp}` hosts them, routes canvas
  events, exposes `activeTool/setActiveTool/foregroundColor/hasPendingCrop/
  commitCrop`. Commands `image.crop`, `view.options`, `window.panels.tools`.
  `selftest.cpp` self-test (exit codes 39–46): tool switching, marquee rect (16 px)
  + ellipse (12 px), combine modes (16/28/12/4), lasso (25 px) + short-lasso
  rejection, quick selection, crop remap, layer move, eyedropper sample
  (`ffff0000`/`00000000`). Verified: build green, fixture and no-arg self-tests
  exit 0, `cargo fmt/clippy` clean, 411 tests (0 failed, 1 ignored; +10 engine
  tests), `openspec validate --all --strict` 40/40 pre-archive (42 after),
  `guard.sh` OK. OpenSpec change `m18-toolbox-tools` (capabilities
  `tool-framework`, `shape-selection-tools`, `canvas-tools`), archived.
- **M19** — SVG icon set and cursors. New `assets/icons/` (40 original
  original SVGs: `app`, eight `tool.*`, and 31 implemented-command icons named
  by command id, e.g. `file.saveAs.svg`, `view.screenMode.full.svg`) and
  `assets/cursors/` (eight `tool.*` SVG cursors; eye-dropper hotspot (2,22),
  others (12,12)); `assets/pictura.qrc` bundles all 48. `icons.{h,cpp}` provides
  `QIcon pictura::icon(id)` and `QCursor pictura::cursor(id)` (QSvgRenderer,
  DPR-scaled render, existence-guarded so unknown ids are silent). Build gains
  `Qt6::Svg`, `CMAKE_AUTORCC`, the `.qrc`, and the `Qt6::Svg` link. Wiring:
  application/window icon, Tools-panel action icons, the 31 implemented
  menu-action icons, and the active tool's SVG cursor. `selftest.cpp` self-test exit
  codes 47–49: all 40 icons resolve + unknown is null, all 8 cursors resolve,
  window icon set. Verified: build green, fixture and no-arg self-tests exit 0
  with no Qt warnings, `cargo fmt/clippy` clean, 411 tests (0 failed, 1 ignored;
  unchanged), `openspec validate --all --strict` 43/43 pre-archive (44 after),
  `guard.sh` OK. OpenSpec change `m19-svg-icons` (capabilities `icon-assets`,
  `svg-cursors`), archived.
- **M20** — Panel parity. `history.rs` becomes a labeled linear model (one
  labeled state per undoable step, depth 20, plus up to 10 named snapshots);
  `edit-history` undo/redo semantics are unchanged. Bridge gains
  `layer_blend`/`set_layer_blend`, `layer_opacity`/`set_layer_opacity`,
  `set_layer_name`, `move_layer`, and `layer_thumbnail`. Seven `QDockWidget`
  panels in `crates/pictura-app/cpp/panels/` (`layers_panel`, `history_panel`,
  `navigator_panel`, `color_panel` + `ColorState`, `swatches_panel`,
  `info_panel`, `histogram_panel`; objectNames `layersPanel`, `historyPanel`,
  `navigatorPanel`, `colorPanel`, `swatchesPanel`, `infoPanel`,
  `histogramPanel`). `frame` registers the docks (replacing the debug dock),
  rebinds them on active-document change, owns a `ColorState` fed by the
  Eyedropper (`ToolController::foregroundSampled`), routes
  `ImageView::mouseMoved` into the Info panel, and implements checkable
  `Window > Panels` toggles for Navigator/History/Color/Swatches/Info/Histogram
  in addition to Layers/Tools. `CMakeLists.txt` gains the panel sources.
  `selftest.cpp` self-test exit codes 50–52 (`m20_layer count=2 name=1 blend=1
  badblend=1 opacity=128 dirty=1`; `m20_history states=5 open_label=1 grew=1
  jump=1 snapshot=1`; `m20_panels registered=7 toggled=7`), and the
  headless-shutdown hang is fixed by disabling the interactive
  unsaved-document prompt before the self-test quit timer. Deferred non-goals:
  group-tree expansion, drag-reorder, layer lock flags, clipping/link/color
  labels, filter/search row, swatch library file I/O, Info color samplers,
  Histogram source/cache states, and history branching beyond the bounded
  stack. Verified: build green, fixture and no-arg self-tests exit 0, `cargo
  fmt/clippy` clean, 414 tests (0 failed, 1 ignored), `openspec validate --all
  --strict` 45/45 pre-archive. OpenSpec change m20-panels (capabilities
  layers-panel, history-panel, navigator-panel, color-swatches-panel,
  info-histogram-panel), archived.
- **M21** — Paint engine. New `pictura-paint` crate (depends on `pictura-core`
  only): dab-splatting stroke engine with a procedural round/elliptical tip
  (size 1..5000, hardness, roundness, angle, flip), anti-aliased Brush vs
  aliased Pencil, fixed and velocity-driven spacing with residue carry-over,
  per-pixel coverage accumulation with a flow/opacity model
  (`acc = 1-(1-acc)(1-flow*tip)`, composited alpha `= opacity*acc`), and paint
  modes Normal/Dissolve/Behind/Clear; 25 in-crate tests. Bridge `PictureView`
  gains `begin_paint`/`paint_dab`/`end_paint`/`cancel_paint`/`is_painting`, a
  pre-stroke base plus working document, live image refresh per dab, and one
  labelled history state ("Brush"/"Pencil") per completed stroke. Qt:
  `ToolId::Brush`/`Pencil` (values 8/9), `B`/`Shift+B` cycling, size and hardness
  shortcuts (`[`/`]`, `Shift+[`/`Shift+]`), a paint options bar (size, hardness,
  opacity, flow, mode, Pencil Auto Erase), and `ToolController` stroke routing.
  Bug fixed: `PictureView::new_document` now creates one raster layer (opaque for
  "white", transparent for "transparent"), matching
  `docs/10-workflow-io/open-and-new.md`; previously a fresh document had no layer
  and could not be painted. `selftest.cpp` self-test exit codes 53–56
  (`m21_stroke ended=1 painted=196 dirty=1 hist=2`; `m21_opacity a1=84 a2=140`;
  `m21_aliased pencil_ok=1 brush_aa=1`; `m21_undo changed=1 restored=1`), with
  identical output on fixture and no-argument runs. Deferred non-goals:
  sampled/bristle/erodible/airbrush tips, Shape Dynamics, Scattering, Texture,
  Dual Brush, Color Dynamics, Transfer, Brush Pose, airbrush time build-up,
  tablet pressure mapping, `.abr` presets, Brush/Brush Presets panels, HUD, the
  remaining 23 paint modes, 16/32-bit and non-RGB painting, lock transparency,
  and sparse tile scratch storage. Verified: `cmake --build build` OK, both
  self-tests exit 0 with no FAILs, `cargo fmt/clippy` clean, 439 tests (0 failed,
  1 ignored; +25 `pictura-paint` tests), `openspec validate --all --strict`
  50/50 pre-archive. OpenSpec change m21-paint-engine (capabilities paint-engine,
  brush-tools; MODIFIED tool-framework), archived.
- **M22** — Artistic filters. New `pictura-filters::artistic` module implements
  the 15 CS6 Artistic filters — Colored Pencil, Cutout, Dry Brush, Film Grain,
  Fresco, Neon Glow, Paint Daubs, Palette Knife, Plastic Wrap, Poster Edges,
  Rough Pastels, Smudge Stick, Sponge, Underpainting, Watercolor — as
  behavioural-parity models (Adobe's kernels are closed), each carrying a
  `// ponytail:` ceiling note. Shared helpers: `artistic/reduce.rs`
  (`posterize`, `edge_magnitude`), `artistic/noise.rs` (seeded value noise via
  `ChaCha8Rng`), and `artistic/texture.rs` (`TextureSurface`
  Brick/Burlap/Canvas/Sandstone, `emboss`, `TextureOptions { surface, scaling,
  relief, light_direction, invert }`). All 15 are `Filter` variants with typed
  parameters, in-range validation, alpha preservation, and seeded determinism for
  the stochastic ones; new enums `BrushType`
  (Simple/LightRough/DarkRough/WideSharp/WideBlurry/Sparkle) and
  `TextureSurface`. App `filter_from_kind` maps the 15 kebab-case kinds with fixed
  in-range defaults and `seed: 1`. Self-test exit codes 57–58
  (`m22_applied applied=15/15`; `m22_deterministic=1`), measured on fixture and
  no-arg runs. Deferred non-goals: Filter Gallery dialog and cumulative stack,
  Smart Filter entries, `Edit > Fade`, 16/32-bit and CMYK/Lab gating, `Load
  Texture` file I/O, and the remaining families (Brush Strokes, Sketch, Texture,
  Oil Paint). Verified: `cmake --build build` OK; both self-tests exit 0 with no
  FAILs; `cargo fmt --all --check` OK; `cargo clippy --workspace --all-targets --
  -D warnings` OK; 465 tests (0 failed, 1 ignored; +26 filter tests); `openspec
  validate --all --strict` 52/52 pre-archive. OpenSpec change m22-artistic-filters
  (capability artistic-filters), archived.
- **M23** — CS6 UI chrome. `theme.{h,cpp}` gains `Theme::styleSheet(int level)`,
  a CS6-style QSS built from the four-level dark ramp (menu bar, options bar,
  dock tabs/title bars, tool buttons, status bar, scrollbars, menus, tooltips,
  panel content, push buttons); `Theme::apply` now sets the palette then the
  stylesheet. `toolbox.{h,cpp}` is rebuilt as a two-column icon-button grid
  (10 tools) replacing the single-column toolbar, plus a
  `ForegroundBackgroundWidget` (overlapping fg/bg swatches, active target,
  reset) and a screen-mode button emitting `screenModeRequested()`. `ColorState`
  gains an active target (`foregroundActive()`/`setForegroundActive()`/
  `activeChanged`) shared by the Color panel and the toolbox control.
  `frame.cpp` tabifies the default docks into CS6 groups — Color+Swatches,
  Layers+History, Navigator+Info+Histogram — and sets the canvas default colour
  to the CS6 dark grey `QColor(37,37,37)` (the four-entry cycle is unchanged).
  Reference screenshot: `docs/02-ui-ux/reference/cs6-workspace.png`. Self-test
  hygiene: an isolated `XDG_STATE_HOME` (`QTemporaryDir`) is set before the frame
  is constructed, so a user-saved layout cannot affect the checks. Self-test
  exit codes 59–61, measured identically on fixture and no-argument runs:
  `m23_stylesheet applied=1 levels=4 distinct=1`;
  `m23_toolbox dock=1 buttons=11 fgbg=1`; `m23_groups grouped=4/4`. Deferred
  non-goals: pixel-exact CS6 metrics and icon art, HUD/on-image displays, new
  panels (Gradients/Patterns/Properties/Adjustments/Libraries/Channels/Paths/
  Brush), workspace presets/switcher, icon-collapse docks, floating-panel drop
  zones, and deeper Layers-panel internals beyond M20. Verified:
  `cmake --build build` OK; both self-tests exit 0 with no FAILs; `cargo fmt
  --all --check` OK; `cargo clippy --workspace --all-targets -- -D warnings`
  OK; 465 tests (0 failed, 1 ignored; unchanged — no Rust changes); `openspec
  validate --all --strict` 53/53 pre-archive. OpenSpec change
  m23-cs6-ui-chrome (MODIFIED application-shell, tool-framework), archived.
- **M24** — Panel rail and right-side placeholders. `panels/placeholder_panel.{h,cpp}`
  adds one shared `pictura::PlaceholderPanel(title, message, parent)` dock with a
  centred CS6 empty-state label; `panels/panel_rail.{h,cpp}` adds
  `pictura::PanelRail`, a vertical icon-only `QToolBar` (objectName `panelRail`).
  `frame` creates the eight new placeholder docks — objectNames `gradientsPanel`,
  `patternsPanel`, `propertiesPanel`, `adjustmentsPanel`, `librariesPanel`,
  `channelsPanel`, `pathsPanel`, `actionsPanel` (Properties shows "No Properties"),
  structural only, no real functionality yet — and tabifies the right docks into
  the three CS6 groups Color+Swatches+Gradients+Patterns,
  Properties+Adjustments+Libraries, and Layers+Channels+Paths (tabify + raise the
  first). The rail carries five glyph buttons for History, Actions, Info,
  Navigator, Histogram; each button and its matching `Window > Panels` command
  share one toggle path, and the button's checked state tracks the dock's
  visibility. `commands.h`/`command_tree.cpp` gain ids and implemented, checkable
  `Window > Panels` entries for the eight new panels; `CMakeLists.txt` gains the
  two new sources. Self-test exit codes 61 (grouping, revised), 62 (panels), and
  63 (rail), measured identically on fixture and no-argument runs:
  `m24_groups grouped=8/8`; `m24_panels found=8/8 properties_empty=1`;
  `m24_rail actions=5 toggled=1 synced=1`. Deferred non-goals: real content for the
  placeholder panels (gradient/pattern presets, Properties binding, adjustment
  presets, libraries, channel/path lists, actions), icon-collapse auto-collapse,
  floating-panel drop zones, workspace presets/switcher, and panel-title-bar menus.
  Verified: `cmake --build build` OK; both self-tests exit 0 with no FAILs;
  `cargo fmt --all --check` OK; `cargo clippy --workspace --all-targets --
  -D warnings` OK; 465 tests (0 failed, 1 ignored; unchanged — no Rust changes);
  `openspec validate --all --strict` 53/53 pre-archive. OpenSpec change
  m24-panel-rail (capability panel-rail; MODIFIED application-shell), archived.
- **M25** — Remaining filter families. `pictura-filters` gains the four remaining
  CS6 families, 29 filters total: **Brush Strokes** (`brush_strokes.rs`: Accented
  Edges, Angled Strokes, Crosshatch, Dark Strokes, Ink Outlines, Spatter, Sprayed
  Strokes, Sumi-e), **Sketch** (`sketch/{mod,relief,paper}.rs`: Bas Relief, Chalk
  & Charcoal, Charcoal, Chrome, Conté Crayon, Graphic Pen, Halftone Pattern, Note
  Paper, Photocopy, Plaster, Reticulation, Stamp, Torn Edges, Water Paper),
  **Texture** (`texture.rs`: Craquelure, Grain, Mosaic Tiles, Patchwork, Stained
  Glass, Texturizer), and **Oil Paint** (`oil_paint.rs`). All are
  behavioural-parity models (Adobe kernels closed), reusing the M22 shared helpers
  `artistic::{reduce,noise,texture}` (posterize, edge_magnitude, clamp_u8,
  value_noise, surface_height, emboss, `TextureOptions`). New `Filter` enums
  `StrokeDirection`, `LightDirection`, `HalftoneType`, `GrainType` and 29 variants
  in `lib.rs`; `apply` dispatches to the family functions. Colour-dependent Sketch
  filters carry explicit `foreground`/`background` RGB (Bas Relief included —
  dark/recessed→foreground, light/raised→background); Conté Crayon and Texturizer
  take shared `TextureOptions`; seeded filters carry `seed: u64` and are
  bit-reproducible. **Oil Paint is a deliberate non-parity divergence:** CS6
  hard-requires a supported GPU (closed OpenCL kernel, no CPU fallback), so this
  is a CPU behavioural model (gradient-orientation directional edge-stopping
  smoothing + a luma/scale/bristle height field shaded Lambert/Blinn-Phong from
  `angular_direction`/`shine`); it is deterministic (no seed) and a GPU compute
  path is the deferred upgrade. **Accented Edges** is neutral (no-op) at Edge
  Brightness 25 by contract; the app default is set to 38 (CS6's dialog default)
  so the menu item visibly acts. App: `filter_from_kind` maps all 29 kebab-case
  kinds with fixed in-range defaults (`seed: 1` for seeded ones). Self-test exit
  codes **68/69** (`m25_applied applied=29/29`; `m25_deterministic=1`), identical
  on fixture and no-argument runs. Self-test label disambiguation: the earlier
  canvas-perf checks were renamed `m25_*` → `canvas_*` (`canvas_centre`,
  `canvas_middle_pan`, `canvas_move`, `canvas_preview_cache`) so the `m25_` prefix
  belongs to the actual M25 milestone; exit codes 64–67 unchanged. Verified:
  `cmake --build build` OK; both self-tests exit 0; `cargo fmt/clippy` clean;
  **508 tests (0 failed, 1 ignored)** — up from 465; `openspec validate --all
  --strict` 54/54 pre-archive. OpenSpec change `m25-filter-families` adds
  capabilities `brush-stroke-filters`, `sketch-filters`, `texture-filters`,
  `oil-paint-filter` (4 new → **57** capabilities after archive). Deferred
  non-goals: Filter Gallery dialog and cumulative/reorder stack, Smart Filters,
  `Edit > Fade`, 16/32-bit and CMYK/Lab gating, `Load Texture` file I/O, and an
  Oil Paint GPU compute pass.

- **M26** — GPU compute backend (GPU default). `pictura-render::gpu` gains
  `Backend { Gpu, Cpu }`, `gpu_available()` (cached probe), and
  `composite_active(doc, gpu_enabled) -> (PixelBuffer, Backend)`; re-exported from
  the crate root. The compute pipeline/bind-group layout/params are cached once per
  device (was rebuilt per composite). The params uniform was moved from
  process-global into the per-composite `Gpu` — a real race fix for parallel
  composites. Compositor completeness: the four **non-separable** modes
  Hue/Saturation/Color/Luminosity now run on the GPU (mode ids 23–26; separable
  ids 1–22 unchanged); the app's five **adjustment layers** (invert, posterize,
  threshold, brightness/contrast, hue/saturation) are applied on the GPU at the
  layer's stack position, matching the CPU oracle; **Dissolve** is the only
  remaining CPU-only blend mode (random, not bit-reproducible). Layer source/mask
  uploads are now rect-only. Parity: separable 26 modes max delta 0 LSB,
  non-separable 4 modes max delta 0 LSB, adjustments 0 LSB except hue/saturation
  1 LSB (all within ±1). App: an app-wide `gpuCompute` preference (default `true`)
  is persisted in the XDG session store, schema **v2** (a missing field or
  schema-1 store loads `true`). A checkable implemented command
  **`view.gpuCompute`** ("Use GPU Compute", View menu after View > Options)
  toggles it, enabled only when an adapter is present (greyed when none). The
  status bar shows `GPU` / `CPU` / `CPU (no GPU)`. Bridge `PictureView` gains
  `set_gpu_compute`, `gpu_compute`, `gpu_available`, `active_backend`;
  `current_buffer`/`document_to_image` route through `composite_active`, so the
  canvas, panels, painting refresh, and every recomposite use the GPU by default,
  falling back to CPU (per call) for `Dissolve`, unsupported adjustments, or any
  `GpuError`. Self-test exit codes **70/71**: `m26_gpu available=1 default_on=1
  off_cpu=1 on_back=1`; `m26_session gpu_off=1 gpu_on=1 default=1` — identical on
  fixture and no-argument runs. Timing evidence (1024×1024, 4 pixel layers, debug
  test build): `m26 timing 1024x1024x4layers: cpu 485 ms, gpu 744 ms` (the GPU
  path is readback-bound). Verified: `cmake --build build` OK; both self-tests
  exit 0; `cargo fmt/clippy` clean; **515 tests (0 failed, 1 ignored)** — up from
  508 (514 before the close-out timing test); `openspec validate --all --strict`
  58/58. OpenSpec change `m26-gpu-compute` adds capability `gpu-compute-backend`
  and MODIFIES `gpu-compositing` (→ **58** capabilities after archive). Deferred:
  on-screen zero-copy present (manual QRhi + QWindow swapchain; `QRhiWidget`
  blocks device adoption — see `crates/pictura-app/GPU-INTEROP-NOTES.md`),
  off-GUI-thread compositing, GPU filters and GPU painting (next change
  `m27-gpu-filter-acceleration`), and Dissolve on GPU.
- **M27** — GPU-native acceleration. The compositor data path is now 8-bit end
  to end: layer sources upload as raw planar 8-bit channel samples over the layer
  rect, the mask uploads as an 8-bit plane, the working canvas is packed 8-bit
  RGBA in a storage buffer, and output is read back as packed 8-bit — no
  host-side full-canvas `f32` buffer and no `f32` readback (was the M26
  bottleneck). Public API and ±1 LSB parity unchanged. Release timing,
  1024×1024×4 pixel layers: **CPU 72 ms → GPU 19 ms (~3.8×; pre-M27 was ~1.2×)**.
  Parity: separable 26 modes 0 LSB, non-separable 4 modes 0 LSB, five adjustment
  layers 0 LSB except hue/saturation 1 LSB, group/mask scenes 0 LSB. **GPU filter
  acceleration:** new `pictura-render` API `filter_gpu_available()` and
  `apply_filter_active(filter, buf, gpu_enabled) -> Result<Backend, FilterError>`
  (re-exported from the crate root); a wgpu compute shader (kernel / separable-H /
  separable-V / motion / combine modes, weights from the CPU kernel builders)
  accelerates nine filters **byte-exact (0 LSB vs the CPU oracle, alpha
  bit-identical)** — Gaussian Blur, Box Blur, Motion Blur, Blur, BlurMore,
  Sharpen, SharpenMore, Unsharp Mask, High Pass — while every other filter falls
  back to the CPU `pictura_filters::apply` byte-identically (including the M22
  Artistic and M25 Brush Strokes/Sketch/Texture families and Oil Paint).
  `pictura_render::apply_filter` gained a `gpu_enabled` parameter and routes
  through `apply_filter_active`; the app passes the M26 `gpuCompute` flag, so
  filters are GPU-accelerated by default and CPU when disabled or no adapter
  exists. `pictura_render::gpu::shared_device()` (crate-private) now serves both
  the compositor and the filter path; the filter path no longer opens a second
  Vulkan device. Self-test exit code **72**: `m27_filter byte_identical=1` — a
  Gaussian Blur through the GPU path and the CPU path produces byte-identical
  images, identical on fixture and no-argument runs. Verified: `cmake --build
  build` OK; both self-tests exit 0; `cargo fmt/clippy` clean; **519 tests (0
  failed, 1 ignored)** — up from 515; `openspec validate --all --strict` 59/59.
  OpenSpec change `m27-gpu-acceleration` adds capability
  `gpu-filter-acceleration` and MODIFIES `gpu-compositing` (→ **59** capabilities
  after archive). Deferred: GPU kernels for the painterly/stochastic filter
  families and Oil Paint; GPU painting/brush; on-screen zero-copy present;
  off-GUI-thread compute; Dissolve on GPU.
- **M28** — GPU heavy filters. `crates/pictura-filters/tests/profile.rs` profiles the
  CPU filters at 1024² (release) and ranks them by cost; the heavy **deterministic**
  kernels were then ported onto the M27 GPU filter path. Six new kernels —
  **Surface Blur, Maximum, Minimum, Median, Custom (5×5), Oil Paint** — match the
  CPU oracle within ±1 LSB (mostly 0) with alpha bit-identical. Surface Blur is a
  bilateral `SURFACE` window (spatial Gaussian × luma-range Gaussian, `f32`
  accumulation, `round_away` quantize); Median is an **exact order statistic**
  (binary search over the byte value), not a separable approximation;
  Maximum/Minimum use the morphology window; Custom adds the 5×5 `KERNEL` path.
  **Oil Paint now runs on the GPU** (four passes: luma → edge-stopping directional
  aggregation → height/bristle → Lambert/Blinn-Phong shading), resolving the M25
  CPU-only deferral. The GPU filter path therefore accelerates the previously-named
  nine kernels plus these six. Measured GPU speedups at 1024² (release, this box):
  Surface Blur **7161 → 83 ms (~86×)**, Median **1273 → 6.2 ms (~205×)**, Oil Paint
  **10063 → 15.3 ms (~658×; the brief's baseline was ~1446 ms)**, Custom
  byte-exact. **Selection is profile-driven:** a filter is GPU-accelerated only if
  it is high-cost (≥ ~150 ms at 1024²) *and* reaches ±1 LSB. The stochastic/seeded
  filters stay on the CPU with a byte-identical fallback — `Crystallize`
  (3114 ms), `Watercolor`, `Conté Crayon`, `Paint Daubs`, `Dry Brush`,
  `Ocean Ripple`, `Spatter`, `Sponge`, `Palette Knife`, `Add Noise`,
  `Colored Pencil` — because their RNG stream cannot be reproduced bit-exactly on
  the GPU; the warps/distort and render filters are likewise deferred. Self-test
  exit code **73**: `m28_heavy byte_identical=1` (Surface Blur and Median produce
  identical images through the GPU and CPU paths), identical on fixture and
  no-argument runs. Verified: `cmake --build build` OK; both self-tests exit 0;
  `cargo fmt/clippy` clean; **523 tests (0 failed, 1 ignored)** — up from 519;
  `openspec validate --all --strict` 60/60 pre-archive. OpenSpec change
  `m28-gpu-heavy-filters` MODIFIES `gpu-filter-acceleration` (no new capability →
  **59** capabilities after archive). Deferred: stochastic-family GPU kernels via
  CPU-pre-generated RNG fields, GPU painting/brush, on-screen zero-copy present,
  off-GUI-thread compute, Dissolve on GPU.
- **M29** — large-document performance. A 4000×4000 layer move was 3–5 s per
  update; two root causes were fixed. **GPU dispatch cliff:** the compositor and
  filter paths dispatched a 1-D grid (`ceil(n/64)` workgroups, limit 65535 ≈
  4.19 MP), so 4000² (16.7 MP, 250 000 workgroups) silently fell back to the CPU.
  Both now use a **2-D dispatch** (`x = min(ceil(n/64), 65535)`,
  `y = ceil(ceil(n/64)/x)`, shader index `gid.x + gid.y * (grid_x*64)`);
  rejection only when the 2-D workgroup product is exceeded (~2.8×10¹⁴ px).
  4000² now composites on the GPU (`Backend::Gpu`): **CPU 887 ms → GPU 332 ms**
  (release, 3 layers, 1 LSB). Accelerated filters (Surface Blur, Median) also run
  on the GPU at >4.19 MP. **App-side per-update costs:** `sample_argb`
  re-composited the whole document (~286 ms) — it now reads the cached `image`
  QImage (**0 ms**); `layer_thumbnail(24)` built a full-size RGBA image — it now
  gathers the planar channels straight into a ≤24 px buffer (**90 ms → 0.02 ms**);
  the histogram scans all pixels — it now downsamples to ≤512² first
  (~145 ms → <10 ms); `begin_move_preview` cloned the whole document — it now
  hides the layer in place (**344 → ~306 ms**); `commit_move` composites through
  the new GPU-aware `translate_layer_active` (**739 → ~403 ms**). New
  render-crate API `translate_layer_active(doc, dx, dy, gpu_enabled)` (the CPU
  `translate_layer`/`recompute` remain the oracle). Self-test unchanged codes; the
  temporary 4000² timing block was removed (self-test stays fast) and the
  `move_profile_4000`/`move_profile_1024` tests are `#[ignore]`d (run with
  `cargo test -p pictura_app --release -- --ignored --nocapture move_profile`).
  Verified: `cmake --build build` OK; both self-tests exit 0; `cargo fmt/clippy`
  clean; **530 tests (0 failed, 3 ignored)** — up from 523; `openspec validate
  --all --strict` 60/60. The M29 change MODIFIES `gpu-compositing`,
  `gpu-filter-acceleration`, `info-histogram-panel`, `layers-panel`,
  `document-canvas`; no new capability (→ **59** capabilities after archive).
  Remaining bottlenecks: the history capture still clones the whole document
  (~60 ms/state, and up to 20 states of memory) — **copy-on-write or tile diffs**
  is the deferred fix. (The initial "readback-bound" guess was measured wrong in
  M32: the 4000² composite is dominated by host-side CPU per-pixel assembly, not
  the 64 MB readback — see below.)
- **M30** — canvas transparency and clipping. `ImageView::paintEvent` now draws a
  **transparency checkerboard** behind the document so pixels with alpha < 255
  reveal it (a fully transparent document shows the checkerboard; opaque pixels
  cover it). The checkerboard is **screen-space** (constant 8 px cells,
  independent of zoom), **anchored to the document origin** (stable while
  panning), rendered with a cached 2×2-cell `QPixmap` tile via a brush origin and
  clipped to the document rect ∩ viewport (O(1), never allocated beyond the
  screen). Two light tones `#FFFFFF` / `#CCCCCC` (Photoshop "Light" grid). All
  canvas content — the composited image, the Move-tool live preview layer, and the
  selection overlay — is now **clipped to the document rect**, so a layer dragged
  outside the canvas is cropped instead of drawn over the surrounding area. Test
  hooks `ImageView::transparencyCellSize()/transparencyColorA()/transparencyColorB()`
  were added. Self-test exit code **74**: `m30_canvas checker=1 clipped=1` (a
  transparent document shows both checker tones inside the document rect and the
  canvas colour outside; a preview layer dragged past the canvas edge is clipped),
  identical on fixture and no-argument runs. Verified: `cmake --build build` OK;
  both self-tests exit 0; `cargo fmt/clippy` clean; **530 tests (0 failed, 3
  ignored)** (no Rust change; the check is in the C++ self-test); `openspec
  validate --all --strict` 60/60. The M30 change MODIFIES `application-shell`
  (no new capability → **59** capabilities after archive). Deferred: the
  `Transparency & Gamut` preferences pane (grid size None/Small/Medium/Large and
  colour sets Light/Medium/Dark/Red/Custom), the `View > Show > Transparency Grid`
  toggle, gamut warning, and the GPU/RHI-backed canvas.
- **M31** — region (dirty-rect) compositing. New render API
  `pictura_render::composite_region_active(doc, rect: PsdRect, gpu_enabled) ->
  (PixelBuffer, Backend)` composites only the (clamped) rect and returns a
  rect-sized buffer **byte-identical (0 LSB)** to the corresponding
  sub-rectangle of `composite_active`, verified across separable, non-separable
  (Hue), adjustment-layer, masked and isolated-group scenes. The GPU path threads
  a region origin/size/stride so the canvas, per-layer sources, mask and group
  inner canvases are region-sized and only the region is dispatched and read
  back; the CPU fallback is a full composite + slice (`ponytail:`).
  `composite_gpu`/`composite_active` are unchanged (the region kernel over the
  full rect). Measured: 4000² with a 512² region **52 ms vs 3082 ms full (59×)**.
  New `pictura_render::translate_layer_rect(doc, dx, dy)` shifts a layer's rect
  (and mask) **without** recomputing (the CPU `translate_layer`/`recompute`
  remain the oracle). App: `PictureView` caches the composited document `QImage`;
  `refresh_region(rect)` composites the rect through the active backend and
  patches the cached image (and `doc.composite`) so pixels outside the rect are
  untouched. `commit_move` now invalidates `old ∪ new` layer bounds and refreshes
  only that region; `paint_dab` invalidates the stroke's `dirty()` rect. All
  other mutations keep the full recomposite path (the incremental-extension
  point). Measured region-move: a 64² region **0.497 ms vs 585 ms full
  (~1170×)**. A guard (`REGION_REFRESH_BUDGET = 1_000_000` px) falls back to the
  full recomposite for large dirty unions, because the cached image is patched
  with per-pixel `QImage::set_pixel_color` (FFI per pixel) — the `ponytail:`
  upgrade is a C++ `ImageView::blitRegion` via
  `QPainter::CompositionMode_Source`, or M34's GPU-resident present. Self-test
  exit code **75**: `m31_region moved=1 outside_unchanged=1 undo=1` and
  `m31_region_large moved=1 vacated=1 undo=1` (the latter drives the
  full-recomposite fallback), identical on fixture and no-argument runs.
  Verified: `cmake --build build` OK; both self-tests exit 0; `cargo fmt/clippy`
  clean; **536 tests (0 failed, 3 ignored)** — up from 530; `openspec validate
  --all --strict` 60/60. The M31 change MODIFIES `gpu-compositing` and
  `document-canvas` (no new capability → **59** capabilities after archive).
- **M32** — interactive-path region completion and present caching.
  `begin_move_preview` no longer composites the whole document: it
  region-composites only the moved layer's clamped rectangle with that layer
  hidden (`composite_region_active`) and blits it into a clone of the cached
  canvas, **byte-identical** to a full composite with the layer hidden; it falls
  back to the full path when the layer has no image, the clamped rect is empty or
  over `REGION_REFRESH_BUDGET`, or the cache is null/stale. `set_layer_visible`
  refreshes only the toggled layer's influence rectangle — a raster layer's
  `rect`, or a bounded adjustment layer's mask rect (enabled + data + zero
  `default_color`) — and falls back to a full recomposite for groups, unbounded
  adjustments and masks with a non-zero default. `ImageView` now presents from a
  zoom cache keyed on the source `cacheKey()` and the zoom, built **through a
  `QPainter`** so it is pixel-identical to the previous transform draw (a
  `QImage::scaled` smooth build diverged from the painter's non-smooth filter and
  fractional-offset phase — that was found and fixed); the cache is invalidated on
  image or zoom change and falls back to the transform draw above a 64 MP bound.
  Test hooks: `setPresentCacheEnabledForTest`,
  `presentCacheRebuiltOnLastPaint/RebuildCount/ImageSize/ImageKey`; a
  `topmost_pixel_layer_index` bridge accessor was added for the self-test. The
  measurement that scoped M32: a full 4000² GPU composite costs ~254 ms, split
  `build_source` 146 ms (58%), `to_pixel_buffer` 35 ms, `mapped.to_vec` 22 ms,
  `zero_canvas` 18 ms, `build_mask` 16 ms, readback submit/poll/map 7 ms, GPU
  dispatch 0.2 ms — **host-side CPU assembly dominates, not the readback**, which
  is why M32 deferred the Qt Quick zero-copy present (M34) and why M33 targets the
  assembly. Measured move preview (4000², release, RTX 3090): the old body (hide
  layer → full composite) **276 ms**; the new region body with a canvas-sized layer
  is over the per-pixel blit budget and falls back (**280–290 ms**, unchanged),
  while a moved **512²** layer takes the region path at **~30 ms (~10×)**. Self-test
  exit codes **76/77/78**: `present_cache reused=1 zoom_rebuild=1 src=32x32 z0=1
  z1=2 size=64x64 stable=1 identical=1` and `m32_region preview_base=1
  visibility=1`, identical on fixture and no-argument runs. Verified: `cmake --build
  build` OK; both self-tests exit 0; `cargo fmt/clippy` clean; **539 tests (0
  failed, 3 ignored)** — up from 536; `openspec validate --all --strict` 60/60. The
  M32 change MODIFIES `document-canvas` (no new capability → **59** capabilities
  after archive).
- **M33** — full-composite throughput. `build_source` assembles layer sources with
  whole-row `copy_from_slice` per plane (row-wise fast path; the per-pixel scan
  remains the fallback when a channel does not cover the clamped row intersection,
  preserving grayscale 2-plane, missing-G/B-aliases-channel-0, and
  missing-alpha-fills-255 semantics); `build_mask` fills the influence rect
  row-wise when there is no enabled data-carrying mask (per-pixel `mask_alpha`
  retained otherwise); the readback de-interleaves directly from the mapped
  staging slice into the planar `PixelBuffer` with no host packed RGBA
  intermediate (`to_pixel_buffer` walks 4-byte chunks via `as_chunks::<4>`); the
  canvas is cleared with a GPU command (`clear_buffer`) instead of a host
  full-canvas zero write. Output stays byte-identical (GPU 0 LSB, ±1 LSB vs the
  CPU oracle, alpha unchanged) — `cargo test --workspace` includes the new
  equivalence tests `m33_rowwise_source_matches_per_pixel` and
  `m33_rowwise_mask_matches_per_pixel`, and `gpu_parity` (18 tests) passes.
  Measured 4000² (2 RGB layers, release, RTX 3090): total **~123 ms (from
  ~254 ms, ~2×)**; `zero_canvas` **0.05 ms (from 18)**, `build_source` **~69 ms
  (from 146)**, `build_mask` **~15 ms (from 16)**, readback (transfer +
  de-interleave) **~38 ms (from ~57 = 22 `to_vec` + 35 de-interleave)**,
  dispatch 0.2 ms. 1024² total **~4.3 ms**. The remainder is dominated by the
  per-composite source/mask **upload** (~128 MB + 16 MB per composite), which only
  resident per-layer GPU buffers would remove (deferred). M33 stopped here
  because the measured bottleneck after the change is the upload, and residency
  needs content versioning (deferred); `build_mask` gained least because its
  upload dominates the removed `mask_alpha` calls. Self-test: unchanged codes;
  **541 tests, 0 failed, 5 ignored** (was 539, 3 ignored); `openspec validate
  --all --strict` 60/60; the M33 change MODIFIES `gpu-compositing` only (no new
  capability → **59** after archive).
- **M33 follow-up — A2: shader-side planar output** (direct continuation of the
  M33 deferred item; no OpenSpec change, no new capability). A second compute
  shader (`crates/pictura-render/src/gpu/shader.rs` `PLANAR_SHADER`, entry
  `cs_planar`) de-interleaves the packed RGBA canvas into four byte planes in one
  storage buffer (`backend.rs` `PlanarResources`, cached in `Devices` via
  `planar_resources()`), and `Gpu::read_canvas` copies each plane's `n` bytes
  into the planar `PixelBuffer` instead of gathering a channel per pixel.
  `to_pixel_buffer` stays (`#[cfg(test)]`) for the profile micro-benchmark.
  Byte-identical: the 20 `gpu_parity` tests pass. Measured 4000² (2 RGB layers,
  release, RTX 3090): `readback` **~41 ms → ~27–32 ms**, `total` **~130 ms →
  ~117–131 ms** (the GPU planarize dispatch and the 64 MB plane stream are
  bandwidth-bound; the host gather is gone). `cargo test --workspace` **592
  tests, 0 failed, 9 ignored** (unchanged); self-test stderr byte-identical.
- **M36 — layer attributes end-to-end** (the first milestone of the Layers-panel
  program; see `docs/dev/layers-panel-program.md`). `pictura_core::Layer` gained
  `fill: u8` (default 255), `lock: LockFlags` (newtype, `TRANSPARENCY|PIXELS|
  POSITION`, `all()` = 0x07) and `color: ColorLabel` (None/Red/Orange/Yellow/
  Green/Blue/Violet/Gray); 37 explicit `Layer { .. }` literals across 14 files
  were updated with the defaults (plus the `..bare` update-syntax layer in
  `pictura-core`'s `masked` test). The compositor now uses effective layer alpha
  = `opacity/255 × fill/255` in both the CPU oracle and the GPU shader (a new
  `fill` uniform word; groups use 1.0), byte-identical to the previous composite
  at `fill == 255` and within ±1 LSB on the GPU. The codec reads/writes the
  `lspf` (lock), `lclr` (color) and `iOpa` (fill) additional-layer blocks,
  omitted at defaults — a default document's `write_psd` output is byte-identical
  (proven against `crates/pictura-codec/tests/fixtures/default_before.psd`),
  and psd-tools reads the new attributes back. The bridge gained
  `layer_fill`/`set_layer_fill`, `layer_lock`/`set_layer_lock`,
  `layer_color`/`set_layer_color` with the frozen refusal rules (fill refused for
  group/Background/fully-locked; opacity refused for Background/fully-locked;
  lock/color refused for Background) and history labels `Fill Opacity`/`Lock`/
  `Layer Color`. The panel gained a Fill spinbox, a four-button lock strip and an
  eight-entry color-label context menu. Honest limits: CS6's `lspf` "Lock All"
  high-bit `0x80000000` encoding is not handled (writes `0x07`); `layer_kind`'s
  `"background"` is a name+index heuristic (M37 replaces it); the color context
  menu is not greyed on the Background row (the bridge refuses). The `lspf`/
  `iOpa`/`lclr` source disagreements are recorded in
  `docs/dev/layers-panel-program.md` §5. Self-test exit codes 86–89:
  `m36_attrs fill=1 lock=1 color=1 undo=1`, identical on the fixture and
  no-argument runs. Verified: `cmake --build build` OK; both self-tests exit 0;
  `cargo fmt --all --check`/`cargo clippy --workspace --all-targets --
  -D warnings` clean; **556 tests, 0 failed, 7 ignored** (up from 546/7; the
  ignored set is unchanged, so the raw `cargo test` ignored count is 8 with the
  `pictura-render` doctest); `openspec validate --all --strict` 60/60. The M36
  change MODIFIES `layers-panel` and `psd-layer-io` and ADDs to
  `layer-compositing` (no new capability → **59** capabilities after archive).
  Deferred: group Fill in compositing (CS6 has no group Fill; the value
  round-trips and is ignored), the forced type/shape locks, and a first-class
  Background flag (M37).

- **M37 — layer creation and grouping** (the second milestone of the Layers-panel
  program; see `docs/dev/layers-panel-program.md`). New
  `crates/pictura-render/src/document_ops/layer_ops.rs` (418 lines) adds five pure
  functions over `&mut Document` — `add_layer(doc, above, name) -> i32`,
  `add_group(doc, above, name) -> i32`, `duplicate_layer(doc, index) -> i32`,
  `group_layer(doc, index) -> i32`, `ungroup_layer(doc, index) -> bool` — plus
  `next_layer_name(doc, prefix)`, re-exported from `document_ops` and the crate
  root. Insertion is "directly above `above`" = `above + 1` in the bottom-first
  stack, clamped to the top; a negative sentinel (no selection) or an out-of-range
  value also lands on top. `duplicate_layer` deep-clones the node (children,
  channels, mask, adjustment and all attributes) directly above the source and
  names the copy `"<name> copy"`; `group_layer` wraps the target in place (the
  group takes the layer's slot, the layer becomes its only child) and names it
  `"Group N"` via `next_layer_name`; `ungroup_layer` splices the children back in
  order and refuses a non-group (state unchanged). A new layer is a
  document-sized transparent raster layer (channels `0/1/2/-1` of `w*h` zero
  bytes, `Normal`/opacity 255/fill 255/visible) and leaves `composite_rgba`
  unchanged; a new group is an empty `is_group` node with a `Normal` blend (not
  `PassThrough`) and an empty `{0,0,0,0}` rectangle. Ceiling: the empty layer is
  stored document-sized and costs `w*h*4` bytes before it is painted — Photoshop
  stores nothing until a dab — marked with a `// ponytail:` empty-rect upgrade in
  the module. The bridge `PictureView` gains `add_layer`/`add_group`/
  `duplicate_layer`/`group_layer`/`ungroup_layer` `#[qinvokable]`, each
  `recomposite()`-then-`record()` with labels `New Layer` / `New Group` /
  `Duplicate Layer` / `Group Layers` / `Ungroup Layers`; a failed op records
  nothing. The panel adds **New Group** then **New Layer** buttons before Delete
  (Add Adjustment / New Group / New Layer / Delete / Move Up / Move Down) and
  `LayersPanel::currentLayer()`/`selectLayer(int)`; the five `Layer` leaves are
  frozen to `command_ids` (`layer.new.layer`, `layer.new.group`,
  `layer.duplicate.layer`, `layer.group.layers`, `layer.ungroup.layers`) and
  handled in `frame.cpp` against the active view's current layer. Honest limit:
  `Group Layers`/`Ungroup Layers` (and `Duplicate Layer`) act on the **single**
  selected layer — CS6 groups a multi-selection; M39's selection work upgrades
  these to per-selection operations (the handler carries the `// ponytail:` note).
  Self-test exit codes 90–94: `m37_create new=1 group=1 duplicate=1 ungroup=1
  undo=1` (count growth, an inert transparent layer, a `" copy"` duplicate name,
  wrap/unwrap ordering, one history step per op, and five undos restoring the
  start), identical on the no-argument and `two_layers.psd` runs. Verified:
  `cmake --build build` OK; both self-tests exit 0 with no FAILs; `cargo fmt --all
  --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean;
  **568 tests, 0 failed, 7 ignored** (up from 556/7; the ignored set is unchanged,
  so the raw `cargo test --workspace` ignored count is 8 with the pre-existing
  `pictura-render` doctest); `openspec validate m37-layer-creation --strict` valid
  and `openspec validate --all --strict` 60/60. The M37 change MODIFIES
  `layers-panel` (no new capability → **59** capabilities after archive).
  Deferred: a New Layer / New Group **dialog** (neutral-color fill, blend/opacity,
  use-previous-as-clipping), multi-selection grouping, and empty-rect layer
  storage.

- **M38 — icon and cursor library** (a user-requested interruption to the
  Layers-panel program; see `docs/dev/layers-panel-program.md` and the contract
  `docs/dev/m38-icon-cursor-library.md`; OpenSpec change
  `m38-icon-cursor-library`). The full CS6 toolbox catalogue and icon/cursor set
  landed in the Qt shell only — no Rust, bridge, document, compositor, or PSD
  change. **Assets:** `assets/icons/` now holds 137 SVGs and `assets/cursors/`
  71; `assets/pictura.qrc` was regenerated to 208 entries with the on-disk set
  equal to the listed set (all parse under `xmllint`, none use `<text>`, and a
  48 px render check found no blank and no solid-fill assets). **Tool
  catalogue:** `ToolId`/`ToolInfo` in `tools.{h,cpp}` expanded from the M19/M23
  ten to all **71** CS6 tools in catalogue order, each carrying `name`, `label`,
  `shortcut`, `Qt::CursorShape`, `hint`, `group` 1..23, `implemented`, and
  `hotspotX/Y`; `allToolIds()` returns 71, `implementedToolIds()` 10,
  `toolImplemented()` answers membership, and `static_assert(kToolCount == 71)`
  guards the table (the separate `ToolCatalogueEntry` layer in the design was
  collapsed into the existing `ToolInfo` table). **Toolbox:** rebuilt as one
  button per group (23 slots, single column); a slot shows its current member
  (first implemented member, else first) and gets a flyout triangle plus a
  hold/right-click menu only when the group has more than one member; Alt-click
  cycles the implemented members; an all-unimplemented slot is disabled with the
  exact `<label> — not implemented yet` tooltip; `setActiveTool` refuses an
  unimplemented id. The fg/bg swatch widget and the Screen Mode button are
  unchanged below the slots; `slotButtons()` exposes the slots for the
  self-test. **Cursors:** `cursor(id, hotX, hotY)` was added (the 24×24 centre
  default is preserved); the hardcoded eyedropper hotspot is gone and per-tool
  hotspots come from the catalogue; a null or failed SVG render falls back to
  the tool's `Qt::CursorShape`, so a missing asset can never yield an invisible
  cursor. **Panels:** rail buttons carry their `window.panels.<panel>` icons
  (history/actions/info/navigator/histogram — none null); the Layers strip is
  the CS6-order icon set `link, fx, mask, fillAdjustment, group, newLayer,
  delete` (link/fx/mask disabled "not implemented yet";
  fillAdjustment/group/newLayer/delete wired to the existing behaviour); the
  History snapshot button uses `history.snapshot`. Existing objectNames and
  `currentLayer()`/`selectLayer()`/`setView` are unchanged. **Self-tests:**
  `m38_tools icons=1 cursors=1 slots=1 guard=1` (exit codes 95–98) and
  `m38_panels rail=1 strip=1 history=1` (codes 99–101), both identical on the
  no-argument and `two_layers.psd` runs; all earlier lines/codes are unchanged
  (`m23_toolbox dock=1 buttons=24 fgbg=1` still passes). Two fixes during the
  milestone: `assets/cursors/tool.move.svg` was clobbered by a parallel asset
  agent and restored from git before the required compound rewrite, and
  `tool.brush`/`tool.pencil` had no cursor assets at all (the original defect)
  and were added. **Honest limits:** dock-tab window icons were not wired (only
  the rail, Layers strip, and History snapshot get icons); the 3D-object,
  3D-camera, and Count slots are Extended-only and every all-unimplemented slot
  is disabled; the blur/sharpen/smudge slot has no default letter (flyout or
  Alt-click only); the toolbox 1-/2-column toggle is not present (single column
  only); and the **functionality** of the 61 new catalogue entries is not
  implemented — assets and disabled UI only. Verified: `cargo fmt --all
  --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean;
  **568 tests, 0 failed, 7 ignored** (unchanged — no Rust change; the raw
  `cargo test --workspace` ignored count is 8 with the pre-existing
  `pictura-render` doctest); `openspec validate m38-icon-cursor-library
  --strict` valid and `openspec validate --all --strict` 60/60; both self-tests
  exit 0. The M38 change ADDs to `application-shell` and `layers-panel` and
  MODIFIES `tool-framework`, `panel-rail`, `svg-cursors` (no new capability →
  **59** capabilities after archive). Because it claimed the M38 number, the
  Layers-panel program's stages shift by one: **M39** panel anatomy, **M40**
  filtering/search, **M41** remaining management ops, **M42** styles/effects,
  **M43** smart objects / vector masks / artboards / layer comps; the deferred
  canvas-performance tracks stay by name.

- **M39 — panel anatomy** (the Layers-panel program's panel-anatomy milestone
  after the M38 interruption; see
  `docs/dev/layers-panel-program.md`; OpenSpec change `m39-panel-anatomy`, brief
  `docs/dev/m39-panel-anatomy.md`). The Layers panel becomes a full expandable
  tree over the document's existing `Layer.children` hierarchy, and edits become
  path-addressed and selection-based. **Path/tree core**
  (`crates/pictura-render/src/document_ops/layer_ops.rs`): a frozen path grammar
  (`segment *("/" segment)`, digits with no leading zeros, bottom-first),
  `resolve_path`/`resolve_path_mut`/`parent_path`, `flatten_rows ->
  Vec<(String,u32)>` (depth-first, topmost-first `(path, depth)` pairs), and
  `is_background` (the single source of truth; the app's `is_background_layer`
  delegates). Batch ops `set_visible_paths`/`apply_visibility`/`set_blend_paths`/
  `set_opacity_paths`/`set_fill_paths`/`set_lock_paths`/`set_color_paths`/
  `delete_paths`/`duplicate_paths`/`group_paths`/`ungroup_paths`/`rename_path`/
  `move_path`/`add_layer_in`/`add_group_in` apply the frozen per-node skip vs
  whole-op refuse table (only `group_paths` refuses the whole op); structural ops
  resolve-then-apply deepest-first with ancestor-descendant dropping so indices
  never invalidate. `move_path` refuses the Background and fully-locked layers
  (the LAY-002 ruling). **Bridge** (`cxxqt_object/impl_layers.rs`): `layer_row_count` + 17
  `layer_row_*` getters over the projection (path, depth, name, kind, visible,
  blend, opacity, fill, lock, color, clipping, has-mask, has-adjustment,
  expandable, child-count, thumbnail, mask-thumbnail); `set_layer_name_path`/
  `move_layer_path`; `QStringList` batch mutators for visible/blend/opacity/fill/
  lock/color plus `apply_visibility(paths,label)`, `delete_layers`,
  `duplicate_layers`, `group_layers`, `ungroup_layers`; `add_layer_in`/
  `add_group_in`. Undo contract: `recomposite()` then `record(label)` only when
  the changed count is non-zero; a zero-change call records nothing and emits no
  `changed`. Labels per the design (`Set Visibility`, `Solo Visibility`,
  `Restore Visibility`, `Group Layers`, …). Every legacy top-level
  `layer_*(i)`/`set_layer_*(i)`/M37 op is left **byte-identical** — the new path
  surface is the tree implementation and nothing calls legacy after the panel
  migration — with the deliberate asymmetry that legacy `move_layer` still swaps
  unconditionally while `move_layer_path` refuses. Thumbnails:
  `layer_row_thumbnail(i,size,entire_document)` (layer bounds vs
  document-positioned) and `layer_row_mask_thumbnail`; groups and adjustments
  return a null image (the delegate draws a folder glyph). **Panel**:
  `LayersModel` becomes a `QAbstractItemModel` over the flat projection (roles,
  `internalPointer` path, `CheckStateRole`→visibility, `EditRole`→rename);
  `LayerRowDelegate` paints the eye (with a press hit-test), thumbnail/folder
  glyph, name, color swatch, clip indent + base underline, mask thumbnail, and
  `fx` badge (null-asset safe); a `QTreeView` with `ExtendedSelection`,
  `SelectRows`, `uniformRowHeights`, `expandsOnDoubleClick(false)`,
  `dragEnabled(false)`; panel-side expansion (`QSet<QString>`, default collapsed,
  new groups expanded) and path-based re-selection; multi-selection routing for
  the header controls and Delete/Duplicate/Group/Ungroup (one undo step each);
  solo (`Alt`-click) snapshots visibility and uses `apply_visibility` for a
  one-step exact restore; rename `Tab`/`Shift+Tab` moves to the next/previous
  visible row with no wrap; tooltips `"<name> (<kind>)"`; Panel Options (Medium /
  Entire Document / Expand New Effects on) persisted in session **schema v3**
  (`layersThumbSize`/`layersThumbContents`/`layersExpandNewEffects`); the panel
  menu (`layersPanelMenu`) + row context menu with the wired commands only and a
  `Color Label` submenu; eye right-click show-only/show-all. The seven-button
  strip is unchanged and reordering is menu-only. **Fixes/decisions:**
  `PicturaMainWindow::saveSession()` now loads before writing so the v3 fields
  are not clobbered; `layerTooltip` now always returns `"<name> (<kind>)"` (the
  old helper predated M39 and would have failed the new tooltip check).
  **Self-tests:** `m39_tree` (102/103), `m39_multi` (104/105), `m39_solo` (106),
  `m39_rename` (107), `m39_options` (108), `m39_badges` (109), `m39_menus` (110),
  `m39_tooltip` (111), `m39_strip` (112); all earlier lines/codes unchanged.
  **Honest limits:** the clip-indent rendering is not positively proven (no
  bridge operation can set `Layer.clipping` and no fixture has a clipped layer —
  `m39_badges clip=1` only proves the role is plumbed and `clipBase` is not
  spuriously set); group thumbnails are folder glyphs (no group composite);
  drag-reorder is explicitly deferred to M41; multi-row move is deferred;
  expansion is session-only (not persisted); solo is one undo step per direction.
  Verified: `cargo fmt --all --check` and `cargo clippy --workspace --all-targets
  -- -D warnings` clean; **588 tests, 0 failed, 7 ignored** (up from 568/7; the
  ignored set is unchanged, so the raw `cargo test --workspace` ignored count is
  8 with the pre-existing `pictura-render` doctest); `openspec validate
  m39-panel-anatomy --strict` valid and `openspec validate --all --strict`
  60/60; both self-tests exit 0. The M39 change MODIFIES `layers-panel` (no new
  capability → **59** capabilities after archive). Deferred within the program:
  filtering/search (M40), the remaining management ops and drag-reorder (M41),
  styles/effects (M42), smart objects / vector masks / artboards / layer comps
  (M43).

- **M40 — CS6 Tools panel** (a second user-requested interruption to the
  Layers-panel program; OpenSpec change `m40-tools-panel`, brief
  `docs/dev/m40-tools-panel.md`). The Tools panel becomes CS6-shaped. **Flyout:**
  `ToolSlotButton` paints a 5 px filled triangle at the **bottom-right** when the
  slot's group has ≥2 members (counting unimplemented members); `MenuButtonPopup`
  and `setMenu` are gone so the icon is centred with no stock arrow. A **300 ms**
  hold timer opens the group `QMenu` below the button (screen-clamped);
  **right-click opens immediately**; a release before the timeout selects
  normally; `Alt`+click still cycles. Menu items carry the group's key via
  `QAction::setShortcut` + `setShortcutVisibleInContextMenu(true)` +
  `setShortcutContext(Qt::WidgetWithChildrenShortcut)` (no window-global shortcut,
  so a disabled item cannot steal the key); unimplemented members stay disabled
  with the "not implemented yet" tooltip. `refreshSlot()` no longer calls
  `setShortcut`. **Columns:** a custom `QDockWidget` title bar (`Tools` label + a
  flat double-arrow button, `objectName` `toolsColumnToggle`) toggles one/two
  columns; the icon shows the **target** layout
  (`assets/icons/panel.columnsTwo.svg` in one column, `panel.columnsOne.svg` in
  two — both new, 24×24 stroke-`#c8c8c8`, added to the regenerated qrc); reflow is
  row-major `(i/2, i%2)` with `minimumWidth` 66 → 104; the fg/bg widget and Screen
  Mode button stay pinned below the slots. **Standalone dock:**
  `setAllowedAreas(Left|Right)` and `setFeatures(Movable|Floatable|Closable)`;
  tabification refused by an event filter plus a reactive
  `PicturaMainWindow::ensureToolsNotTabified()` (float → re-add → show) driven from
  `dockLocationChanged`/`topLevelChanged`. **Shift cycling:**
  `toolShortcutKeys()`/`toolGroupForKey()` in `tools.{h,cpp}` and
  `Toolbox::handleToolKey(key, shift)`; `frame.cpp` registers one plain and one
  `Shift`+letter `QShortcut` per distinct key routed to it, replacing the
  hard-coded B / Shift+B `cyclePaintTool`. A plain letter activates the slot's
  current member; `Shift` cycles to the next **implemented** member (skipping
  unimplemented, wrapping); an all-unimplemented group is a no-op;
  `setShiftKeyForToolSwitch(false)` makes the plain letter cycle. Gated on a new
  session preference `useShiftKeyForToolSwitch` (default **true**) — **no UI
  yet**; M41 adds the Preferences dialog (General + Interface) and gives it a home
  alongside `Auto-Collapse Iconic Panels`. **Session v4:** `toolsColumns` (1|2)
  and `useShiftKeyForToolSwitch` with per-key defaults;
  `PicturaMainWindow::saveSession()` still loads-then-writes so unknown keys
  survive; `columnsChanged` persists. **Self-tests:** `m40_columns` (113/114),
  `m40_flyout` (115), `m40_keys` (116), `m40_shift` (117, driven through the real
  `QShortcut`/`QKeyEvent` path), `m40_dock` (118), `m40_session` (119);
  `m23_toolbox` tightened from a loose `buttons < 10` to `buttons != 25` plus
  `toggle=1` (the new title-bar toggle is found by `objectName` and is not one of
  the 23 slots). All earlier lines/codes unchanged. **Honest limits:** Qt has no
  clean per-dock tabify veto — the event filter only covers the toolbox body and a
  drop can briefly tabify before the reactive re-dock; the
  **right-click-immediate and quick-release-select timing are not self-tested**
  (the paths exist but only triangle/member-split/popup-placement are asserted);
  the preference has no UI until M41; the two-column width is 66/104 rather than
  the sketched 34/64 (the swatch + dock chrome need the width); `m40_columns`
  asserts widening via `minimumWidth` (the actual mechanism) with actual width
  only non-decreasing. No Rust changes: **588 tests, 0 failed, 7 ignored**
  (unchanged; the raw ignored count is 8 with the pre-existing
  `pictura-render` doctest). Verified: `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` → `verify-fast: OK`
  (588 tests, 0 failed); both self-tests exit 0 with all m20–m40 lines `=1`;
  `openspec validate m40-tools-panel --strict` valid and
  `openspec validate --all --strict` 60/60. The M40 change MODIFIES/ADDs to
  `tool-framework` and ADDs to `application-shell` (no new capability → **59**
  capabilities after archive).

- **M41 — CS6 panel column** (a third user-requested interruption to the
  Layers-panel program; OpenSpec change `m41-panel-column`, brief
  `docs/dev/m41-panel-column.md`). The fifteen right-hand `QDockWidget`s are
  replaced by a custom column of plain content widgets. **Hosts:** `PanelGroup`
  is a `QTabWidget` with `setTabPosition(QTabWidget::North)` forced; the tab
  text is the panel title and there is **no separate group label** (a
  single-panel group still shows its tab). `PanelColumn` is a `QScrollArea` over
  a vertical `QSplitter` of groups with no hard panel minimums, so the window
  shrinks freely. **Groups (CS6 Essentials, fixed):** `Color | Swatches |
  Styles`; `Adjustments` (plus a hidden `Properties` tab); `Layers | Channels |
  Paths`; `Navigator | Histogram | Info`; iconic `History`, `Actions`; and a
  hidden overflow group `Gradients | Patterns | Libraries` (kept reachable from
  `Window → Panels`). `Styles` is a new placeholder; Properties is folded into
  Adjustments. **Width toggle:** a `panelColumnToggle` double-arrow in the column
  header switches **normal ⇄ iconic**. **Iconic mode** is a vertical icon strip
  with group dividers, labels-on-widen (threshold 120 px), and a `Qt::Popup`
  flyout per panel that reparents the panel in and restores it on close.
  Per-group **Collapse to Icons** reuses the same component; **Minimize** rolls a
  group up to its tab bar (distinct from iconic). **Tab context menu** (right-click
  a group's tab bar), exactly: `Close`, `Close Panel Group`, `Minimize`,
  `Collapse to Icons`, sep, `Auto-Collapse Iconic Panels` (checkable),
  `Auto-Show Hidden Panels` (checkable), sep, `Interface Options…`. **Drag &
  drop:** in-group reorder, cross-group regroup, between-groups insert (new
  group), with a **3 px `#2a7fff` drop indicator** (`panelDropIndicator`) — a
  vertical marker at the tab index or a full-width horizontal bar at the group
  boundary; hidden on commit/cancel/out. **Tear-off:** leaving the column floats
  the source group in a `Qt::Tool` `panelFloat`; dragging it back re-docks at the
  index; the float is hidden/`deleteLater`-ed when emptied. Drops are
  remove-then-insert, so a panel is never double-parented. **M24 `PanelRail`
  deleted** (`panel_rail.{h,cpp}` removed from disk and CMake, creation + five
  `setPanelChecked` connections gone). `Window → Panels` is the single visibility
  path; the `m24_rail` self-test (code 63) was repointed to it (`actions=5
  toggled=1 norail=1`). **Tools panel fixes:** the `"Tools"` title label is gone
  (the `toolsColumnToggle` stays); the hard-coded 66/104 widths became content-fit
  **34 / 65** px; `ForegroundBackgroundWidget` scales to the column (30/40) and
  no longer widens the dock. M40 flyout/hold/right-click/shortcuts/Shift-cycling/
  left-right dock all intact. **Session v5:** `panelRailMode` (`normal`|`iconic`,
  default normal), `railWidth` (0 = derive), `autoCollapseIconic` (default
  **false**), `autoShowHidden` (default **false**), `panelGroups` (per-group
  `order`/`visible`/`minimized`/`collapsed`), `schemaVersion` 5.
  `saveSession()` starts from the parsed on-disk object so unknown keys survive;
  a v4 store loads with v5 defaults. **Preferences dialog** (new
  `preferences_dialog.{h,cpp}`, `objectName` `preferencesDialog`, modeless, page
  list + `QStackedWidget`) with exactly two real pages, **General** (the real
  brightness setting) and **Interface** (`Use Shift Key For Tool Switch`,
  `Auto-Collapse Iconic Panels`, `Auto-Show Hidden Panels`). Wired to
  `Edit → Preferences → General` (`edit.preferences.general`, implemented) and
  `→ Interface` (`edit.preferences.interface`, implemented); the other nine
  Preferences leaves stay disabled no-ops (enablement unchanged).
  `PanelColumn::interfaceOptionsRequested()` opens Interface. **M40's UI-less
  preference now has a home:** `Use Shift Key For Tool Switch` →
  `Toolbox::setShiftKeyForToolSwitch` → `handleToolKey`, persisted.
  **`Auto-Collapse Iconic Panels`** (default off) — when a flyout closes and the
  column is in normal mode it returns to iconic; inert when already iconic; no
  per-panel expand-in-place state machine. **`Auto-Show Hidden Panels`** (default
  off) — the iconic strip includes hidden panels' icons and opening one reveals
  it; inert: no hover-at-edge gesture. **Self-tests**, new codes **120–130**:
  `m41_tabs`(120), `m41_width`(121), `m41_iconic`(122), `m41_menu`(123),
  `m41_minimize`(124), `m41_prefs`(125), `m41_drag`(126), `m41_tearoff`(127),
  `m41_session`(128), `m41_tools`(130). Code **129** (`m41_rail`) was folded into
  the repurposed `m24_rail` rather than allocated. Earlier codes all still pass
  with adapted internals (`m24_groups`/`m24_panels` now assert `PanelColumn`
  membership via `groupOfForTest`, `m38`/`m39`/`m40_dock` use
  `QWidget`/`PanelColumn`). **Harness fix (not product):** `main.cpp` now forces
  `QT_QPA_PLATFORM=xcb` for `--self-test` when both `DISPLAY` and
  `WAYLAND_DISPLAY` are set and no platform is pinned. Without it, a live Wayland
  session leaked through `xvfb-run`, Qt picked Wayland, and a
  programmatically-opened `QMenu` popup could not grab and was dismissed — making
  `m40_flyout` (code 115) flaky (**3/8** pass under Phase A; clean HEAD measured
  **8/8** only by luck, **3/12** in the agent's sample). Deterministic **5/5**
  no-arg after the guard. **Honest limits:** multi-monitor tear-off untested and
  the float is not screen-clamped; no cross-process drag; no translucent drag
  ghost (the float is the feedback); float chrome is a plain WM-framed
  `Qt::Tool`; Escape-cancel is not key-bound (`cancelDrag()` is programmatic
  only); drop-on-gap needs the ~4 px splitter or a group's top/bottom half;
  torn-off floats are not serialized; `panelGroups` keys groups by first-ever
  panel objectName and skips a stale name; iconic label threshold (120 px) and
  popup size are unsourced constants; the frame's own minimum width is ~776 px
  from the M40 options bar (column min is 64 px) so shrink-to-nothing is
  untestable; `Styles` has no dedicated SVG asset; the group collapse-icon row is
  horizontal while the column iconic strip is vertical (shared flyout/button
  code, not the identical widget). No Rust/bridge/codec/compositor/PSD change.
  **Capability:** ADD `panel-column`; the M24 `panel-rail` requirement is
  REMOVED (its spec persists without the rail requirement); MODIFIED
  `application-shell`, `tool-framework`, `workspace-persistence`. 59 canonical
  specs → **60 after archive**. Verified: `cargo fmt --all --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` → `verify-fast: OK`;
  **588 tests, 0 failed, 7 ignored** (unchanged — no Rust change; the raw
  ignored count is 8 with the pre-existing `pictura-render` doctest); both
  self-tests exit 0 with the new `m41_*` lines and every earlier m20–m40 line
  unchanged; `openspec validate m41-panel-column --strict` valid and
  `openspec validate --all --strict` 60/60 pre-archive. The M41 change MODIFIES
  `application-shell`, `tool-framework`, `workspace-persistence` and REMOVES the
  M24 `panel-rail` requirement, ADDing the `panel-column` capability (→ **60**
  capabilities after archive).

- **M42 — panel refinements** (a fourth user-requested interruption to the
  Layers-panel program; OpenSpec change `m42-panel-refinements`, brief
  `docs/dev/m42-panel-refinements.md`, research `docs/dev/m42-panel-menus.md`).
  **Phase A — chrome fixes.** The normal-mode `PanelColumn` minimum width is the
  widest visible group's `sizeHint` clamped to `[180,320]` (iconic-strip floor
  40); entering iconic now starts at the smallest possible width instead of
  keeping the prior splitter width. Iconic-strip buttons grew `24→30` (pixmap
  `16→20`) and Tools slots `30→32` (icons `20→22`). The floated Tools dock hugs
  its content height (the trailing stretch is zeroed on `topLevelChanged(true)`
  and the body layout invalidated so the two-column floor is not cached) and
  stays width-tight (1-col min = content = 36, 2-col 69).
  `ForegroundBackgroundWidget` gains the CS6 double-arrow swap control top-right
  (the default-colors X is kept) wired to the **`X`** key (was free; not in the
  tool catalogue or the frame's shortcut list). **Menu-bar overlay root cause
  (item 8):** `frame.cpp` constructed `actionsPanel_` **twice**; the first
  `PlaceholderPanel("Actions")` was never added to a group, so it remained a
  direct child of the main window and painted its dim "Actions" label over
  `File`/`Edit` — that was the "File Act…" artifact (confirmed against
  `/tmp/opencode/clean_wide.png`; a fresh `XDG_STATE_HOME` reproduced it, so it
  was not the persisted layout). Fixed by deleting the duplicate.
  Defence-in-depth: `saveState()` records `layoutRevision` and
  `restoreStoredLayout` discards a stored layout whose revision mismatches
  (`kLayoutRevision=2`), so the pre-M41 dock blob is no longer restored (**the
  session gains `layoutRevision`; no schema bump**). **Phase B — compact-strip
  drag** reuses the M41 `beginPanelDrag`/`updateDrag`/`commitDrop`/`resolveDrop`
  path (no second drag system): `resolveDrop` gained an `onStrip` branch with
  `stripInsertionIndexAt`, `applyStripDrop` rewrites order via
  `PanelGroup::setPanelOrder` or reuses `takePanel`/`insertPanel`/
  `cleanupEmptyGroup` across groups, and a dedicated `stripIndicator_` draws the
  blue line (the normal indicator lives in the hidden scroll viewport). Commit is
  queued (`Qt::QueuedConnection`) so the strip can rebuild after the button's
  event returns. **Phase C — compact flyout.** Opens on the **inner** side,
  derived from the column's geometry vs its window (right-edge column ⇒ popup to
  the left), clamped to `QScreen::availableGeometry`. The open icon is a
  checkable/pressed strip button cleared on restore. The popup is now
  **group-styled**: a `panelFlyoutHeader` (`panelFlyoutTitle` + stretch +
  `panelFlyoutClose`) above the detached panel, with a new
  `assets/icons/panel.closeChevron.svg` (double right chevron, 24×24 `#c8c8c8`,
  qrc regenerated) for the close button. Still `Qt::Popup` (click-away), still
  reparents the panel back exactly once. **Phase D — per-widget header menu.** A
  `▾` `QToolButton` (`panelWidgetMenu_<panelName>`) is installed as
  `QTabWidget::setCornerWidget(..., Qt::TopRightCorner)` on each `PanelGroup`, and
  follows the **current tab** (menu + tooltip switch on `currentChanged`); hidden
  when the current panel has no menu table. Menu contents are transcribed from
  `docs/dev/m42-panel-menus.md` (CS6 panel fly-outs, order best-effort;
  `[toggle]`/radio entries checkable). Unimplemented entries ship **disabled**
  with `"<label> — not implemented yet"`. `Close`/`Close Panel Group` are
  excluded (they stay on the M41 tab menu). **Wired:** Layers — New Layer…,
  Duplicate Layer/Group…, Delete Layer/Group, New Group…, Group Layers, Ungroup
  Layers, Hide Layers, Arrange ▸ Move Layer Up/Down, Panel Options…; History —
  Step Forward/Backward, New Snapshot…; Adjustments — Invert, Posterize,
  Threshold, Brightness/Contrast, Hue/Saturation. Everything else disabled (all
  of Channels, Paths, Color, Swatches, Styles, Navigator, Histogram, Info,
  Actions, Properties; Channels/Paths `Panel Options…` stay disabled — no options
  dialog exists for placeholder panels). Gradients/Patterns/Libraries get no
  header button (not CS6 panels: picker pop-ups / CC-only Libraries). **Phase E —
  in-window float overlay.** `PanelFloat` is no longer a `Qt::Tool` top-level: it
  is a plain child of the main window (`Qt::Widget`, `WA_StyledBackground`,
  `#panelFloat{background:#3a3a3a;border:1px solid #555}`), raised, clipped to
  `centralWidget()`'s rect and clamped there on every header drag (`moveFloat`).
  It is parented to `window()` **not** `centerSplitter`, because
  `QSplitter::childEvent` auto-inserts non-window children as panes. Re-dock is
  the existing `applyGroupDrop` (remove-then-insert; overlay destroyed once
  emptied). `m41_tearoff` is unchanged and proves the same claim (group left the
  column, contains its panels, re-docks, no float remains); windowness is now
  proven separately by `m42_float_overlay` (`!isWindow()`). **Self-tests**
  131–139: `m42_minwidth`(131), `m42_iconic`(132), `m42_dragstrip`(133),
  `m42_flyout`(134), `m42_widgetmenu`(135), `m42_float_overlay`(136),
  `m42_fgbg`(137), `m42_menubar`(138), `m42_tools`(139); `m41_width`(121) was
  amended (output format unchanged) and all 120–140 lines are `=1`. **Honest
  limits:** the per-panel menu order/separators are the research doc's
  best-effort reconstruction; the disabled entries are stubs (no invented
  dialogs); Layers `Panel Options…`/History `New Snapshot…` open modal dialogs so
  the test asserts structural wiring, not execution; `New Snapshot…` is marked
  disabled in the research doc but was wired to the existing snapshot behavior;
  Adjustments route through the Layers panel's view (the Adjustments placeholder
  has no view handle); the float overlay does not persist position, is not
  re-clamped on window resize, and cannot cover the docked Tools panel (clamped
  to the central-widget rect); min widths/icon sizes are chosen constants, not
  CS6 metrics; `m42_tools` tolerates ±8 px on the float height under xvfb; the
  stale-layout revision guard is defence-in-depth (proven not to be the overlay's
  cause); drop-on-stack from the strip requires the column to be expanded
  mid-drag. No Rust change: **588 tests, 0 failed, 7 ignored** (unchanged; the
  raw ignored count is 8 with the pre-existing `pictura-render` doctest).
  **Capability:** MODIFIES `panel-column`, `tool-framework`, `application-shell`;
  **no new capability** → **60** canonical specs after archive. Verified:
  `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --
  -D warnings` clean; `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` →
  `verify-fast: OK`; both self-tests exit 0 with all m42 lines `=1` and every
  earlier m20–m41 line unchanged; `openspec validate m42-panel-refinements
  --strict` valid and `openspec validate --all --strict` 61/61.

- **M43 — panel multicolumn** (a fifth user-requested interruption to the
  Layers-panel program; OpenSpec change `m43-panel-multicolumn`, brief
  `docs/dev/m43-panel-multicolumn.md`). **Phase A — drag/chrome/float.** A tab
  press drags/floats **only that panel** (a one-panel float built via `takePanel`
  + a fresh wired `PanelGroup`), an empty-header press drags/floats the **whole
  group**; both work docked and for a group already floating
  (`PanelColumn::beginPanelDrag`/`createFloat` payload-aware). The panel tab bar
  carries `objectName` `panelTabBar` and scoped QSS — selected tab `${base}`
  (identical to the `QTabWidget::pane`/widget background), unselected
  `${window}` (hover `${hover}`); the document bar is `documentTabBar`,
  unaffected (pixel sample: active `srgb(35,35,35)` = base, inactive
  `srgb(43,43,43)` = window). The corner `▾` is fixed with `ElideRight` +
  `setExpanding(false)` + the corner width added to `updateMinimumWidth`, so it
  is fully visible at the column minimum. `placeFlyout` uses the **actual button
  geometry** and the column's side, clamping only the inner coordinate so it can
  never flip outward or overlap the button. The Tools dock gets
  `setFixedWidth(content)` + `QSizePolicy::Fixed` horizontal + a `Resize`
  event-filter clamp (belt-and-braces for QMainWindow's internal splitter); the
  separator no longer resizes it, min == max == content width in 1- and 2-column
  modes and while floating, with the M42 float-height and the M40 standalone-dock
  contract intact. **`D`** resets fg/bg to default (black/white) through the
  existing `resetColors()`; the M42 `X` swap is kept (`D` was free — not in the
  tool catalogue). Compact icon buttons grew again (30 → 34 px, pixmap 24).
  **Phase B — multi-column host.** `centerSplitter_` now hosts an ordered set:
  left `PanelColumn`s, the document tabs (`objectName` `documentTabs`), right
  `PanelColumn`s; stretch stays on the tabs. `PanelColumn::side()` is derived
  from the splitter index vs `documentTabs` (not geometry). Columns are
  **created on drop** (`createPanelColumn(side)`) and **removed when empty**
  (`removeColumnIfEmpty`, dynamic-only), with the frame owning the wiring
  (`stateChanged`, `interfaceOptionsRequested`). `resolveDrop` gained a
  `DropKind` enum — `Reorder`, `IntoGroup`, `AboveGroup`, `BelowGroup`,
  `OnStrip`, `NewColumnLeft`, `NewColumnRight`, `Outside` — with a **28 px outer
  band / over-the-Tools-dock** rule for new columns, group top/bottom halves for
  above/below, tab-bar hits for into-group, cross-column `IntoGroup` delegation,
  and compact mode handled first so strip drags still reorder. One `#2a7fff`
  indicator marks every candidate. `flyoutSide()` is now `side() == Right ?
  "left" : "right"`. `Window → Panels` targets the owning column;
  `setPanelsHidden`/screen modes iterate all columns. **Phase C — session v6.**
  `panelColumns: [{side, order, groups:[{name, order, visible, minimized,
  collapsed}]}]` with `schemaVersion` **6**; the legacy flat `panelGroups` is
  still written as a mirror and a **v5 store loads as a single right-hand
  column** (synthesised when `panelColumns` is absent); load-then-write keeps
  unknown keys; the M42 `layoutRevision` guard and all v4/v5 keys survive.
  `applyPanelSession` rebuilds N columns and re-applies rail mode to all;
  `clearDynamicColumns` for re-apply. `m41_session`'s schema assertion changed
  from `==5` to `>=5` (the schema advanced — the only earlier-check change).
  **Self-tests** 140–151: `m43_tabdrag`(140), `m43_tabcolors`(141),
  `m43_corner`(142), `m43_newcolumn`(143), `m43_intogroup`(144),
  `m43_boundary`(145), `m43_singlefloat`(146), `m43_tools`(147),
  `m43_icon`(148), `m43_flyout`(149), `m43_dreset`(150), `m43_session`(151);
  all older codes green. **Honest limits:** cross-column drops delegate only
  `into-group` — a whole group dropped onto another existing column (not the
  outer edge) resolves as tear-off rather than a cross-column move; new columns
  are only allocated at the workspace outer edges or over the Tools dock;
  compact "above the first group" boundary is unreachable (the gap resolves as
  on-strip); the vertical group order **inside** a column is written but not
  re-applied on restore (pre-existing M41 behaviour); one workspace-wide
  `panelRailMode`/`railWidth`; float existence/position not persisted; only the
  left+right pair is self-tested (not multiple columns on the same side); the
  primary right column is never removable (an all-left layout leaves it present
  but empty); the Tools `setFixedWidth` clamp is verified under xcb only;
  `headerCornerWidthForTest` over-reserves when the `▾` is hidden. No Rust
  change: **588 tests, 0 failed, 7 ignored** (unchanged — no Rust change; the raw
  ignored count is 8 with the pre-existing `pictura-render` doctest).
  **Capability:** MODIFIES `panel-column`, `application-shell`, `tool-framework`,
  `workspace-persistence`; **no new capability** → **60** canonical specs after
  archive. Verified: `cmake --build` clean; both self-tests exit 0 (no-arg 5/5)
  with all m43 lines `=1` and every earlier m20–m42 line unchanged except the
  `m41_session` `>=5` schema assertion; `cargo fmt --all --check` and `cargo
  clippy --workspace --all-targets -- -D warnings` clean; `TASK_ALLOWS_DOCS=1
  bash scripts/verify-fast.sh` → `verify-fast: OK`; `openspec validate
  m43-panel-multicolumn --strict` valid and `openspec validate --all --strict`
  61/61.

- **M44 — panel/theme polish** (a sixth user-requested interruption to the
  Layers-panel program; OpenSpec change `m44-panel-theme-polish`, brief
  `docs/dev/m44-panel-theme-polish.md`). **Phase A — new-document canvas bug
  (E1).** On startup with no PSD the canvas showed the M0.5 GPU demo's repeating
  black/white/green/red banding instead of the white scratch document. Root
  cause: `PictureView::render_gpu` (the M0.5 GPU spike now in
  `cxxqt_object/impl_core.rs`)
  offscreen-rendered `crate::gpu::render_gradient` and **assigned it directly to
  `rust.image` without touching `rust.doc` or setting `display_dirty`**; startup
  creates a white scratch document, calls `render_gpu()`, then presents
  `view->image()`, so the canvas showed the gradient while the document composite
  stayed white — any later recomposite derives the display from `doc.composite`,
  which is why moving a layer turned it white. Fix: removed the image
  assignment; `render_gpu` is now a pure smoke probe (`Rendered { distinct, .. }`)
  that never mutates the display image (a pre-fix capture had 8634 distinct
  canvas colours, 1 after). New Rust regression
  `crates/pictura-render/tests/gpu_parity.rs::fresh_white_document_composites_uniform_white_twice`
  (37×23, CPU oracle + GPU first and second composite byte-equal white; self-skips
  without an adapter, **runs** on the RTX 3090). The startup check that previously
  asserted the gradient was non-blank became `fresh_white=1`; self-test **153**
  `m44_newdoc white=1 uniform=1 immediate=1`. **Phase B — theme/borders/style.**
  **W1** the collapse chevrons were inverted; swapped the `panel.columnsOne`/
  `columnsTwo` mapping so the toggle shows the correct target state (**154**
  `m44_chevrons`). **W2** the "last item active" default was
  **`PanelColumn::restorePanelState`**: replaying `PanelGroup::setPanelVisible`
  left the **last visible** tab current after a session restore (every normal
  launch after the first), fixed with `setCurrentToFirstVisible()` at the end of
  each group's restore (**155** `m44_defaultactive`). **W3** tab colours: the
  measured visible widget surface is `${window}` (`#2b2b2b`), not the QSS pane;
  active tab = `${window}`, inactive = `${base}` (`#232323`), panel groups get
  `QTabWidget#panelGroupTabs::pane { background: ${window} }`, document tabs
  unchanged (**156** `m44_tabswap`; the M43 `m43_tabcolors` direction corrected).
  **W6** group divider: splitter `setHandleWidth(kGroupDividerWidth = 6)` +
  `QSplitter#panelColumnSplitter::handle { background: ${border} }` (**159**).
  **C2** the compact-strip divider is now 2 px `${border}` (dark grey) instead of
  white (**161**). **C4** strip labels now **elide as soon as there is any room**
  (`QFontMetrics::elidedText`), replacing the fixed `>= 120 px` show/hide
  threshold (**164**). **F1/F2** document tab bar: `QTabBar#documentTabBar {
  border-right: 1px solid ${border}; border-top: 0 }` plus
  `QTabWidget#documentTabs::pane { border-top: 0 }` (**165**; F2 is a no-op at the
  sampled pixels — the only top line was the options bar's own bottom border).
  **S1** darker grey 1 px `${border}` on `#toolsPanel`, `#panelGroupTabs`,
  `#panelColumnIconStrip`, `#panelColumnContainer` (**166**). **S2** removed the
  inline `#panelFloat`/`#panelIconFlyout`/`#panelFlyoutHeader` stylesheets;
  docked, popup and floating now all take the `${window}` surface + `${border}`
  border from theme. Shared constants `Theme::kPanelBorderWidth = 1`,
  `Theme::kGroupDividerWidth = 6`. **Phase C — drag/dock.** **T1** the floating
  Tools dock height is now locked (both axes fixed while floating; the M40
  four-area dock contract is intact — `m40_dock` now asserts all four areas).
  **T2/W5** `CreatePanelColumn(side, anchor)` inserts immediately before/after an
  **anchor** column (not only the splitter ends); `columnEdgeAnchorAt()` yields a
  new-column target beside any `PanelColumn`; `newColumnSideAt` maps the Tools
  dock's side; the Tools dock uses `setAllowedAreas(AllDockWidgetAreas)` with a
  width-or-height lock per dock side. Same M43 `DropKind` resolver, one `#2a7fff`
  indicator (**158** `m44_docksides toolbar=1 column=1 workspace=1 float=1`).
  **W4** `createFloat` no longer cleaned the source group on a tab drag (the
  source tab bar owns the implicit mouse grab), so the float now **tracks the
  cursor until release**; the source is cleaned on commit/cancel (**157**
  `m44_floatdrag`). **C3** `resolveIconicDrop` proximity rule: icon hit → into
  that group; divider → new group at that boundary; inside a group container →
  on-strip/boundary; otherwise new group at the top/bottom end. Each group is a
  `panelIconGroup` container with a `panelIconGroupGrip` (`•••`) drag handle above
  its icons, and the container gets a `${base}` background/border so the icons
  read as one group (**162** `m44_compactdrop`, **163** `m44_draghandle`).
  **160** `m44_popupstyle parity=1` — the flyout shares the docked group's styling.
  **Self-tests** 153–166: `m44_newdoc`(153), `m44_chevrons`(154),
  `m44_defaultactive`(155), `m44_tabswap`(156), `m44_floatdrag`(157),
  `m44_docksides`(158), `m44_divider`(159), `m44_popupstyle`(160),
  `m44_compactdivider`(161), `m44_compactdrop`(162), `m44_draghandle`(163),
  `m44_elide`(164), `m44_filebar`(165), `m44_panelborder`(166); exit **152**
  remains the user's headless-platform check; all earlier m20–m43 lines unchanged
  except the corrected `m43_tabcolors` direction and the strengthened `m40_dock`.
  **Honest limits:** widget columns dock only left/right of another
  column/workspace (the host is a horizontal splitter) — top/bottom is supported
  for the Tools **dock** only; the compact "very close above/below ⇒ into" zone is
  the group container including the grip; multiple columns on one side with a
  column drag in flight remain best-effort; the M44 spec's parenthetical describes
  the pane as `${base}` while the measured surface/implementation uses `${window}`
  (the behavioural scenario holds); F2 is a no-op at sampled pixels; elide width
  is an approximate row allowance; the Rust regression pins the composite
  invariant but not the demo overlay itself (that is pinned by `m44_newdoc` + the
  startup `fresh_white` assertion). **Capability:** MODIFIES `panel-column`,
  `application-shell`, `tool-framework`, `document-canvas`; **no new capability**
  → **60** canonical specs after archive. Verified: `cmake --build` clean;
  `./build/pictura --headless --self-test` exit 0 with all `m44_*` `=1`; the PSD
  headless self-test exit 0; `cargo fmt --all --check` and `cargo clippy
  --workspace --all-targets -- -D warnings` clean; `cargo test --workspace`
  **589 tests, 0 failed, 7 ignored** (up from 588/7; the raw ignored count is 8
  with the pre-existing `pictura-render` doctest); `openspec validate
  m44-panel-theme-polish --strict` valid and `openspec validate --all --strict`
  61/61.

- **M45 — panel fixes** (a seventh user-requested interruption to the
  Layers-panel program; OpenSpec change `m45-panel-fixes`, brief
  `docs/dev/m45-panel-fixes.md`). **Phase A — Tools toolbar.** **T1 sizing:**
  `updateContentMetrics` previously fixed only one axis and read `sizeHint()`
  before the reflowed grid was active, while `topLevelChanged` latched a
  `floatHeight_`; the M43 width lock and M44 height lock could each keep a size
  from the previous column count, so 1↔2 read as 1-col ≈507 px and 2-col ≈862 px.
  Both axes now come from one content formula (`contentWidth` + a new
  `contentHeight` = title bar + margins + `rows*slot + gaps` + fg/bg + screen
  mode + body spacing), released and re-fixed after `layout()->activate()`;
  measured `w1=36 h1=862 w2=69 h2=507`, stable across the toggle. **T2:** the dock
  allowed areas revert to **left/right only** (M44 had `AllDockWidgetAreas`),
  keeping the M40 contract and the per-side lock. **T3:** the floating toolbar now
  docks **beside any widget column** via the existing `columnEdgeAnchorAt` + a new
  `PanelColumn::showEdgeDropIndicator(side)` (reusing the single `#2a7fff`
  indicator) and `commitToolboxDrop` (inserts at the anchor's splitter index); the
  toolbox title-bar drag emits `toolbarDragMoved`/`toolbarDragFinished`. It is
  hosted as a **central-splitter pane** (a `QDockWidget` cannot sit *between*
  columns); its title-bar re-float is not re-wired yet. **Phase B — widget
  panel.** **W1/W2/W3/W6 indicator correctness:** `DropTarget` gained
  `PanelColumn* owner`; `resolveDrop` sets it (`this` for local/workspace-edge/
  iconic, the **anchor** for beside-column, and the **destination column** for a
  delegated cross-column `IntoGroup`); `updateDrag` renders through the owner and
  clears the previous owner's line on change. Root cause: `resolveDrop` delegated
  the target but `showIndicatorFor` still ran on the **source column**, mapping
  the target's tab-bar x through the wrong `scroll_->viewport()` — so a
  right-hand target mapped off to the left (W1) or outside/clipped (W2). Tab
  inserts draw at `target.group->tabInsertionX(target.tabIndex)` (W3) and bottom
  boundaries at the last visible group's bottom edge (W6), in the owning column.
  `kEdgeInside` reduced 6→0 so an inside-edge tab insert is not mistaken for a
  new-column anchor. **W4 emptied column:** one `PanelColumn::maybeRemoveSelf()`
  (calls `frame->removeColumnIfEmpty(this)`) runs from `commitDrop`, `cancelDrag`,
  `closeGroup`, `showPanel(name,false)`, `restoreFlyoutPanel`;
  `removeColumnIfEmpty` **rehomes still-live groups into the primary column** via
  the new `PanelColumn::adoptGroup` (so panels survive for a later Window-menu
  show) and keeps a column that still owns a float. **W5 minimize actually
  collapses:** `PanelGroup::applyMinimize` clamps the **group's** `maximumHeight`
  (and size policy) to the tab-bar height, saving/restoring
  `savedGroupMaxHeight_`; the tab menu entry is state-derived — **"Expand Panel"**
  while minimized, "Minimize" otherwise (`tabMenuActionsForTest` applies the same
  substitution). **W7 never clip:** the scroll area's horizontal policy changed
  `AlwaysOff → ScrollBarAsNeeded`, tab text elides, the corner button keeps its
  reserved width. **W8 shared floor:** one `constexpr int kPanelMinWidth = 180`
  for every normal-mode column (the compact strip keeps `kIconStripMinWidth = 40`),
  replacing the per-column widest-derived floor, so columns share the floor and
  none can vanish. **Phase C — compact parity.** **C1 the popup is a real
  group:** `ensureFlyout` no longer builds a bespoke one-tab header;
  `openIconFlyout` finds the group, `PanelGroup::setCurrentPanel(clicked)`,
  records its index, and reparents the **whole `PanelGroup`** into the popup (it
  stays in `groups_`); `restoreFlyoutGroup` inserts it back at the recorded
  splitter index exactly once, guarded by `restoringFlyout_`/`flyoutGroup_`.
  Docked, popup and floating are now the **same widget instance** (same tabs, `▾`
  menu, minimize, drag, styling). **C2 group-drag line:** in compact mode a
  whole-group drag (`!dragIsPanel_`) anchors `stripIndicator_` to the target
  group's **container top −1** (above the `•••` grip dots), not the first icon
  button; panel drags keep the M42/M44 anchoring. **Self-tests** 167–179
  (`m45_tools_sizing` 167, `m45_tools_sides` 168, `m45_tools_beside_column` 169,
  `m45_indicator_side` 170, `m45_indicator_cross_column` 171,
  `m45_indicator_rightmost_tab` 172, `m45_empty_column_removed` 173,
  `m45_minimize_collapse` 174, `m45_indicator_bottom` 175, `m45_no_clip` 176,
  `m45_min_width_floor` 177, `m45_popup_group` 178, `m45_compact_group_line`
  179); exit **152** remains the headless-platform check. **Honest limits:** the
  toolbar is hosted as a central-splitter pane, so it cannot sit *between* columns
  as a dock and its title-bar re-float is not re-wired yet; a narrow column's
  group content can still scroll (W7's stated trade-off); `removeColumnIfEmpty`
  deliberately does not tear down a column that owns a live float; drag *from
  inside* the popup is not automatically tested; `PanelGroup::detachPanel`/
  `attachPanel`/`detached_` are now unused (left in place) and `theme.cpp`'s
  `panelFlyoutHeader` selector is dead (untouched); C2 falls back to the last strip
  box when the boundary maps to a group hidden from the strip; chosen constants
  remain unsourced CS6 metrics. No Rust change: **589 tests, 0 failed, 7 ignored**
  (unchanged; the raw ignored count is 8 with the pre-existing `pictura-render`
  doctest). **Capability:** MODIFIES `panel-column`, `application-shell`,
  `tool-framework`; **no new capability** → **60** canonical specs after archive.
  Verified: `cmake --build` clean; `./build/pictura --headless --self-test` exit 0
  with all `m45_*` `=1`; the PSD headless self-test exit 0; `cargo fmt --all
  --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `cargo test --workspace` **589 tests, 0 failed, 7 ignored**; `openspec validate
  m45-panel-fixes --strict` valid and `openspec validate --all --strict` 61/61.

- **CI headless and build speed** (infrastructure, not a milestone; archived
  OpenSpec change `ci-headless-and-speedup`). `main.cpp` gains an explicit **`--headless`**
  flag: it selects the offscreen QPA plugin before `QApplication` (when
  `QT_QPA_PLATFORM` is unset), wins over the `--self-test` xcb override, and
  implies `--self-test` when no document is given so it never blocks in
  `app.exec()`. The self-test asserts `QApplication::platformName() ==
  "offscreen"` and exits `152` otherwise. `--interop-probe` is excluded (it
  needs a real platform Vulkan instance); explicit `QT_QPA_PLATFORM=offscreen`
  and `xvfb-run` still work. Build speed: a new `.cargo/config.toml` links with
  `lld`, `[profile.test]` compiles dependencies at `opt-level = 0` with the
  workspace crates pinned at `2`, and CMake uses Ninja + `--parallel` with the
  self-test run headless. CI is rebuilt into three jobs — `rust` (fmt, clippy,
  nextest, doctests), `qt-headless` (pinned Qt 6.11.1 + CMake/Ninja +
  `--headless --self-test`), and `oracles` (ImageMagick + `psd-tools`, full
  `cargo test --workspace`) — with registry/sccache caching. No Rust API,
  document, codec, compositor, or dependency change: **588 tests, 0 failed, 7
  ignored** (unchanged).

- **Post-M47 cleanup pass** (review follow-up; no OpenSpec change, no new
  capability, canonical specs unchanged). **Bug fix — document tab reorder:** the
  document `QTabWidget` was movable but nothing connected `QTabBar::tabMoved`, so
  dragging a tab left `docs_` in the old order while `tabs_->currentIndex()`,
  `viewAt` and `removeDocument` indexed the new one — the wrong document became
  active, closed, or returned. `frame.cpp` now connects `tabMoved(from,to)` to
  `docs_.move(from,to)` (valid-index guarded); Qt keeps the dragged tab current,
  so the active document is unchanged. Regression: self-test exit **196**
  `doc_tab_reorder aligned=1` (`PicturaMainWindow::reorderDocumentsForTest`,
  which also exercises `viewAt`/`documentName`/`activeDocumentIndex` after the
  move). **Dead-code deletions:** `PanelGroup::detachPanel`/`attachPanel`/
  `removePanel`, `PanelColumn`'s `removePanel`, the unused `blendName`,
  `activeForeground_`, a no-op provider, the dead `panelFlyoutHeader` QSS
  selector, and three unused panel test hooks. **Dedupe (pure moves to one
  home):** the duplicated filter math helpers hoisted into
  `kernel`/`luma`/`texture` (bit-identical, oracle-verified), the frame TU
  include block shared via `frame_includes.h`, the shared `storage_entry` in
  `pictura-render::gpu`, and the bridge `mutate_layer`/`reset_edit_state`
  helpers. **Hardening:** the panel-drop path no longer orphans/leaks a lifted
  panel when no column can be created (`applyNewColumnDrop` re-inserts it into
  its home group), session reads are capped at 1 MiB, thumbnail size math
  widened, temp files cleaned up on save failure, over-cap layer channel counts
  rejected by the codec writer, filter coverage honours disabled masks, and
  NaN/overflow guards added in `pictura-adjust`/`pictura-filters`/
  `pictura-select`. **Kept despite review flags** because they are spec'd or
  planned: `history_depth`, `move_layer`, `layer_thumbnail`, `file_path`,
  `resize_canvas`, `rotate_arbitrary`, `composite_gpu_or_cpu`, `parent_path`,
  `hash_bytes`, `Layer::is_group`, `pictura_color::assign`, the whole
  `pictura-color` crate, and `move_preview`. Verified: `cmake --build` clean;
  `./build/pictura --headless --self-test` exit 0 with `doc_tab_reorder=1`;
  `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D
  warnings` clean; `cargo test --workspace` **592 tests, 0 failed, 9 ignored**
  (up from 589/7; the raw ignored count rose 8→9 with the newly-ignored M25
  `filter_profile_1024`).
- **Post-M47 cleanup pass, part 2** (follow-up to the above; commits
  `f0b3205`, `3dabab9`). `pictura-filters/src/filter.rs` (919 LOC) split into a
  facade plus `filter/types.rs` (`Filter` enum) and `filter/apply.rs`
  (`apply()`); `Selection::combine_with` now mutates in place and returns `()`
  (the clone-return was unused by its sole caller); the panel tab-menu labels
  come from one `tabMenuTexts(bool)` source instead of a duplicated array;
  `gSharedFloor` is recomputed from live columns instead of only growing (M45
  shared-floor semantics preserved); the layers row build is two-pass so a
  forward-referenced parent resolves instead of silently rooting; and the
  write-only `PanelGroup::defaultIconic` / `PanelColumn::isDynamic` flags are
  gone. All self-test stderr unchanged (code 196 unaffected).

- **layers-panel-controls** (the Layers-panel program's controls stage, named by
  content; OpenSpec change `layers-panel-controls`, archived). Opacity and Fill
  became percentage controls: a reusable `PercentField`
  (`crates/pictura-app/cpp/panels/percent_field.{h,cpp}`) with a text box, a
  popup slider, and drag-on-label scrubbing, converted at the view boundary with
  `round(pct*255/100)`; the bridge keeps reporting and editing `0..=255`, so no
  stored-format change. The lock strip became five icon toggles — alpha, paint,
  position, nesting, full — and `LockFlags` gained `NESTING = 0x08`, with
  `all()`/`is_all()` moving to the four-bit `0x0F`. The `lspf` reader mask
  widened to `& 0x0F` and `lock_from_bits` sets the fourth flag, so the PSD
  round-trips; `lock_bit("nesting")` reaches the bridge. A nesting-locked layer
  keeps its structural parent: `group_paths` refuses and `ungroup_paths` skips
  it, while within-container `move_path` still reorders. The row delegate paints
  a clipping-mask glyph (`layers.clipMask`) for a clipped layer. Six independent-creation
  SVGs were added and the qrc regenerated. Ceiling: a PSD whose `lspf` is the old
  three-bit `0x07` is no longer treated as fully locked (`all` now requires
  `0x0F`); `Lock All` sets all four bits. Self-tests `lpc_percent` (199) and
  `lpc_nesting` (200) live in a new `selftest_layers_controls.cpp` so
  `selftest.cpp` stays at its 6730 allowance. Verified: `cargo nextest run
  --workspace` **594 passed, 8 skipped**; `TASK_ALLOWS_DOCS=1 bash
  scripts/verify-full.sh` → `verify-full: OK` (TOTAL 768 passed · 9 skipped · 0
  failed, file-size OK, guard OK, `openspec validate --all --strict` 61/61).
  Capability: MODIFIED `layers-panel`; no new capability.

- **layers-filtering-search** (the Layers-panel program's filter/search stage,
  named by content; OpenSpec change `layers-filtering-search`, archived). The
  panel gained the CS6 filter/search row above the blend/opacity header: a
  dimension popup (Name, Kind, Effect, Mode, Attribute, Color; default Kind), a
  criteria stack, and an on/off switch. A new `LayersFilterProxyModel`
  (`layers_filter_proxy.{h,cpp}`) wraps the tree model and accepts a row when it
  matches or when any descendant matches (ancestor promotion); Name is a
  case-insensitive substring, Kind a multi-select over the model's kinds, Mode a
  blend key, Color a label index, and Attribute one of Visible/Hidden/Locked/Has
  Mask/Clipped; active criteria AND and Kind values OR. The Effect dimension is
  present but disabled until layer styles exist. The panel now uses the proxy as
  its view model, with `proxyIndexForPath`/`pathForProxyIndex` mapping selection,
  expansion, rename, and eye hit-tests; activation auto-expands promoted groups
  and toggling off restores the prior expansion. Filtering is view-only — no
  history state, never serialized — and resets to Kind/off on a document switch.
  New `layers_filter_bar.{h,cpp}`; the shared blend table moved to
  `layers_panel_internal.h`; theme QSS added. Self-tests `lfs_name` (201),
  `lfs_kind` (202), `lfs_mode` (203), `lfs_color` (204), `lfs_none` (205),
  `lfs_ancestor` (206), `lfs_toggle` (207), `lfs_live` (208), `lfs_reset` (209)
  live in a new `selftest_layers_filter.cpp`, keeping `selftest.cpp` at 6730.
  Ceiling: Kind offers only the kinds the model has (pixel, adjustment, group,
  background) and Effect is inert until the styles stage; the ancestry scan is
  O(subtree) per row (`// ponytail:` in the proxy). Verified: `TASK_ALLOWS_DOCS=1
  bash scripts/verify-full.sh` → `verify-full: OK` (TOTAL 777 passed · 9 skipped
  · 0 failed, file-size OK, guard OK, `openspec validate --all --strict` 62/62).
  Capability: ADD `layers-filtering-search`; no Rust change.

- **layers-panel-chrome-fixes** (a UI-correction follow-up to the controls and
  filter stages, named by content; OpenSpec change `layers-panel-chrome-fixes`,
  archived). **Header order/labels:** the header now stacks the filter row, then
  `[blend | Opacity label + field]`, then `[five locks | Fill label + field]`,
  then the layer list, then the action strip, and the `PercentField` popup opens
  centred under its field. **One menu entry point:** the panel's own Qt menu
  button (`layersPanelMenu`) was removed; the panel-group widget menu (`▾`) is
  the single path for the wired Layer commands, and the M39 menu self-test now
  asserts only the row and colour menus. **Left-anchored rows:** a new
  `LayersTreeView` suppresses the stock branch indicators; the tree runs at zero
  indentation and the delegate anchors the eye at the panel's left edge for every
  row, indenting the thumbnail/name by depth and drawing an expand/collapse
  chevron for a group (the panel's event filter toggles expansion on a chevron
  click, like the eye). **Filter lightswitch:** two original SVGs
  (`layers.filterOn/Off`) back an icon toggle that starts **on**; with no
  criterion the filter is inert, and the auto-expand-on-filter only runs when a
  criterion is actually active (`hasActiveCriteria`), so an enabled empty filter
  never expands every group. Self-test `lpc_chrome` (210) covers the order,
  labels, removed menu button, on/with-icon toggle, left-anchored eye, and the
  chevron click; the filter-reset test still passes with the reset now Kind/on.
  Ceiling: the kind toggle labels elide at the narrow default width. Verified:
  `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh` → `verify-full: OK` (TOTAL 778
  passed · 9 skipped · 0 failed, file-size OK, guard OK, `openspec validate --all
  --strict` 62/62). Capability: MODIFIED `layers-panel` (+1 requirement, ~1) and
  `layers-filtering-search` (~2); no Rust change.

- **layers-panel-row-interactions** (a third UI pass on the Layers panel, named
  by content; OpenSpec change `layers-panel-row-interactions`, archived).
  **Percent fields:** `PercentField` now owns its text label and a `%` suffix,
  and pressing/dragging the label or the `%` scrubs the value like the field.
  **Semantic lock icons:** only `layers.lockAll` is a padlock; alpha is a
  transparency checkerboard, paint a brush, position a move cross, nesting
  nested squares. **Kind filter icons:** the Kind toggles are icon-only
  (`layers.kindPixel/Adjustment/Group/Background`) with the kind name as
  tooltip. **Eye and disclosure:** the visibility toggle is an eye SVG
  (`layers.eyeOn/Off`) drawn slightly inset (`kEyeInset = 6`), and a group shows
  `layers.disclosureRight`/`Down` at its indented position. **Drag and drop:**
  `LayersTreeView` enables drag/drop, overrides `startDrag` to carry the current
  row's path in `application/x-pictura-layer`, and resolves a drop to a target
  path plus a mode (above/below/into); the panel's handler calls the new
  `move_layer_to`. The path math lives in `pictura_render::move_path_to` (mode
  0/1/2) with refusal rules for the Background, fully/nesting-locked sources, a
  self or descendant drop, and an `Into` target that is not a group; the bridge
  recomposites and records one `"Move Layer"` state. Dragging no longer starts a
  rubber-band multi-selection, and dropping a row on the Delete / New Layer /
  New Group strip buttons runs delete / duplicate / group on the dragged paths
  (the buttons are drop targets with a `layerDropAction` property); mask/link/fx
  stay inert. Self-tests `lpr_rows` (211), `lpr_drag` (212), `lpr_drop` (213)
  plus the engine unit test `move_path_to_reparents_and_refuses`. Ceiling: only
  the current row is dragged (a multi-selection drag moves one row; marked
  `// ponytail:`). Verified: `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh` →
  `verify-full: OK` (TOTAL 779 passed · 9 skipped · 0 failed, file-size OK, guard
  OK, `openspec validate --all --strict` 62/62); `cargo nextest run --workspace`
  **595 passed, 8 skipped**. Capability: MODIFIED `layers-panel` (+2, ~1) and
  `layers-filtering-search` (~1); no new dependency.

- **layers-panel-control-polish** (a fourth UI pass; OpenSpec change
  `layers-panel-control-polish`, archived). Four fixes. **Eye only:** the model
  stops reporting `Qt::CheckStateRole`, so a row paints no native checkbox next
  to the eye; the eye stays the sole visibility control. **`%` inside:** the
  PercentField suffix is reparented onto the value `QLineEdit`, a right text
  margin is reserved, and it is repositioned on resize, so the sign sits in the
  box. **Lock badge:** `LayerRowDelegate::lockRect` plus a `layers.lockAll` badge
  painted at the row's right edge (before fx/mask) whenever `LockRole != 0`.
  **Preview vs commit:** new bridge `preview_layers_opacity`/`commit_layers_opacity`
  and `preview_layers_fill`/`commit_layers_fill` (new `opacity_preview_changed`/
  `fill_preview_changed` flags) mirror the move-tool split; `PercentField` emits
  `valueChanged` during a scrub/slider drag and one `valueCommitted` on release
  (or text `editingFinished`, or popup hide), so a drag previews live and adds a
  single undo state. `set_layers_*`/`set_layer_*` are untouched. Ceiling: the
  preview flag is sticky, so a drag that returns exactly to its start value still
  records one redundant state (marked `// ponytail:`). Self-tests `lpc_preview`
  (214), `lpr_percent` (215), `lpc_lockbadge` (216), `lpr_eye` (217) in
  `selftest_layers_controls.cpp`. Verified independently: `TASK_ALLOWS_DOCS=1 bash
  scripts/verify-full.sh` → `verify-full: OK` (TOTAL 786 passed · 9 skipped · 0
  failed, file-size OK, guard OK, `openspec validate --all --strict` 63 items
  while the change was open); `cargo nextest run --workspace` **595 passed, 8
  skipped**; app self-test **153 passed, 0 failed**. Capability: MODIFIED
  `layers-panel` (4 requirements). Follow-up fix: the inside-`%` field was too
  narrow (`setFixedWidth(34)`) and clipped `100` to `0`; the edit is now sized
  from the font metrics for `100` + `%`, and `lpr_percent` asserts the fit. The
  `layers.eyeOn/Off` art was redrawn as a bolder Lucide-style eye for 14–20 px
  and `LayerRowDelegate::paintAsset` now renders a square, device-pixel-ratio-
  aware pixmap so the eye and chevron stay crisp. The popup slider is now a
  `JumpSlider` that jumps to the clicked point and tracks the held cursor, since
  the stock `QSlider` only page-steps; `lpr_slider` (218) covers it.

## Canvas viewport & performance (post-M24 pass)

Not an OpenSpec capability — a correctness/performance pass; the intended
behaviour (move-tool behaviour, budget, suspected bottlenecks, acceptance
checks) is written up in `docs/dev/canvas-view-spec.md`.

- `image_view.{h,cpp}`: `setImage` now fits-and-centres (fit when the image
  exceeds the viewport, else 100 % centred) and re-applies that initial view on
  resize until the user pans/zooms; middle-button drag pans the canvas
  regardless of the active tool; `fitOnScreen`/`actualPixels` re-arm the
  initial view.
- `frame.{h,cpp}`: `refresh()` keeps the canvas/status/menu updates synchronous
  but defers the expensive panel refresh (`retargetDock`) behind a single-shot
  120 ms `QTimer`, so a burst of `changed` signals no longer blocks the canvas
  repaint; tab add/remove/switch force an immediate panel refresh.
- `cxxqt_object/impl_transform.rs` + `tools.cpp`: the Move tool previews live **without
  compositing during the drag**. `begin_move_preview()` caches a base image (the
  document composited with the moved topmost raster layer hidden), the layer's
  own image, its document-space top-left, and its opacity; `move_preview_base/
  layer/x/y/opacity` and `end_move_preview()` expose/clear it.
  `ImageView::beginMovePreview(base, layer, layerPos, opacity)` +
  `setMovePreviewDelta(delta)` + `endMovePreview()` draw the cached base then the
  moved layer at the live Qt delta (source-over with `setOpacity`) in
  `paintEvent`. `ToolController` Move press seeds the preview, move updates only
  the delta, and release calls `end_move_preview` → `commit_move(dx, dy)` once
  (a single composite, exactly one "Move Layer" history state) →
  `endMovePreview`; switching tools or rebinding the canvas cancels it. The old
  per-event `move_preview(dx, dy)` is retained only for the self-test
  (`// ponytail: slow path`). One drag = one history state; undo restores the
  pre-drag pixels.
- Move-tool root cause, measured: each mouse-move previously ran `move_preview`
  → `pictura_render::translate_layer` (full composite) plus `document_to_image`
  (a **second** full composite) and a full planar→RGBA conversion. A single
  1024×1024 two-layer `composite_rgba` measures ~**257 ms in the debug build**
  (the old CMake default) and ~**39 ms in the optimized build** (~6.6×). The one
  remaining cost is the single commit composite on mouse-up; the live preview is
  source-over only (non-Normal blend modes, masks, and clipping are not
  reproduced mid-drag, but the committed image is exact).
- `CMakeLists.txt` now defaults `CMAKE_BUILD_TYPE` to `RelWithDebInfo` when
  unset; Corrosion maps any non-Debug config to cargo `--release`, so the Rust
  crate is built optimized too (verified: `build/libpictura_app.a` is the release
  artifact).

Self-test exit codes 64–67, measured identically on fixture and no-argument
runs: `canvas_centre offset=(270.691, 5) zoom=1`; `canvas_middle_pan
delta=(30,15)`; `canvas_move preview=1 hist=1 undo=1`; `canvas_preview_cache
began=1 base=1 layer=1 hist_unchanged=1`. These labels were renamed `m25_*` →
`canvas_*` during M25 so the `m25_` prefix belongs to the actual milestone; the
exit codes are unchanged. Gates green:
`cmake --build build`; both self-tests exit 0; `cargo fmt/clippy/test` clean
(no new Rust tests; the app crate stays green). The headless self-test cannot
measure frame timing, so the interactive feel (pan/zoom/move latency on large
documents) still needs a real GUI check.

### Canvas view performance — CPU display pyramid (change `canvas-view-performance`)

Implemented `openspec/changes/canvas-view-performance`. This adds a CPU **display
pyramid** in `pictura-render::ViewPyramid`, distinct from the deferred M38
GPU-tiles/proxy-compositing track. The pyramid stores no level 0 — it borrows a
planar straight-alpha level-0 view per call; the app supplies that from a cached
full-resolution **sRGB** `level0` frame (color-managed from `doc.composite`, and
also the display image's source), so the common RGBA path adds one full-resolution
buffer over `doc.composite`. Levels below 0 are stored premultiplied and halve to
a 256 px long side. The
canvas (`cpp/image_view.cpp`) picks the coarsest level still at least as fine as
the screen and blits a premultiplied crop
(`PictureView::display_image(level, x, y, w, h)`), so a pan or hover repaint never
rescales the full-resolution document. The 64 MP present-cache cap and the
full-resolution `drawImage(image_)` fallback are gone; sampling is smooth below
200 % and nearest at and above it. `CanvasDamage` (`edited` / `mark` / `take`)
replaces the `display_dirty` bool, preserving `regionBlitted` / `changed` and the
no-`changed`-on-a-region-refresh contract. The CPU composite fallback is
row-parallel (`crates/pictura-render/src/composite_rows.rs`, `rayon`). The display
conversion emits `Format_RGBA8888_Premultiplied`; the shared straight-alpha
export/thumbnail helpers are byte-identical. The navigator draws from a pyramid
level and adds no second full-resolution copy.

Memory at 16000² (measured, `crates/pictura-render/examples/mem_probe_16k.rs`):
one full-resolution level-0 frame plus halved levels peak ≈ **2.0 GB**; storing a
premultiplied level-0 copy as well would cost ≈ **3.0 GB**. The probe covers the
document and pyramid buffers, not the display `QImage` (`rust.image` /
`ImageView::image_`). This is display-only — it caches the full composite and
does not composite layer proxies.

Exit code **541** (`canvas_present_contract`) pins level-crop reuse across a pan,
level-0 crop identity against a full-resolution transform draw, and no `changed`
on a region refresh; **539**/**540** pin the filter/level formula and the
navigator level source. Profile-only `scroll_zoom_pan_profile_4000` /
`_16000` time the crop against a full-resolution rescale.

The 16000² raster-import prerequisite landed here too: `pictura_codec`'s default
`ImageBudget` allocation cap rose from 512 MiB to **2 GiB** and the Qt decode edge
raises `QImageReader`'s 256 MB allocation limit to **4 GB**
(`crates/pictura-app/cpp/decode_image.cpp`), so `World Map [16,000 x 16,000].png`
(16507×16196×4 ≈ 1020 MiB) opens. Both caps were below the target, so the file was
refused before the decoder ran. The 30 000 px dimension cap is unchanged.

### Large-document interactive performance (change `large-document-interactive-performance`)

Implements `openspec/changes/large-document-interactive-performance` (issue #130),
the follow-up to the canvas-view work. Interactive present, operation refresh,
panels, ICC, and decode no longer pay whole-document costs:

- **Mid-stroke present.** `ImageView::presentCrop` no longer bails while
  `is_painting()`; a live stroke presents the zoom level crop like the idle
  canvas. The pyramid already carried the stroke's dabs (`refresh_region` patches
  level 0 and repairs levels), so nothing else was needed. `reset_pyramid` now
  composites a live stroke's working document when a stray full rebuild happens,
  and undo/redo/history jump/restore cancel an in-progress stroke.
- **Described-change coverage.** `end_paint`/`cancel_paint` refresh the stroke's
  extent; `mutate_layer` (opacity/blend/fill) refreshes the layer's bounded
  influence rect; `apply_filter`/`apply_pictura_raw` the target layer rect;
  `add_adjustment` the mask rect when bounded; clipboard clear/paste the
  affected/new layer rect; `translate_layer`/`move_preview`/`commit_transform`
  the union of source and destination. Bounded ops emit `region_blitted` (not
  `changed`), so no panel read forces a full image rebuild. Selection-only edits
  no longer run the compositor. See the "Described-change coverage" section of
  `docs/dev/canvas-compositing-plan.md`.
- **One full-resolution display frame.** The cached `PictureViewRust::image`
  `QImage` is gone; `PictureView::image()` builds on demand from the level-0
  frame. The sRGB `level0` frame stays because the pyramid needs it for regional
  updates.
- **Panels and codec.** The Histogram bins a view-pyramid level (≤512), so it no
  longer builds the full image. `buffer_to_srgb` reuses a cached working-profile
  transform and borrows the buffer (no copy) for an unprofiled document. The Qt
  decode edge copies the decoded frame with one bulk `copy_bytes` bridge call
  instead of a per-byte `push_back` loop.

New self-test checks **542** (`ldoc_selection_no_recomposite`) and **543**
(`ldoc_paint_presents_from_crop`) live in `cpp/selftest_large_doc.cpp`. The
history copy-on-write / tile-diff rework (M36) remains deferred.

### Brush stroke latency and copy-on-write planes (`brush-stroke-latency`, `cow-pixel-storage`)

`brush-stroke-latency` hoisted the stroke-constant tip profile (`TipParams`)
and the layer's plane positions (`PlaneIndex` / `Stencil`) out of the per-pixel
loop, and made the in-stroke present frame-bounded (`queue_present` /
`flush_present`, self-test **544** `pp_present_flush`). `cow-pixel-storage` made
`Channel::data`, `PixelBuffer::data` and `LayerMask::data` a refcounted
`pictura_core::Plane`, so `Document::clone` — and therefore stroke start and
every history snapshot — is a refcount bump (0.004–0.008 ms at 4000² against the
81/360 ms it cost); a write forks only the plane it touches. Measured brush-dab
split is in `docs/dev/canvas-view-spec.md` § 3.4. A round tip's rasterization now
walks the row chord rather than the bounding box's corners (`TipParams::x_span`).

### Large-brush LOD preview (`stroke-lod-preview`)

Above the 262 144 px raster budget (⌀≈512), the in-progress stroke presents from
a stored view-pyramid level instead of full resolution: `preview_dab` rasterizes
the accumulated dabs' coverage at level 3 (1/8) over a snapshot of that level
taken at stroke start, `preview_present` patches the level, emits `regionBlitted`
(so the canvas repaints and the frame-flush chain runs), and `ImageView` forces
the crop to that level. `end_paint` replays the logged samples through a real
full-resolution `Stroke` before `record`, so the committed pixels are exact;
`cancel_paint` and `reset_pyramid` drop the preview. Self-test **545**
(`pp_large_preview`) pins it; details in the "Described-change coverage" section
of `docs/dev/canvas-compositing-plan.md`.

### Staged next steps (not implemented)

- **`gpu-stroke-rendering`** (landed): `begin_paint` seeds a resident RGBA8 GPU
  buffer (`pictura_render::gpu::GpuStroke`, one word per pixel) from the *target
  layer* at its document rect, with an immutable `base` copy and a per-pixel
  coverage plane. Each dab is a compute dispatch over its bounding box that
  accumulates the CPU's 8-bit coverage and recomposites each changed pixel from
  the base (so overlapping dabs match the CPU instead of compounding),
  returning only the dab's changed rectangle. The app writes that rectangle into
  the exact `Stroke`'s working document (`Stroke::patch_working_layer`) and
  presents frame-bounded through `refresh_region`, which composites from that
  document; `end_paint` commits it directly, so there is no release-time replay.
  The committed pixels equal the GPU stroke and lie within ±1 LSB of the CPU
  oracle. Selected when "Use Graphics Processor" is on, the mode is Normal or
  Clear, `auto_erase` is off, the target layer is not transparency-locked, the
  dab is over the 262 144 px raster budget, and an adapter exists; below the
  budget the exact CPU path runs (a small dab costs ~0.5 ms, less than the ~65 ms
  full-frame GPU seed measured at 4000²), and above it with no adapter the LOD
  preview runs. Evidence: `gpu_stroke_matches_the_cpu_oracle` (ignored GPU test,
  ±1 LSB over a placed overlapping stroke) and self-test 546 `pp_gpu_commit`
  (commit within ±1 LSB), which skip without an adapter. Task list:
  `openspec/changes/gpu-stroke-rendering/tasks.md`.
- **Tiled storage / region-delta history (M36/M38)**: 64–128² tiles with
  region-scoped history, so a heavily-edited layer's history states share
  unchanged tiles. Still needs content versioning.


## Spec workflow (OpenSpec)

OpenSpec is the per-change requirements layer over `docs/`. See `AGENTS.md`
"Spec workflow (OpenSpec)". M0–M34 are archived; `openspec/specs/` is now the
canonical contract, with the per-change history under
`openspec/changes/archive/`. New work starts as a new change under
`openspec/changes/` (not as code), with `proposal.md`, `design.md`, `tasks.md`,
and `specs/<capability>/spec.md` deltas, archived into `openspec/specs/` when
complete.

## Conventions (keep doing)

- Task briefs live in `docs/dev/m*-*.md`; docs changes need a commit message
  containing `TASK-ALLOWS-DOCS` or `TASK_ALLOWS_DOCS=1` for `guard.sh`.
- Each milestone: freeze interfaces → dispatch 2–3 `general` sub-agents on
  **disjoint files/crates** → orchestrator integrates, un-ignores oracle tests,
  verifies, commits. Never let an implementer verify its own work without an
  independent oracle (psd-tools / ImageMagick / the app self-test).
- Oracles: don't fake tolerances. Where ImageMagick/Photoshop semantics diverge,
  reclassify as "no faithful equivalent" and use property/known-value tests.

## Next: panels program (remaining stages), canvas perf series deferred

### Panels program — panel/column/toolbar work done; layer filtering/search next

The CS6 Layers panel program's research, gap analysis, and staged plan live in
`docs/dev/layers-panel-program.md`. **M36 — layer attributes end-to-end** (change
`openspec/changes/m36-layer-attributes`) and **M37 — layer creation and grouping**
(change `openspec/changes/m37-layer-creation`) are implemented and verified (see
the milestone entries above): M36 added `Layer.fill`/`lock`/`color`, `opacity ×
fill` compositing on CPU and GPU, `lspf`/`lclr`/`iOpa` PSD I/O, the bridge
getters/setters, and the Fill/lock/color panel controls; M37 added
`document_ops::layer_ops` New Layer / New Group / Duplicate / Group / Ungroup, the
bridge methods, the panel buttons and the five `Layer` menu commands.

**M39 — panel anatomy** (change `openspec/changes/m39-panel-anatomy`, brief
`docs/dev/m39-panel-anatomy.md`) is implemented and independently verified (see
the milestone entry above): the full expandable layer tree, the frozen layer-path
grammar and depth-first topmost-first projection, the path/batch bridge API,
the `QAbstractItemModel` tree + delegate row anatomy, multi-selection with the
per-node refusal table, solo visibility, `Tab` rename, Panel Options (session
schema v3), the panel/row menus, tooltips, and the explicit drag-reorder
deferral to M47.

**M38 was a user-requested interruption: the full CS6 toolbox icon/cursor
library and the panel icons** (`openspec/changes/m38-icon-cursor-library`,
contract `docs/dev/m38-icon-cursor-library.md`) — the frozen 71-tool catalogue,
the full icon/cursor asset set, the single-column flyout toolbox, and the
panel/Layers/History icons. It is implemented and verified (see the milestone
entry above); it took the M38 number, so the panel stages shifted by one:
**M39 — panel anatomy** is done (see the milestone entry above).

**M40 was a second user-requested interruption — the CS6 Tools panel**
(`openspec/changes/m40-tools-panel`, brief `docs/dev/m40-tools-panel.md`): the
custom lower-right flyout triangle, hold/right-click flyout with shortcut keys,
the one/two-column double-arrow toggle, the standalone dock (left/right only, no
tab groups), and the generic `Shift`+letter group cycling gated by a session
`Use Shift Key For Tool Switch` preference (session schema v4). It is
**implemented and independently verified** (see the milestone entry above).

**M41 was a third user-requested interruption — the CS6 panel column**
(`openspec/changes/m41-panel-column`, brief `docs/dev/m41-panel-column.md`):
content widgets moved under a `PanelColumn`: in normal mode vertical tab groups
with the tabs explicitly on top, in compact mode an icon strip with group
dividers, labels-on-widen, and `Qt::Popup` flyouts that close on click-away; the
`Auto-Collapse Iconic Panels` preference defaults **off**, the double-chevron
toggle, fixed CS6 Essentials groups, and the hard panel minimums removed with a
scroll so the window resizes freely; session **v5** adds
`panelRailMode`/`railWidth`/`autoCollapseIconic`, and the Preferences dialog
(General + Interface panes) gives `Use Shift Key For Tool Switch` and
`Auto-Collapse Iconic Panels` a UI — M40's previously UI-less `Use Shift Key For
Tool Switch` now has its home there. It is **implemented and independently
verified** (see the milestone entry above). **M42 was a fourth user-requested
interruption — panel refinements** (`openspec/changes/m42-panel-refinements`,
brief `docs/dev/m42-panel-refinements.md`, research
`docs/dev/m42-panel-menus.md`): the compact-strip drag/reorder, the group-styled
compact flyout, the per-widget header menus, the in-window float overlay, the
bounded normal-mode width / larger icons, and the menu-bar overlay root cause (a
duplicate `actionsPanel_`). It is **implemented and independently verified** (see
the milestone entry above). **M43 was a fifth user-requested interruption — panel
multicolumn** (`openspec/changes/m43-panel-multicolumn`, brief
`docs/dev/m43-panel-multicolumn.md`): the multi-column host of
create-on-drop/remove-when-empty columns, tab-vs-group drag and single-panel
floats, the unified `DropKind`/`resolveDrop` drop targets, the `panelTabBar` tab
colours, the fixed corner button and actual-geometry inner-side flyout, the
fixed-width Tools dock, the `D` colour reset, and session **v6**
  (`panelColumns`). It is **implemented and independently verified** (see the
  milestone entry above). **M44 was a sixth user-requested interruption — panel/
  theme polish** (`openspec/changes/m44-panel-theme-polish`, brief
  `docs/dev/m44-panel-theme-polish.md`): the new-document canvas E1 root cause
  (the GPU demo image assigned straight to `rust.image`) and its fix, the
  chevron/tab-colour/default-active corrections, the theme border/tab/flyout/elide
  unification, the float-drag continuation and any-side docking, and the compact
  group-relative drop with drag handles. It is **implemented and independently
  verified** (see the milestone entry above). **M45 was a seventh user-requested
  interruption — panel fixes** (`openspec/changes/m45-panel-fixes`, brief
  `docs/dev/m45-panel-fixes.md`): the resolver-owner drop indicator, the single
  emptied-column cleanup, the group-height minimize with its state-derived label,
  the one-formula Tools sizing and left/right-only beside-column pane, the shared
  minimum-width floor with no clipping, and the whole-group compact popup. It is
  **implemented and independently verified** (see the milestone entry above).
  The M40/M41/M42/M43/M44/M45 interruptions claimed six numbers the Layers-panel
  program had reserved. Two further user-requested panel-fix changes then took
  **M46** (`m46-panel-toolbar-fixes`) and **M47**
  (`m47-panel-interaction-fixes`): drop-indicator geometry, cross-column commit
  routing, the floating-Tools drag across Qt's dock mouse grab, primary-column
  removal, minimize min-height clamping, empty/ghost-column cleanup, compact
  float drag and group creation, the shared content floor with no horizontal
  scroll, the Tools central-splitter pane, and the floating close button. Both
  are implemented, independently verified, and **archived**
  (`openspec/changes/archive/2026-09-18-m46-panel-toolbar-fixes/` and
  `…-m47-panel-interaction-fixes/`). The **controls stage**
  (`layers-panel-controls`: percent Opacity/Fill, the five-lock strip with the
  nesting lock, and the clipping row indicator) and the **filter/search stage**
  (`layers-filtering-search`: the six-dimension filter row with ancestor
  promotion) have landed and archived, followed by a **chrome pass**
  (`layers-panel-chrome-fixes`: header order and labels, left-anchored eye with
  a group chevron, and the filter lightswitch on by default), a **row pass**
  (`layers-panel-row-interactions`: label scrub + `%`, semantic lock/eye/Kind
  icons, and row drag-and-drop reorder plus drop-on-strip), and a **control
  polish pass** (`layers-panel-control-polish`: eye-only rows, `%` inside the
  box, a right-side lock badge, and one-undo-state live Opacity/Fill preview),
  and the **management stage** (`layers-panel-management`: Merge
  Down/Layers/Visible/Clipping Mask and Flatten, the New Layer/Group dialog and
  Group from Layers, a first-class `Layer.background` flag with both
  conversions, Layer via Copy/Cut, Select Similar/Linked and transient link
  sets, Delete Hidden/Hide Layers, a minimal solid-fill layer kind with the
  Rasterize subset, and the deferred drag-reorder drop rules with a dry-run
  predicate).
  The Layers-panel program therefore continues **by content, not by number**:
  next is **layer styles / effects** (fx menu and badge, effect child rows, the
  Layer Style dialog, effect rendering passes, and the `Fill`-after-effects
  compositor change), then **smart objects / vector masks /
  artboards-as-non-goal / layer comps**. The management stage's deliberate
  inferred behavior and documented ceilings (rename↔background, the
  clipping-coverage compositor gap, zero-layer Flatten, `select_similar`
  excluding the active layer, fixed background/solid-fill colors, the `SoCo`
  descriptor ceiling) are listed in
  `openspec/changes/archive/…-layers-panel-management/design.md` §Residual. The
  pre-shift numbers still stand in `docs/dev/layers-panel-program.md`; this file
  is the up-to-date anchor.

The **selection tools** `selection-tools-and-menu` change is also landed and
archived: Elliptical Marquee (plus marquee Anti-alias/Feather/Style options),
Polygonal Lasso (click-vertex, close on first vertex/double-click/Enter, Esc
cancels, Feather), Magic Wand (Tolerance, Contiguous global/contiguous, combine
modes), and the whole Select menu — Reselect, Inverse, Modify
(Border/Smooth/Expand/Contract/Feather), Grow, Similar, Save/Load Selection to
an alpha channel, and All/Deselect/Similar Layers — mostly wiring the existing
`pictura-select` engine. Magnetic Lasso, Color Range, Refine Edge, and Transform
Selection are deliberately deferred and left visible-but-disabled; the anti-alias
and Sample-All-Layers options are honest disabled controls. The residual ceilings
are recorded in `openspec/changes/archive/…-selection-tools-and-menu/design.md`
§Residual.

The committed selection edge is drawn as **marching ants**: `pictura-select::contour`
extracts the 50 %-coverage boundary as chained lattice polylines (bbox-scoped, with
a Select-All short-circuit), `PictureView::selection_contour()` serializes them, and
`ImageView` paints them as an animated black dash over a white line in the document
transform. A tool drag shows the same marching-ants outline live as a selection
preview (`ImageView::setSelectionPreview`, the ellipse tool previewing the actual
ellipse, not its bounding box), replaced by the committed contour on release. Tool
commits reach the canvas through the now-connected
`ToolController::selectionCommitted` → `refreshSelectionOverlay()`; menu/`changed`
paths go through `refresh()`. `View > Show > Selection Edges` is wired as a
per-canvas toggle (default on). `apply_selection` deliberately does not emit
`changed()` (Quick Selection would repaint per mouse-move); only the released
commit refreshes the overlay.

A selection UX pass followed: the options bar packs left (`addStretch` per page)
with icon selection-mode buttons and a leading tool-icon + presets-chevron button;
the marquee/elliptical tools use a crosshair reticle cursor with Shift `+` / Alt
`-` variants; the lasso tools gained a top-left arrowhead (hotspot `(2,2)`); the
polygonal lasso previews an open polyline through clicked vertices only (no
rubber band to the cursor, no phantom closing edge) and commits on close; and
dragging inside an existing selection with a marquee/lasso tool now moves the
selection outline (arrow + small-marquee cursor) instead of starting a new
selection, recorded as one `Move Selection` state (`Selection::translate`).
Presets, pixel-content moves, and the deferred tools remain future work.

A modifiers-and-content-move pass then closed the Photoshop gap. Combine quick
keys are decided at the first press and locked for the gesture: Shift = Add, Alt
= Subtract, Shift+Alt = Intersect, but only when a selection already exists
(otherwise the options-bar mode applies); `ToolController::selectionModeForModifiers`
maps them and `dragMode_` carries the result through the commit. The Rectangular
and Elliptical Marquee constrain the Normal drag — Shift squares/circles it, Alt
treats the press point as the centre — and a floating `W x H` readout follows the
cursor (`ImageView::setDragSizeHint`). The Polygonal Lasso preview is now a solid,
open rubber band from the first vertex through the clicks to the live cursor (the
`solid` flag on `setSelectionPreview`); the cursor segment is preview-only. The
canvas enables mouse tracking so the move-selection cursor appears on hover, and
Ctrl over a selection shows it too. Moving selected pixels is real:
`pictura_render::move_selection_content` reuses `layer_via_copy`/`layer_via_cut`,
translates the new layer, and merges it back down (or leaves it as a new layer
when duplicating); the bridge records one `Move Selection` state. The Move tool
moves the selected pixels (Alt duplicates to a new layer) whenever a selection
exists, and a selection tool with Ctrl does the same (Ctrl+Alt duplicates) while
a plain drag keeps moving only the outline. Content-move drags preview the moving
selection outline, not the pixels (`ponytail:` the masked base composite is the
upgrade), and the target layer is still the topmost pixel layer, matching the
existing Move tool. Self-tests `tsc_quick_modes` (270) through
`tsc_quick_mode_drag` (276) cover the mapping, geometry, view hooks, polygon band,
and the move/duplicate wiring.

PSD interop phase P1 (`docs/dev/psd-support-roadmap.md`) landed in
`pictura-codec`: the composite and layer channel readers now accept compression
`2` (ZIP/zlib deflate, with a raw-deflate fallback) and `3`
(ZIP-with-prediction, inverting the byte-wise per-row delta after inflating), an
unrecognized blend key degrades to `BlendMode::Normal` instead of aborting the
file, and a layered document whose merged composite is absent parses with a
zero-filled composite. `flate2` (pure-Rust miniz_oxide backend) is the only new
dependency. Write behaviour is unchanged (still raw) so the byte-layout golden
holds. psd-tools' writer only emits raw/RLE, so the differential oracle
(`tests/oracle.rs`) builds a ZIP/ZIP-with-prediction PSD by hand and requires
psd-tools' *decoder* to agree byte-for-byte with `read_psd`. Remaining PSD gaps
(color modes, 16/32-bit, image resources/ICC/metadata, effects/smart
objects/text, PSB write, write-side RLE/ZIP) are tracked in the roadmap.

PSD interop phase P2 (`psd-opaque-preservation`) made an open→save round trip
lossless for everything the engine does not interpret. `Document` now carries
the raw color-mode-data and image-resource sections, the global layer mask, and
the trailing layer-section bytes; `Layer` carries its original blend key (only
when unrecognized), blending ranges, unknown additional-layer-info tagged blocks,
and unmodeled channels (the `-3` real user mask and any other id outside
`-2..=2`); `LayerMask` carries the mask block bytes past the fixed fields. The
codec captures them on read and re-emits them verbatim on write, so ICC/EXIF/XMP
resources, resolution, effects/smart-object/text blocks, and vector-mask channels
are preserved even though the engine cannot render them yet. A recognized blend
key is not stored, so a constructed document still equals one read from disk, and
engine-created documents keep empty storage — `write_psd` output for them is
byte-identical (the `default_before.psd` golden holds). The model change touched
~160 struct literals across the workspace via `..Default::default()`, and
`layer_ops/tests.rs` was split (`tests_via.rs`) to stay under the size cap; the
independent oracle proves a psd-tools-authored fixture round-trips
whole-`Document`-equal and that psd-tools still opens our re-emitted resources.

> These numbers reuse M36–M38 previously sketched for canvas performance below.
> `docs/dev/canvas-compositing-plan.md` is frozen and still uses them, so read
> those tracks by name (history COW, resident GPU sources, 256² tiles), not by
> number; they are deferred until after the panel program.

M31 removed the full composite and readback from every move and paint
(dirty-rect compositing), M32 removed it from the move-preview base and the
visibility toggle and cached the present-scale, and M33 removed the host-side
per-pixel assembly from the remaining full composites; every other mutation still
composites and reads back the **whole** document, and the CPU compositor and
`pictura_filters::apply` are still the oracles. The research and the M31–M35 plan
are written up in `docs/dev/canvas-compositing-plan.md`; the M32 brief with the
measured phase table is `docs/dev/m32-interactive-canvas.md`, and the M33 brief is
`docs/dev/m33-composite-throughput.md`. The 4000² full composite is now ~123 ms,
dominated by the per-composite source/mask **upload** (~128 + 16 MB), not the GPU
dispatch (~0.2 ms) or the ~38 ms readback, so a zero-copy present still saves
little while the upload stays.

The perf series was **paused** for **M34 — composite coherence and cheap
undo/redo** (OpenSpec change `m34-composite-coherence`, **implemented**; brief
`docs/dev/m34-composite-coherence.md`): the canvas rebuild persists its rendered
result into `doc.composite` (RGBA for an RGB document, plane-count-preserving
for a non-RGB mode), Save serializes that current composite, `undo`/`redo`
restore the display from the snapshot composite instead of a full composite
(unconditionally), and the `record` call moves after the composite step so the
snapshot carries it. It is app-local and bounded: no `write_psd` format change,
no new capability. Next in order:

- **M35 — region blit in C++ (implemented; archive pending).** OpenSpec change
  `m35-cpp-region-blit` (MODIFIED `document-canvas`; no new capability → **59**
  after archive), brief `docs/dev/m35-cpp-region-blit.md`. `PictureView` no longer
  maintains a region-patched full-resolution `QImage`: `refresh_region` composites
  the region, patches the planar `doc.composite` with `copy_from_slice`, converts
  only the region-sized buffer to a `QImage`, sets a `display_dirty` marker and
  emits a new `region_blitted(QImage, x, y)` signal; `ImageView::blitRegion` paints
  it with `QPainter` + `CompositionMode_Source` (invalidating the present zoom
  cache so the next paint rebuilds it identically). The per-pixel
  `QImage::set_pixel_color` loop and `REGION_REFRESH_BUDGET` are deleted, so a large
  dirty region is blitted in C++ instead of forcing a full document composite.
  `image()` rebuilds from `doc.composite` when `display_dirty` (with an explicit
  in-stroke guard), `sample_argb` reads the planar composite directly, and
  `move_preview_base` builds from the current composite. Measured 4000² (release,
  GPU): a 1024² region refresh **~33 ms → ~8.8 ms** (composite 4.95 + composite
  patch 1.81 + region convert 2.04) plus one `QPainter::drawImage` blit in C++; a
  512² region refresh **3.29 ms**. The old path was a 27.3 ms per-pixel FFI blit
  plus a ~5.6 ms composite. Honest notes: `refresh_region` no longer emits
  `changed`, so panel refresh on the region path is debounced in `frame.cpp`; a
  region blit invalidates the present zoom cache (one rescale on the next paint,
  the same cost as the old `replaceImage`); `begin_move_preview` now pays one planar
  clone + conversion per drag start; mid-stroke `sample_argb` reads the pre-stroke
  composite (unreachable while painting). `m31_region_large` now asserts the region
  path ran *and* the canvas equals a full recomposite (strictly stronger than
  before), and `move_preview_region` no longer has an oversized-rect fallback
  because the budget is gone. Verified: `cargo test --workspace` **546 tests, 0
  failed, 7 ignored** (up from 544, 6 ignored; the new
  `m35_region_refresh_profile_4000`), `cargo fmt`/`clippy` clean, both self-tests
  exit 0 with `m35_region_blit region=1 changed=0 canvas=1 rebuilt=1 cache=1` and
  `m35_region_large region=1 recomposite=0 canvas=1`, `openspec validate --all
  --strict` 60/60.

- **M34 — composite coherence and cheap undo/redo (implemented; archived).**
  OpenSpec change `m34-composite-coherence` (MODIFIED
  `edit-history`, `document-lifecycle`; no new capability → **59** after archive).
  The canvas rebuild now persists its rendered frame into `doc.composite`
  (`store_composite`: RGBA for an RGB document, colour-plane-count-preserving
  otherwise), `recomposite` renders → stores → builds the `QImage` from the
  rendered frame, `refresh_region` patches the composite on its region path, and
  `record` runs after the composite step so every snapshot carries a current
  composite. `undo`/`redo` therefore rebuild the display with
  `buffer_to_image(&snapshot.doc.composite)` instead of a full composite, and
  Save serializes the current composite. Measured 4000² (2 RGB layers, release,
  GPU): the old undo display path `document_to_image(gpu=true)` **163.16 ms** vs
  the new `buffer_to_image(snapshot.composite)` **40.94 ms** (~4×); the remaining
  per-snapshot cost is the whole-document `History::capture` clone at **59.49 ms**
  (COW/tile-diff history remains deferred). Honest limits: a layered **grayscale**
  document with transparent coverage restores opaque because its composite stays
  1-plane, and a dimension-changing op on grayscale keeps the pre-existing
  4-plane outcome. Verified: `cargo test --workspace` **544 tests, 0 failed, 6
  ignored** (up from 541, 5 ignored; the new `m34_undo_profile_4000`), `cargo
  fmt`/`clippy` clean, both self-tests exit 0 with
  `m34_coherent composite=1 undo=1 save=1`, `openspec validate --all --strict`
  60/60.
- **M33 — full-composite throughput (done; archived).** OpenSpec change
  `m33-composite-throughput` (MODIFIED `gpu-compositing`; no new capability),
  implemented and verified. Row-wise source/mask assembly, a fused planar readback
  that skips the packed `Vec`, and a GPU command-buffer canvas clear took the
  4000² two-layer composite ~254 ms → ~123 ms (~2×), byte-identical. The remaining
  bottleneck is the per-composite upload; resident per-layer GPU source buffers are
  deferred (they need content versioning).
- **GPU-resident zero-copy present (deferred; was planned as M34)** via Qt Quick
  (`QQuickRhiItem` sharing the window's `QRhi` +
  `QQuickWindow::createTextureFromRhiTexture()`, or one shared Vulkan device via
  `QQuickGraphicsDevice::fromDeviceObjects(...)`); `QRhiWidget` cannot adopt the
  wgpu device. Deferred because it removes only the ~38 ms readback of a ~123 ms
  composite, not the upload.
- **256² GPU tiles (deferred) + LRU + seam gutters + mipmaps**
  (Graphite-style), only if pan/zoom over documents larger than VRAM demands it;
  includes display-time LoD so a zoomed-out view composites a proxy.

Deferred canvas-performance tracks (previously sketched as M36–M38; those
numbers are now claimed by the panels program above, so these are deferred
until after M49). The remaining canvas-performance tracks — history
copy-on-write / tile diffs, resident per-layer GPU source buffers, 256² tiles +
LoD, plus the GPU-resident zero-copy present — each need their own design (the
small, app-local region-blit slice landed as M35 above):

- **Cheap undo/redo + composite coherence/save** — landed as M34
  (`m34-composite-coherence`); see the brief `docs/dev/m34-composite-coherence.md`.
  The original concern — persisting the rendered composite into `doc.composite`
  changes what `write_psd` serializes — is handled by storing the rendered RGBA
  frame for RGB and preserving the composite's colour-plane count for a non-RGB
  mode, so the byte layout is unchanged.

- **M35 — region blit in C++ / `REGION_REFRESH_BUDGET` removal — landed.** See
  the milestone entry above; OpenSpec change `m35-cpp-region-blit`, brief
  `docs/dev/m35-cpp-region-blit.md`. The per-pixel `QImage::set_pixel_color` loop
  and the budget fallback are gone; a dirty region of any size takes the region
  path.
- **History copy-on-write / tile diffs** — the history capture still clones
  the whole document (~60 ms per state at 4000², and holds up to 20 states).
- **Resident per-layer GPU source buffers** — deferred from M33; the
  shader-side planar output landed as the M33 A2 follow-up above. Keeping a
  layer's source plane resident on the GPU across a composite session needs
  content versioning to detect a changed layer; the remaining composite cost is
  the per-composite upload (~128 MB + 16 MB at 4000²), which residency would
  remove.
- **Transparency grid preferences** — M30's checkerboard is fixed at an 8 px
  Light (`#FFFFFF`/`#CCCCCC`) grid; the `Transparency & Gamut` preferences pane
  (grid size None/Small/Medium/Large, colour sets Light/Medium/Dark/Red/Custom),
  the `View > Show > Transparency Grid` toggle, and gamut warning are deferred.
- **GPU painterly/stochastic filters and GPU painting** — the remaining M22
  Artistic and M25 Brush Strokes/Sketch/Texture families (`Watercolor`,
  `Conté Crayon`, `Paint Daubs`, `Dry Brush`, `Ocean Ripple`, `Spatter`,
  `Sponge`, `Palette Knife`, `Add Noise`, `Colored Pencil`, `Crystallize`) on the
  GPU by **pre-generating their seeded RNG fields on the CPU and running only the
  spatial work on the GPU**, since the RNG stream cannot be reproduced
  bit-exactly on the GPU, and the paint dab loop (GPU painting).
- Real content for the M24 placeholder panels (gradient/pattern presets,
  Properties binding, adjustment presets, libraries, channel/path lists, actions)
  and image modes / bit-depth (16/32-bit, CMYK/Lab gating for filters and
  adjustments).

Process: every new milestone is proposed through OpenSpec first
(`openspec/changes/<name>`, new capabilities), validated, then implemented.
M6 through M34 are archived; their deltas now live in `openspec/specs/`.

## Known risks / open items

- Core API is frozen only where noted; adding a field to a core struct breaks
  only the handful of exhaustive struct literals — the ~160 others use
  `..Default::default()`.
- PSD descriptor coverage is broad: adjustment layers decode and re-encode
  (Invert, Posterize, Threshold, Brightness/Contrast, Levels, Hue/Saturation,
  Exposure, Photo Filter, Vibrance, Shadows/Highlights, Black & White, Gradient
  Map, Color Balance, Channel Mixer, Curves, Selective Color, Color Lookup, and
  the Solid/Gradient/Pattern fills), and layer styles decode and re-encode from
  the layer's `lfx2` descriptor. A key the engine does not model is kept
  verbatim and re-emitted.
- GPU is the default compositor and the default filter path, but the CPU
  compositor and `pictura_filters::apply` remain the oracles.
- A `Dissolve` layer or an unsupported adjustment forces a whole-document CPU
  fallback for that composite.
- Sixteen filter kernels have GPU implementations (Gaussian Blur, Box Blur,
  Motion Blur, Radial Blur Zoom, Blur, Blur More, Sharpen, Sharpen More, Unsharp
  Mask, High Pass, Surface Blur, Maximum, Minimum, Median, Custom 5x5, Oil
  Paint), but eleven of them — the blur/sharpen/High Pass family plus Maximum,
  Minimum, Median and Custom — are routed to the CPU on purpose, because the
  CPU beats the GPU round trip for them. The stochastic/seeded filters and the
  warps/distort and render filters have no GPU kernel and fall back to the CPU
  oracle byte-for-byte.
- The GPU compositor and filter path are host-side bound, not readback-bound: at
  4000² the composite is now dominated by the per-composite source/mask upload
  (~128 + 16 MB) after M33 removed the per-pixel assembly, while the 64 MB
  readback is ~7 ms (see `docs/dev/canvas-compositing-plan.md` §2.1). Resident
  per-layer GPU source buffers have since landed (a stamp-keyed LRU in
  `crates/pictura-render/src/gpu/resident.rs`); the GPU-resident zero-copy
  present is still deferred — it was planned as M34, which shipped as composite
  coherence and cheap undo/redo instead.
- M35 (`m35-cpp-region-blit`) removed both the per-pixel `QImage::set_pixel_color`
  blit and the 1 MP `REGION_REFRESH_BUDGET` fallback: the planar composite is the
  authoritative canvas, `refresh_region` signals the region, and C++
  `ImageView::blitRegion` copies the region rows into the canvas in place (a
  clamped row copy with `CompositionMode_Source` semantics) rather than
  painting it with `QPainter`. A
  CPU display pyramid (change `canvas-view-performance`) now caches the full
  composite's halved levels and crops one for the canvas, so a zoomed-out present
  no longer scales the full-resolution document; proxy-level *compositing*
  (composite a proxy of each layer) is still absent, so a composite still runs at
  document resolution for the damaged region.
- History capture is copy-on-write with tile diffs: one materialized state is
  kept and every other state stores the 64×64 tiles that changed against its
  predecessor, so retention scales with the edited area rather than with the
  document (up to 20 states).
- The GPU path still silently falls back to the CPU on any `GpuError`.
- `pictura-app` has 18 `#[ignore]`d tests, all profiling harnesses (4000²/16000²
  region, dab, visibility, filter-dialog, crop, marquee and undo profiles) that
  run only with `--ignored --nocapture`.
- The recent-files submenu is repopulated each time it opens, so a file opened
  in-session appears in it at once; a file that has since moved or been deleted
  is dropped from the list.
- Quick Selection is a wand-union approximation, not a true Photoshop quick
  selection. Selection marching ants are implemented: the committed contour is
  drawn as a solid white base under an animated black dashed pen. Crop is not
  destructive by default — the Crop tool has an interactive region (handles,
  ratio lock, overlays) and the pixels outside the frame are kept unless
  `Delete Cropped Pixels` is on, so a crop can be widened again.
- Icon art is Lucide-derived or hand-authored (195 icons, mapped in
  `docs/dev/icon-provenance.md`); it is not CS6-pixel-exact, and a visual
  refinement pass can still change SVG paths without any code change.
- Cursors render at a single DPR (no per-screen 2×/3× cursor variants yet).
- The M20 Layers panel is a full tree: groups expand, rows drag-reorder, the
  lock strip sets the transparency/position/nesting/all locks, the clipping
  base, the layer and mask links, and the CS6 colour label are shown, and a
  filter bar filters rows by name, kind, mode, colour, attribute, effect, or the
  current selection.
- Swatch library file I/O (`.aco` and library load/save) and the Histogram
  panel's source selector and cache states (Uncached Refresh, cache-level badge,
  statistics block) are not implemented; the Info panel's colour samplers are.
- Painting is limited to 8-bit RGB single raster layers; the coverage scratch is
  a layer-sized buffer (sparse tiles deferred).
- The paint modes are Normal, Dissolve, Behind, Clear and Multiply; there is no
  tablet pressure mapping (the Stylus Pressure toggle is shown disabled), but
  brush presets are implemented — the options bar's tip picker carries the
  default brush set with size, hardness, spacing and dynamics.
- Artistic filters are behavioural-parity models without an Adobe oracle. The
  Filter Gallery (category thumbnails plus a stacked, reorderable effect list)
  and Smart Filters (add, toggle, delete, reorder, and `Convert for Smart
  Filters`; the engine models the camera-raw filter) are implemented, but the
  filters have no depth or mode gating.
- The CS6 chrome is a defensible dark look, not a pixel-exact match: the palette
  carries explicit CS6 dark swatches and the CS6 accent, but the metrics and the
  remaining surfaces are approximations.
- Panel contents are implemented well beyond M20 (Properties, Channels, Paths,
  History, Navigator, Color, Swatches, Info, Histogram, Notes, Brush, Clone
  Source, Character, Paragraph, Glyphs, Paragraph Styles), as are the workspace
  presets and the icon-collapse docks; Gradients, Patterns, Adjustments, Actions
  and Styles are still empty placeholders.
- Some M24 panels are still structural placeholders with empty states
  (Gradients, Patterns, Adjustments, Actions and Styles) while Properties,
  Channels and Paths carry real content. Icon-collapse, workspace presets and
  panel-title-bar menus are implemented.
- Oil Paint is a CPU behavioural model: CS6 requires a supported GPU (closed
  OpenCL kernel, no CPU fallback), so the CPU result is a deliberate non-parity
  divergence rather than verified parity. The engine also carries a four-pass
  GPU kernel for it; the CPU model remains the oracle.
- The OS font picker (the type tool's `QFontComboBox`) and the Filter Gallery
  are both present.
