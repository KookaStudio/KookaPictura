# Brush Presets

- **Spec ID:** `BRU-006`
- **Status:** `Draft`
- **Parity tier:** `Core` — the Brush Presets panel, `.abr` libraries, and the Preset Manager are in CS6 Standard.
- **New in CS6:** `Changed` — CS6 serializes brush libraries as **ABR version 10 (subversion 1)**; the panel UI and preset vocabulary are otherwise unchanged from CS5.
- **Depends on:** `BRU-001` brush-engine, `BRU-002` brush-dynamics, `BRU-003` bristle-brushes, `BRU-004` mixer-brush-engine, `10-workflow-io/presets-manager.md`, `10-workflow-io/presets-manager.md`, `ARCH-008` document-model.

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and
> what did not. The ABR format is closed and only partly publicly documented;
> bold claims about serialization are attributed to the community analysis,
> not Adobe. Anything unverified is under Open questions.

## CS6 behavior

A **preset brush** is a saved brush tip with defined characteristics such as
size, shape, and hardness. Presets are browsed in the **Brush Presets panel**
and the options-bar **Brush preset picker**.

- **Selecting a preset.** Pick from the options-bar pop-up or from `Window >
  Brush Presets`. The picker temporarily exposes **Diameter**, **Use Sample
  Size**, and **Hardness** (round/square only); these edits are temporary —
  re-choosing the preset later restores the brush's original settings.
  To keep changes, create a new preset.
- **Display options** (Brush Presets panel menu): `Text Only`, `Small`/`Large
  Thumbnail`, `Small`/`Large List`, and `Stroke Thumbnail` (a sample stroke plus
  thumbnail; available for brush presets only). Hovering a preset previews sample
  strokes at the bottom of the Brush panel.
- **Create.** `New Brush Preset` from the panel menu, or the `Create New Brush`
  button. New presets are saved in a **Preferences file**; they are lost if that
  file is deleted/damaged or if brushes are reset to the default library. To make
  them permanent or shareable, save them as a library.
- **Libraries.** Brush libraries use the `.abr` extension. Panel-menu commands:
  `Load Brushes` (append to the current list), `Replace Brushes` (replace the
  list), a library file listed at the bottom of the menu (OK = replace, Append =
  append), `Reset Brushes` (restore defaults, replace or append), `Save Brushes`
  (write the current list to an `.abr`). Saving anywhere works, but placing the
  `.abr` in the default `Presets/Brushes` folder makes the library name appear in
  the panel menu after restart. The **Preset Manager** (`Edit > Presets > Preset
  Manager` in CS6) can also load/reset brush libraries and manage swatches,
  gradients, styles, patterns, contours, custom shapes, and tool presets.
- **Rename/Delete.** `Rename Brush` from the panel menu or double-click a tip in
  the Brush panel; `Delete Brush` via the menu, the Delete icon, or Alt/Option-
  click a preset.
- **Dynamic options.** Every Brush panel setting (`BRU-001`/`BRU-002`) is part of
  the brush preset. `Clear Brush Controls` resets all non-shape controls at once.
  Per-section padlocks (and `Protect Texture`) retain settings when switching
  presets.
- **Brush preset vs tool preset.** Help distinguishes them: a **tool preset** is
  for storing customized brush tip characteristics together with options-bar
  settings such as opacity, flow, and color. Tool presets live in the
  Tool Preset picker, the Tool Presets panel, and the Preset Manager, with `Show
  All Tool Presets` vs `Show Current Tool Presets` (Current Tool Only) filtering.
  Community usage (Glen Smith) similarly frames the choice: create a tool preset
  or a brush preset; a newly created brush preset can capture the brush size, the
  included tool settings, and optionally the color.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Options bar, Brush preset picker | Pop-up panel | — | Diameter, Use Sample Size, Hardness; cog menu with New Brush Preset |
| Brush Presets panel | Dock | `F5` / `Window > Brush Presets` | Preset grid/list, dynamic hover preview, Create New Brush |
| Brush Presets panel menu | Menu | — | Display options; Load/Replace/Reset/Save Brushes; Rename/Delete; New Brush Preset |
| Brush panel | Dock | `F5` / `Window > Brush` | Edit the selected preset's settings; Clear Brush Controls; padlocks |
| Preset Manager | Dialog | `Edit > Presets > Preset Manager` | Brush + all other preset types; Load/Replace/Append/Reset; rename/delete |
| Tool Preset picker / Tool Presets panel | Pop-up / Dock | — | Tool presets (brush + options-bar settings) |
| Edit > Define Brush Preset | Menu | — | Creates a sampled tip and a new preset from a selection |
| Filesystem | Files | — | `Presets/Brushes/*.abr`; default preset location |

## Parameters & ranges

### ABR file format (community analysis)

Header (all versions): `version` (big-endian int16), then `count`/`subversion`
(int16). Version history relevant to CS6:

| Version | Photoshop era | count field | Structure |
|---|---|---|---|
| 1 | PS 5.x and earlier | brush count | Flat brush records |
| 2 | PS 6.0–7.0 | brush count | Flat records + UCS-2 names |
| 6 sub1 | PS CS (8.0) | subversion=1 | 8BIM sections (`samp`, `patt`, `desc`) |
| 6 sub2 | PS CS2–CS5 | subversion=2 | Larger fixed header (301 vs 47 bytes) |
| **10 sub1** | **PS CS6** | subversion=1 | Same 8BIM structure as v6 |
| 10 sub2 | PS CC (14.0+) | subversion=2 | Larger header |

Versions 3/4/5/7/8/9 do not exist. Computed brushes (type 1) are a 14-byte
record: spacing (0–999), diameter (1–999), roundness (0–100%), angle (−180…180),
hardness (0–100%). Sampled brushes (type 2) carry a grayscale bitmap with bounds,
depth 8, and PackBits/RLE compression (chunked at 16384 scanlines). v6/v10 files
are `8BIM` tagged sections: `samp` (tip images, with a UUID linking to `desc`),
`patt` (patterns/textures), `desc` (ActionDescriptors with the brush parameters),
and optional `phry` metadata.

### Preset fields (behavioral)

| Field group | Includes | Notes |
|---|---|---|
| Tip shape | Size, Hardness, Spacing, Angle, Roundness, Flip X/Y, tip type | `BRU-001`; bristle/erodible/airbrush have their own params |
| Shape Dynamics | Size/Angle/Roundness jitter, Minimum Diameter/Roundness, Tilt Scale, Brush Projection | `BRU-002` |
| Scattering | Scatter, Count, both Jitter + Control, Both Axes | `BRU-002` |
| Texture | Pattern, Invert, Scale, Texture Each Tip, Mode, Depth, Min Depth, Depth Jitter, Brightness, Contrast | `BRU-002` |
| Dual Brush | Secondary tip, Mode, Diameter, Spacing, Scatter, Count | `BRU-002` |
| Color Dynamics | Apply Per Tip, FG/BG Jitter, Hue/Sat/Brightness, Purity | `BRU-002` |
| Transfer | Opacity/Flow jitter (+ Mixer Wet/Load/Mix) | `BRU-002`/`BRU-004` |
| Brush Pose | Tilt X/Y, Rotation, Pressure + Override | `BRU-002`, CS6 |
| Toggles | Noise, Wet Edges, Airbrush/Build-up, Smoothing, Protect Texture | `BRU-001`/`BRU-005` |

## Algorithms & pipeline

**Preset lifecycle.**

1. **Author** — modify a tip/options in the Brush panel; `New Brush Preset`
   captures the full settings block into the in-memory preset list.
2. **Persist (session)** — new presets are written to the Preferences file, so
   they survive a restart but not a preferences reset.
3. **Persist (durable)** — `Save Brushes` serializes the current list to an
   `.abr`; `Load`/`Replace`/`Append`/`Reset` manipulate the list.
4. **Apply** — selecting a preset loads its tip + dynamics into the active tool;
   per-section padlocks/Protect Texture decide which current values survive the
   switch.

**ABR read (proposed).** Parse the header, then the `8BIM` sections:

- `samp` — decode bounds/depth/compression → one `GrayBitmap` per tip; read the
  embedded UUID.
- `patt` — decode patterns (grayscale/indexed/RGB/CMYK) for textured presets.
- `desc` — parse the recursive ActionDescriptor tree (keys such as `Nm  `, `Dmtr`,
  `Hrdn`, `Angl`, `Rndn`, `Spcn`, `flipX`, `useTipDynamics`, `szVr`,
  `minimumDiameter`, `angleDynamics`, `roundnessDynamics`, `useScatter`,
  `useTexture`, `dualBrush`, `useColorDynamics`, `usePaintDynamics`, `Wtdg`,
  `Nose`, `Rpt `) and map them onto the model above. Link tips/patterns by UUID.

**ABR write.** The community analysis warns the ActionDescriptor encoding is
inconsistent (variable key-length and count-width encoding, boolean padding), and
the most complete parser still cannot write valid modern ABR. **Proposal: do not
write ABR as the primary format.** Use an open, versioned native preset format for
round-tripping and offer ABR **import** (and best-effort export) for
interoperability. This is a deliberate deviation from parity and is recorded as
such.

## Rust module mapping

- `pictura-presets::brush::BrushPreset` — the full settings block + name + id.
- `pictura-presets::brush::PresetLibrary` — ordered list; `load_abr`,
  `replace`, `append`, `reset`, `save_native`.
- `pictura-presets::abr::AbrReader` — header + `8BIM` section parser.
- `pictura-presets::abr::ActionDescriptor` — recursive value tree with the
  documented heuristics for key/count width; `DescValue` enum (`Text`, `UnitFloat`,
  `Double`, `Long`, `Bool`, `Enum`, `Object`, `List`).
- `pictura-presets::abr::SampledTip`, `PatternData` — `samp`/`patt` decoders.
- `pictura-presets::native` — open native format (ABR-independent round-trip).
- `pictura-presets::tool::ToolPreset` — brush + options-bar settings (opacity,
  flow, color, mode).
- `pictura-presets::prefs::PreferencesStore` — session-persistent new presets.

Boundary types: `BrushPreset`, `GrayBitmap`, `PatternData`, `ToolPreset`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `BrushPresetsPanel` | `QWidget` dock | Grid/list view with display-mode switch and stroke thumbnails |
| `BrushPresetModel` | `QAbstractListModel` | Presets + thumbnails; hover-preview role |
| `BrushPresetPicker` | `QComboBox`/popup | Options-bar picker with Diameter/Hardness/Use Sample Size |
| `PresetManagerDialog` | `QDialog` | Load/Replace/Append/Reset; rename/delete; multi-type tabs |
| `ToolPresetsPanel` | `QWidget` dock | Tool presets; Show All / Current Tool Only |
| `AbrFileDialog` | `QFileDialog` | Load/Replace/Save Brushes; default `Presets/Brushes` location |
| `NewPresetDialog` | `QDialog` | Name + Capture size / Include tool settings / Include color |

Widgets (desktop panels). Preset thumbnail rendering is delegated to the Rust
brush renderer and surfaced as a `QImage`/`QQuickItem`; the list uses a
`QAbstractListModel` so the Brush Presets panel and picker stay in sync. File I/O
runs off the UI thread; ABR import reports partial-parse diagnostics (community
formats often fail partway).

## Data-model impact

- **No PSD fields.** Presets are not part of the document; brush libraries are
  external files.
- **Serialization:** `.abr` (read/best-effort write) and a native format. The
  ABR `samp` tips and `patt` patterns are embedded binary; `desc` holds dynamics.
  `Presets/Brushes` is the discovery folder.
- **Preferences:** session presets live in the preferences store; a preferences
  reset drops them (matching CS6), so the UI must warn before reset/replace.
- **Undo:** preset edits are not document history states; they are app-state.
- **Tool presets** are a separate namespace from brush presets and must not be
  conflated in storage.
- **Forward compatibility:** store an ABR version/subversion tag and unknown
  descriptor keys so re-saving a file opened from a newer Photoshop preserves
  what was not understood.

## Edge cases

- **Preferences reset.** New (unsaved) presets are lost; warn first.
- **ABR parse failure partway.** Keep successfully parsed tips; report the
  section/offset that failed; never crash on malformed input.
- **Unknown ABR version/subversion.** Degrade to a heuristic scan (community
  parsers do this) and flag the preset as "partially imported."
- **Duplicate names / duplicate UUIDs.** Disambiguate on load; do not silently
  overwrite.
- **Computed (type 1) brushes.** Rarely implemented by other apps; support them
  procedurally (spacing/diameter/roundness/angle/hardness).
- **Huge sampled tips / patterns.** Enforce the 2500×2500 tip and memory bounds;
  stream/limit patterns from `patt`.
- **Missing pattern for a textured preset.** Preset loads but Texture is a no-op;
  `Protect Texture` must not crash.
- **Bristle/erodible/airbrush presets opened by an older/capability-limited
  engine.** Fall back to the nearest standard tip and flag it.
- **Non-ASCII preset names.** UCS-2/UTF-16 names must round-trip.
- **Filesystem placement.** Libraries outside `Presets/Brushes` load but do not
  appear in the menu after restart (expected).
- **Concurrent edits.** Do not write an `.abr` while painting; preset changes must
  not mutate the active stroke's captured config.

## Parity acceptance criteria

- Given a CS6-era `.abr` library, `Load Brushes` appends its presets and their
  names/thumbnails appear; `Replace Brushes` replaces the list.
- Given a brushed preset with all dynamics set, saving to a library and
  reloading reproduces every setting (native round-trip exact; ABR round-trip
  best-effort with documented gaps).
- Given a temporary Diameter/Hardness change on a selected preset, re-selecting
  the preset restores its original values.
- Given `New Brush Preset`, the new preset survives a restart but is lost after a
  Reset Brushes / preferences reset.
- Given `Stroke Thumbnail`, each preset shows a sample stroke thumbnail.
- Given a renamed or deleted preset, the change is reflected in both the panel
  and the options-bar picker.
- Given the Preset Manager, brush libraries can be loaded/reset alongside other
  preset types.
- Given a tool preset, selecting it restores both the brush settings and the
  options-bar settings (opacity/flow/color/mode).
- Given a malformed or unknown ABR, import reports what parsed without crashing.
- Given a CS6 `v10 sub1` file, the version/subversion is identified correctly.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop Help reference (downloaded and text-extracted). Established:
  preset-brush definition; temporary picker changes; display options including
  Stroke Thumbnail; `Load`/`Replace`/`Reset`/`Save Brushes` and the library-file
  menu entry; `New Brush Preset`/Create New Brush; the Preferences-file storage
  caveat; Rename/Delete; the `Presets/Brushes` default location; `Clear Brush
  Controls`; instruction to save a durable set as a library; the distinction
  between brush presets and tool presets (a tool preset combines brush-tip
  characteristics with options-bar settings); Tool Preset picker /
  Tool Presets panel / Show All vs Current Tool Only; Preset Manager location
  (`Edit > Presets > Preset Manager` in CS6); `Copy Texture to Other Tools`.
  Primary source.
- `https://raw.githubusercontent.com/darkly-art/darkly/dev/docs/brush/abr-format.md`
  — "ABR Format: Public Analysis". Established: ABR header/version
  history (v1, v2, v6 sub1/2, v10 sub1 = CS6, v10 sub2 = CC); the 8BIM section
  architecture (`samp`, `patt`, `desc`, `phry`); computed (14-byte) and sampled
  tip records; PackBits RLE; the ActionDescriptor tree and its known brush keys;
  and that no implementation can reliably write modern ABR. Community reverse
  engineering, not Adobe.
- `https://glensmith.co.uk/photoshop/mixer-brush` — Mixer walkthrough.
  Established: creating a brush preset from the options-bar picker cog menu, and
  the Capture Brush Size / Include Tool settings / Include Color options at save
  time. Secondary.
- `https://community.wacom.com/en-co/complete-guide-to-photoshop-brushes-pt-3` —
  Wacom brush guide. Established: padlocks/Protect Texture as preset-retention
  behavior and that preset selection can carry or drop settings. Secondary.

## Open questions

- **ABR v10 sub1 exact layout.** The community analysis dates v10 sub1 to CS6 and
  v10 sub2 to CC, but the 47/301-byte header sizes and some descriptor encoding
  rules are heuristic. Confirm against a CS6-generated `.abr`.
- **Whether CS6 can import/write earlier ABR versions unchanged** and how it
  upgrades them is unverified.
- **Default CS6 brush libraries and preset names.** The shipped `.abr` set and
  its organization are not enumerated by the fetched source.
- **Computed (type 1) brush rendering in CS6.** Whether CS6 still emits computed
  presets or always sampled/computed-with-descriptor is unverified.
- **Pattern (`patt`) vector-mask trailing data.** Only partially understood;
  affects full fidelity of textured presets.
- **Descriptor key coverage.** Community coverage is ~60% of keys and ~40% of
  curve semantics; unknown keys must be preserved opaquely.
- **Native format scope.** We propose an open format rather than ABR writing;
  whether to also attempt ABR export is a product decision, not a parity fact.
- **Tool-preset color capture.** Whether CS6 tool presets capture the foreground
  color, the brush color, or neither exactly is unverified.
