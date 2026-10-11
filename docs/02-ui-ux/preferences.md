# Preferences

- **Spec ID:** `UI-010`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 restyles the Preferences dialog (dark theme, same left-hand
  pane list), adds the **Auto Save / Automatically Save Recovery Information** and **Save in
  Background** options to File Handling, moves **History Log** under General, expands **GPU
  Settings** (Mercury Graphics Engine) under Performance, adds **Enable Flick Panning**,
  **Animated Zoom**, **Place Or Drag Raster Images As Smart Objects**, and the **3D**
  preference pane (Extended). The Interface pane gains the four **Color Theme** swatches,
  **UI Font Size** options, **Enable Text Drop Shadows**, and a **canvas colour per screen
  mode**. CS5 already had the same pane list: General, Interface, File Handling, Performance,
  Cursors, Transparency & Gamut, Units & Rulers, Guides/Grid & Slices, Plug-ins and Type.
- **Depends on:** `01-architecture/qt6-ui-design.md`, `01-architecture/document-model.md`,
  `01-architecture/performance-targets.md`, `01-architecture/gpu-rendering-pipeline.md`,
  `02-ui-ux/keyboard-shortcuts.md` (`UI-011`), `02-ui-ux/accessibility.md` (`UI-012`),
  `04-image-ops/image-size.md`, `03-tools/hand-and-zoom.md`, `02-ui-ux/panels/timeline-panel.md`.

> Module, crate and type names are **design proposals**. Panel/setting behavior is taken from
> the CS6 Help reference; settings the CS6 PDF does not describe are marked *(secondary)* or
> *(unverified)* and collected under `## Open questions`.

## CS6 behavior

Photoshop CS6 stores "numerous program settings … including general display options,
file-saving options, performance options, cursor options, transparency options, type options,
and options for plug-ins and scratch disks. Most of these options are set in the Preferences
dialog box. Preference settings are saved each time you quit the application."
(`photoshop_reference.pdf`, "Preferences").

Preferences are opened from `Edit > Preferences` (Windows) / `Photoshop > Preferences` (Mac OS)
and then a pane is chosen from the submenu. The dialog itself lists the same panes down the left
side; the user can switch panes with the left-hand list, or with **Next** / **Prev**. The
**General** pane is the default. In CS6 the dialog follows the dark application theme.

### Opening and resetting preferences

- **Open a pane:** the menu path above, or `Ctrl/Cmd+K` for General. The CS6 Help documents
  only `Ctrl/Cmd+K`; the `Ctrl/Cmd+2`…`Ctrl/Cmd+9` pane shortcuts sometimes reported are not
  in the CS6 Help and collide with documented channel/zoom shortcuts (`Ctrl+1` = Magnify 100%,
  `Ctrl+2`…`5` = composite/RGB channels), so treat them as unverified (see
  `## Open questions`).
- **Switch pane:** click a pane in the left column, or click **Next** / **Prev**.
- **Reset all warning dialogs:** General pane → **Reset All Warning Dialogs** → OK.
- **Restore to default:**
  - Move/delete the preferences file; Photoshop recreates defaults on next launch.
  - Or hold `Alt+Ctrl+Shift` (Windows) / `Option+Command+Shift` (Mac OS) **while starting**
    Photoshop and confirm. CS6 Help notes this also resets custom shortcuts, workspaces and
    color settings.

### Persistence

CS6 keeps settings in the "Adobe Photoshop CS6 Prefs" file (`.psp`) inside the version-specific
Settings folder. Per the CS6 Help link "Preference filenames and locations in CS6" and Adobe
Community answers (CS6 13.0 x64):

| Platform | Path (community-verified for CS6) |
|---|---|
| Windows (Vista/7/8/10) | `%APPDATA%\Adobe\Adobe Photoshop CS6\Adobe Photoshop CS6 Settings\Adobe Photoshop CS6 Prefs.psp` |
| Windows (XP) | `Documents and Settings\[user]\Application Data\Adobe\Adobe Photoshop CS6\...` |
| macOS | `~/Library/Preferences/Adobe Photoshop CS6 Settings/Adobe Photoshop CS6 Prefs.psp` |

Some preset libraries (brushes, swatches, styles, patterns, contours) written from panels are
also stored in a preferences file, not the document.

### Pane: General

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Color Picker | menu | Adobe | Adobe vs. OS picker. |
| HUD Color Picker | menu | (OpenGL) | Hue Strip / Hue Wheel; requires OpenGL. |
| Image Interpolation | menu | Bicubic (CS6 adds **Bicubic Automatic** per secondary) | Nearest, Bilinear, Bicubic, Bicubic Smoother, Bicubic Sharper (+ Automatic in CS6). Drives Image Size / transform default. |
| Options: Auto-Update Open Documents | checkbox | off | — |
| Options: Beep When Done | checkbox | off | Beep after long operations. |
| Options: Dynamic Color Sliders | checkbox | on | Sliders preview color live. |
| Options: Export Clipboard | checkbox | on (secondary) | Exports PS clipboard to OS clipboard on quit. |
| Options: Use Shift Key For Tool Switch | checkbox | on | When on, `Shift`+letter cycles hidden tools; when off, the letter alone cycles. |
| Options: Resize Image During Place | checkbox | off | — |
| Options: Animated Zoom | checkbox | off | Requires OpenGL + `Enable OpenGL Drawing`. |
| Options: Zoom Resizes Windows | checkbox | off | — |
| Options: Zoom With Scroll Wheel | checkbox | off | — |
| Options: Zoom Clicked Point To Center | checkbox | off | — |
| Options: Enable Flick Panning | checkbox | on (secondary) | Momentum panning (Hand tool). |
| Options: Snap Vector Tools And Transforms To Pixel Grid | checkbox | off | — |
| Options: Place Or Drag Raster Images As Smart Objects | checkbox | on | When off, dragged/copied raster files become normal layers. |
| Options: Enable Gestures (Mac only) | checkbox | on | Trackpad rotate/zoom gestures. |
| History Log | toggle | off *(see note)* | Enables the edit history log. The CS6 Help also says per-session history data is saved as file metadata "by default", so the exact first-run checkbox state is *unverified*. |
| History Log: Save Log Items To | menu | Metadata | Metadata, Text File, Both (+ Choose… path). |
| History Log: Edit Log Items | menu | Sessions Only | Sessions Only, Concise, Detailed. |
| Reset All Warning Dialogs | button | — | Re-enables suppressed "Don't Show Again" messages. |

### Pane: Interface

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Color Theme | 4 swatches | 2nd from left (Dark Gray) | Black, Dark Gray, Medium Gray, Light Gray (swatches are unlabeled; names *(secondary)*). CS6 introduced the dark default. |
| Enable Text Drop Shadows | checkbox | off *(unverified)* | CS6 only; adds a white edge to panel lettering, most effective on the two dark themes. |
| Canvas colour: Standard Screen / Full Screen With Menus / Full Screen | menu ×3 | theme-linked (~50% gray / black in full-screen) | Black, Dark Gray, Medium Gray, Light Gray, Custom (default light blue); one setting per screen mode. |
| Screen Mode | menu | Standard Screen Mode | — |
| UI Font Size | menu | Small | Tiny, Small, Medium, Large; restart required. |
| Show Menu Colors | checkbox | on | Tints menu items by workspace/label. |
| Show Tool Tips | checkbox | on | In CS6 this lives in the Interface pane; in CC it moved to a Tools pane. |
| Show Transformation Values | menu | Top Right *(secondary)* | Never / Top Left / Top Right / Bottom Left / Bottom Right. |
| Panels: Auto-Collapse Iconic Panels | checkbox | off | CS6 Help calls it "Auto-Collapse Icon Panels". |
| Panels: Auto-Show Hidden Panels | checkbox | off | Hover the window (Win) or monitor (Mac) edge to reveal the hidden-panel strip. |
| Panels: Open Documents As Tabs | checkbox | on | Tabbed document windows. |
| Panels: Enable Floating Document Window | checkbox | on | — |
| Text: Show Font Names In English | menu | off (locale) | Also under Type pane in CS6. |
| Presentation Mode settings | group | — | Backdrop/screen options for the full-screen presentation *(secondary)*. |

### Pane: File Handling

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Image Previews (Mac: multiple types) | menu | Ask When Saving *(secondary)* | Never / Always / Ask When Saving; Mac adds Icon, Full Size, Macintosh Thumbnail, Windows Thumbnail. |
| File Extension (Windows) | menu | Use Lower Case *(secondary)* | Use Upper Case / Lower Case. |
| Append File Extension (Mac) | menu | Ask When Saving *(secondary)* | Never / Always / Ask When Saving + Use Lower Case. |
| Save As To Original Folder | checkbox | on *(secondary)* | Default save location. |
| Save In Background **(CS6)** | checkbox | on | Save without blocking the UI; the CS6 Help says disable it for the most consistent performance on large files. |
| Automatically Save Recovery Information **(CS6)** | checkbox + interval | 10 min | Crash recovery; the 10-minute default is primary ("Auto recover"), interval user-set. |
| Maximize PSD And PSB File Compatibility | menu | Ask *(secondary)* | Never / Always / Ask. Asked when a document contains features PS cannot represent. |
| Recent File List Contains | number | 20 *(secondary)* | `File > Open Recent` length; 0 disables. |
| Camera Raw Preferences | button | — | Opens the Camera Raw host preferences. |
| Version Cue / Adobe Drive / Clip services | toggles | off | CS6-era file services. |

### Pane: Performance

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Memory Usage | slider (MB / %) | 70 % | Range ~5–100 % of RAM; restart required. |
| Let Photoshop Use [n] MB | number | derived | Numeric mirror of the slider. |
| History States | number | 20 | 1–1000 *(secondary)*; more states cost RAM; restart required. |
| Cache Levels | number | 6 *(CS6-era secondary; Adobe's current KB says 4)* | 1–8 *(secondary)*; higher = faster screen redraw, slower on small edits. |
| Cache Tile Size | menu | 1024 K *(secondary)* | 128 K / 132 K / 1024 K. |
| GPU Settings: Enable OpenGL Drawing **(CS6: Mercury Graphics Engine)** | checkbox | on when a qualifying GPU is present | Greyed out when the card/driver is unsupported; CS6 pre-qualifies detected GPUs before use. |
| GPU Advanced Settings | sub-dialog | Normal *(secondary)* | Drawing Mode Basic / Normal / Advanced (+ vertical sync, anti-alias options) *(secondary)*. |
| Scratch Disks | checkbox list + order | Startup disk | Up to 4 scratch disks; first is primary. |

CS6 also exposes `Edit > Purge > All / Undo / Histories / Camera Raw` and `Edit > Clear` in the
History panel menu; these are separate commands, not preference fields, but they appear next to
the Performance topic in Help.

### Pane: Cursors

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Painting Cursors: Standard | radio | selected | Tool-icon pointer. |
| Painting Cursors: Precise | radio | — | Cross-hair pointer. |
| Painting Cursors: Normal Brush Tip | radio | selected | Outline ≈ 50 % affected area. |
| Painting Cursors: Full Size Brush Tip | radio | — | Outline ≈ 100 % affected area. |
| Painting Cursors: Show Crosshair In Brush Tip | checkbox | off | Cross-hair centred in the brush outline. |
| Painting Cursors: Show Only Crosshair While Painting | checkbox | off | Improves performance with large brushes. |
| Other Cursors: Standard / Precise | radio | Standard | For marquee, lasso, magic wand, crop, slice, patch, eyedropper, pen, gradient, line, paint bucket, magnetic lasso/pen, freeform pen, measure, color sampler. |

Painting cursors apply to: Eraser, Pencil, Paintbrush, Healing Brush, Rubber Stamp, Pattern
Stamp, Quick Selection, Smudge, Blur, Sharpen, Dodge, Burn, Sponge. `Caps Lock` toggles
standard ↔ precise where supported; `Alt+right-click`-drag resizes / changes hardness (OpenGL
preview required).

### Pane: Transparency & Gamut

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Transparency Settings: Grid Size | menu | Medium *(secondary)* | None (hides checkerboard), Small, Medium, Large. |
| Transparency Settings: Grid Colors | menu/picker | Light *(secondary)* | Light, Medium, Dark, Red, Custom (custom color picker). |
| Gamut Warning: Color | color | medium gray *(secondary)* | Color used to overlay out-of-gamut pixels. |
| Gamut Warning: Opacity | slider | 100 % *(secondary)* | Overlay strength. |

`View > Gamut Warning` toggles the overlay; the color/opacity are set here.

### Pane: Units & Rulers

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Rulers | menu | Inches *(US)* / cm *(metric)* | px, in, cm, mm, pt, pica, %, columns. |
| Type | menu | Points | Unit for the Type size field. |
| Print Resolution | number | 300 ppi *(secondary)* | Default target print resolution. |
| Screen Resolution | number | 72 ppi *(secondary)* | Used for print-size previews. |
| Point/Pica Size | radio | PostScript (72 pt/in) | PostScript (72) vs. Traditional (72.27). |

Changing units on the Info panel automatically changes the ruler units. Columns width/gutter
("Specifying columns for an image") is a document setting that reads the Units & Rulers values.

### Pane: Guides, Grid & Slices

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Guides: Color | menu/picker | Cyan | Any preset or Custom. Default per the CS6 Preferences screenshot on #294. |
| Guides: Style | menu | Lines *(secondary)* | Lines / Dashed Lines. |
| Smart Guides: Color | menu/picker | Magenta *(secondary)* | — |
| Grid: Color | menu/picker | Light Gray *(secondary)* | — |
| Grid: Style | menu | Lines *(secondary)* | Lines / Dashed Lines / Dots. |
| Grid: Gridline Every | number + unit | 1 in / 25 % *(secondary)* | Percent creates even divisions. |
| Grid: Subdivisions | number | 4 *(secondary)* | — |
| Slices: Line Color | menu | Light Blue *(secondary)* | Selected slices auto-contrast. |
| Slices: Show Slice Numbers | checkbox | on *(secondary)* | — |
| Slices: Show Slice Badges | checkbox | on *(secondary)* | — |

### Pane: Plug-ins

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Additional Plug-ins Folder | folder picker | (none) | Second location scanned for compatible plug-ins; the CS6 Help notes a **restart** is required for the plug-ins to take effect. |
| Filter Gallery plug-in / legacy filter controls | toggles | — | Controls legacy filter behaviour *(secondary)*. |
| Extension panels: Allow Extensions To Connect To The Internet | checkbox | off *(secondary)* | CS6 Extension panel internet access. |
| Enable Remote Connections | checkbox | off *(secondary)* | Debug/remote connection port. |

A plug-in installed elsewhere can be linked by placing a shortcut (Windows) or alias (Mac OS)
in the Plug-ins folder. If the plug-in list grows too long, newly installed plug-ins fall back
to `Filter > Other`.

### Pane: Type

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Show Font Names In English | checkbox | off | Displays Asian font names in English. |
| East Asian / Show Asian Text Options | checkbox | off | CS6 labels the East Asian engine choice; enables CJK options in the Character/Paragraph panels. |
| Middle Eastern | choice | off | CS6: recommended alternative engine for non-CJK Asian languages. |
| Enable Missing Glyph Protection | checkbox | on | Auto-substitutes a font with the needed glyph. |
| Use Smart Quotes | checkbox | on | Typographer's quotes for new type. |
| Text engine (Windows) | menu | System layout *(secondary)* | Windows / East Asian layout engines. |

Note: `Type > Font Preview Size` moved out of Preferences into the Type menu in CS6.

### Pane: 3D (Photoshop Extended only)

| Setting | Type | CS6 default | Notes |
|---|---|---|---|
| Interactive Rendering: Shadow Quality | menu | (GPU-dependent) *(secondary)* | Better OpenGL shadows; trade speed for quality. |
| Ray Tracer: Render Tile Size | menu | core-count based | Number of ray-trace tiles; default "set based on how many cores are in your computer". |
| Ray Tracer: High Quality Threshold | slider/number | *(secondary)* | Number of tiling passes before high-quality render; trades speed for quality. |

3D preferences are absent in Photoshop Standard.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Edit > Preferences > General` (Win) / `Photoshop > Preferences > General` (Mac) | Dialog pane | `Ctrl/Cmd+K` | Default pane; Reset All Warning Dialogs, History Log, zoom/interpolation options. |
| Preferences dialog, any pane | Dialog | `Next` / `Prev` buttons | Switch to the next/previous pane in the list (CS6 Help). |
| `Edit > Preferences > Interface` | Dialog pane | (none documented) | Theme, text drop shadows, canvas colour, font size, panels, gestures. |
| `Edit > Preferences > File Handling` | Dialog pane | (none documented) | Save/extension/recovery. |
| `Edit > Preferences > Performance` | Dialog pane | (none documented) | Memory, history & cache, GPU, scratch disks. |
| `Edit > Preferences > Cursors` | Dialog pane | (none documented) | Pointer styles. |
| `Edit > Preferences > Transparency & Gamut` | Dialog pane | (none documented) | Checkerboard + gamut warning. |
| `Edit > Preferences > Units & Rulers` | Dialog pane | (none documented) | Units, point/pica, resolutions. |
| `Edit > Preferences > Guides, Grid & Slices` | Dialog pane | (none documented) | Guides/grid/slice colors. |
| `Edit > Preferences > Plug-ins` | Dialog pane | (none documented) | Additional plug-ins folder. |
| `Edit > Preferences > Type` | Dialog pane | (none documented) | Text engines, glyph protection, smart quotes. |
| `Edit > Preferences > 3D` | Dialog pane | (none documented) | Extended only. |
| `Edit > Keyboard Shortcuts` | Dialog | `Ctrl/Cmd+Alt/Option+Shift+K` *(reported)* | Shortcut sets (see `UI-011`). |
| `Ctrl+Alt+Shift` (Win) / `Cmd+Option+Shift` (Mac) at launch | Startup gesture | — | Reset preferences. |
| Right-click ruler | Context menu | — | Quick unit change (routes to Units & Rulers). |

## Parameters & ranges

Key numeric/enumerated controls and their stated or reported ranges. Entries marked
*(secondary)* come from community/Adobe help rather than the CS6 PDF and must be verified
against a running CS6 before the spec reaches `Spec'd`.

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Color Theme | enum | Dark Gray (2nd of 4) | Black/Dark Gray/Medium Gray/Light Gray | Swatches unlabeled; names *(secondary)*. |
| UI Font Size | enum | Small | Tiny/Small/Medium/Large | Restart required. |
| Enable Text Drop Shadows | bool | off | on/off | Interface pane; CS6 only. |
| Canvas colour | enum/picker | theme-linked | Black/Dark Gray/Medium Gray/Light Gray/Custom | One per screen mode. |
| Image Interpolation | enum | Bicubic | Nearest/Bilinear/Bicubic/Bicubic Smoother/Bicubic Sharper/(CS6: Automatic) | Shared with Image Size dialog. |
| HUD Color Picker | enum | Strip | Hue Strip/Hue Wheel | OpenGL required. |
| Show Transformation Values | enum | Top Right *(secondary)* | 5 positions + Never | — |
| Memory Usage | % of RAM | 70 | ~5–100 *(secondary)* | Restart required. |
| History States | int | 20 | 1–1000 *(secondary)* | Affects undo depth + scratch usage; restart. |
| Cache Levels | int | 6 *(CS6-era; Adobe KB: 4)* | 1–8 *(secondary)* | Contested default. |
| Cache Tile Size | enum | 1024 K *(secondary)* | 128 K/132 K/1024 K *(secondary)* | — |
| Scratch Disks | ordered list | Startup | ≤4 disks *(secondary)* | First = primary. |
| Recent File List Contains | int | 20 *(secondary)* | 0–100 *(secondary)* | 0 disables Open Recent. |
| Auto Save interval | minutes | 10 | 5/10/15/30/60 *(secondary)* | CS6 crash recovery. |
| Gridline Every | number+unit | 1 in / 25 % *(secondary)* | any + px/in/cm/mm/pt/pica/% | % = even division. |
| Grid Subdivisions | int | 4 *(secondary)* | ≥1 | — |
| Print Resolution | ppi | 300 *(secondary)* | ≥1 | Default print target. |
| Screen Resolution | ppi | 72 *(secondary)* | ≥1 | Print-size preview. |
| Point/Pica Size | enum | PostScript | 72 / 72.27 pt-per-inch | — |
| Shadow Quality | enum | GPU-dependent | Off/Low/High *(secondary)* | Extended 3D. |
| Render Tile Size | enum | CPU-core based | small…large | Extended 3D. |

## Algorithms & pipeline

Preferences are an application-level key/value store, not a document transform. The pipeline is:
**startup** → load store (or seed defaults if absent/corrupt) → **mutate** via pane widgets →
**persist** on quit (or immediately for a few live options) → **consume** by feature modules.

- **Store format:** Adobe uses a proprietary `.psp` binary. Behavior parity does not require
  bit-compatibility; it requires the *observable* persistence contract: "settings are saved
  each time you quit"; wrong/corrupt preferences reproduce as defaults; deleting the store
  resets everything.
- **Startup reset gesture** intercepts launch before the store is read and moves/ignores the
  existing file.
- **Live vs. deferred:** most changes apply immediately; Memory Usage, History/Cache, UI Font
  Size and some GPU options require a restart (CS6 says so for several). The exact restart set
  is an open question.
- **Scratch-disk ordering** is an ordered list of mounted volumes; Photoshop writes its scratch
  file to the first with capacity, spilling to later entries.
- **History States** bounds the undo ring in memory/scratch; **Cache Levels** pick the image
  pyramid (see `04-image-ops/image-size.md`, `01-architecture/performance-targets.md`).
- Behavioral parity only; Adobe's exact default values for many fields are not published in the
  CS6 PDF. Do not guess them.

## Rust module mapping

- `crate::prefs::Preferences` — typed mirror of every pane; serializable. One struct per pane
  (`GeneralPrefs`, `InterfacePrefs`, `FileHandlingPrefs`, `PerformancePrefs`, `CursorPrefs`,
  `TransparencyPrefs`, `UnitsPrefs`, `GuidesGridSlicesPrefs`, `PluginsPrefs`, `TypePrefs`,
  `ThreeDPrefs`), each `#[derive(Serialize, Deserialize)]`.
- `crate::prefs::PrefsStore` — load/save to a platform path; atomic replace; corruption → defaults.
- `crate::prefs::PrefsSchema` — field metadata (key, type, default, range, pane, restart
  requirement). Drives both the Qt dialog and validation, so defaults live in one place.
- `crate::prefs::PreferencesService` — `watch`-style subscription: `get::<T>()`, `set(...)`,
  emits `PrefsChanged { key, old, new }` over a channel for consumers (zoom, cursors, GPU).
- `crate::prefs::paths::prefs_path()` — XDG-correct location on Linux:
  `$XDG_CONFIG_HOME/kooka-pictura/prefs.json` (or `~/.config/...`), encoding the CS6 layout as
  a *migration source*, not the on-disk format.
- `crate::prefs::import::cs6` — optional, independent best-effort importer that can read a user's
  CS6 `.psp` only if publicly documented later; off by default (see `00-overview/licensing-and-provenance.md`).
- Scratch disks/GPU: `crate::render::{GpuSettings, ScratchDiskConfig}` consumed by
  `01-architecture/gpu-rendering-pipeline.md`.

Types crossing the Rust↔Qt boundary: `GeneralPrefs`, `InterfacePrefs`, …, `PrefsChanged`,
`PrefsFieldMeta`. All are plain data; no Qt types leak into Rust.

## Qt6 component mapping

- `PreferencesDialog` (QDialog) — left `QListView` of panes + `QStackedWidget` of pane
  `QWidget`s; OK/Cancel/Next/Prev buttons. Widgets (not QML) for desktop-native dialog
  behaviour, native menus, and screen-reader exposure.
- `PreferencesPage` (abstract QWidget) — each pane subclasses it; builds controls from
  `PrefsSchema` metadata.
- `ColorSwatchGrid`, `UnitComboBox`, `ScratchDiskListWidget`, `HistoryStatesSpin`, `MemoryUsageSlider`
  — small reusable widgets.
- `PreferencesModel` (QAbstractItemModel) — pane list with icons/labels.
- `PrefsController` (`QObject`, `cxx-qt`/`qmetaobject-rs`) — marshals `PreferencesService` to
  Q_PROPERTYs and slots; emits `changed(key)`.
- Theme: CS6's four-step gray theme is a `QPalette` + `qss` variant; UI Font Size maps to a
  global `QApplication` font. High-DPI/theme integration is specified in `UI-012`.

## Data-model impact

- **Not document data.** Preferences never serialize into PSD/XMP and never enter the document
  undo stack. CS6 Help is explicit: "Program-wide changes, such as changes to panels, color
  settings, actions, and preferences, are not reflected in the History panel."
- **Application-level store** with its own schema version and migration. A `prefs_version`
  field allows forward migration; unknown keys are preserved on write where possible.
- **Per-workspace shortcuts** are a separate store (see `UI-011`); per-document guides/grid
  overrides (document grid settings, slices) belong to the document model, not here.
- **Undo:** preference edits are not undoable via the document History. A Preferences dialog
  Cancel must roll back uncommitted widget state.

## Edge cases

- **Missing/corrupt store:** recreate defaults; do not crash. The CS6 "damaged preferences"
  troubleshooting flow is the contract.
- **No writable config dir:** run with in-memory defaults and surface a warning; do not lose the
  current session.
- **GPU absent/unsupported:** `Enable OpenGL Drawing` is unavailable/greyed; all GPU-gated
  options (Animated Zoom, HUD picker, 3D, Oil Paint) fall back or disable. See
  `01-architecture/gpu-rendering-pipeline.md`.
- **Scratch disk full/removed:** reorder to next available; surface the CS6-style scratch error.
- **64-bit vs. 32-bit:** 32-bit builds cap Memory Usage (≈1.7–3.2 GB). Linux target is 64-bit
  only; keep a documented cap for parity tests.
- **Extended vs. Standard:** the 3D pane is absent in Standard; hide it rather than showing a
  dead pane.
- **Multiple monitors / mixed DPI:** UI Font Size and color theme are app-global; per-monitor
  scaling is a Qt/host concern (`UI-012`).
- **Reset gesture while another instance runs / file locked:** refuse to reset and warn.
- **Localization:** `Show Font Names In English`, Asian/Middle-Eastern engines, smart-quote
  default depend on locale; defaults are locale-sensitive.

## Parity acceptance criteria

1. Given a fresh profile, opening `Edit > Preferences > General` shows a dark-theme dialog whose
   left pane list is exactly: General, Interface, File Handling, Performance, Cursors,
   Transparency & Gamut, Units & Rulers, Guides/Grid/Slices, Plug-ins, Type (and 3D in Extended).
2. Given a changed setting, quitting and relaunching restores the changed value; deleting the
   store and relaunching restores CS6 defaults.
3. Given `Alt+Ctrl+Shift` (Win) / `Cmd+Option+Shift` (Mac) held at launch and confirmed, all
   panes report defaults afterward and custom shortcut/workspace/color sets are reset.
4. Given the Memory Usage / History States / Cache Levels fields, values outside the documented
   range are rejected; a restart-required change shows the CS6 restart prompt.
5. Given a GPU that fails the OpenGL probe, `Enable OpenGL Drawing` is disabled and exactly the
   GPU-dependent features are unavailable — no crash on toggling.
6. Given a document with a gamut-warning color set, `View > Gamut Warning` overlays
   out-of-gamut pixels using that color at that opacity.
7. Given `Automatically Save Recovery Information` on and a simulated crash, relaunch offers
   recovery of the last interval's work.
8. Given a locked/read-only config location, the app starts with defaults and warns; no settings
   are silently discarded without notice.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` (Adobe Photoshop CS6
  Help and tutorials, Feb 2013) — primary, downloaded and text-extracted. Establishes: Preferences
  overview and file behavior (the "About preferences" text still reads "Adobe Photoshop CS5
  Prefs file"); General (Beep When Done, History Log with Sessions Only/Concise/Detailed, zoom/
  interpolation/flick panning/Place Or Drag/Snap Vector/shift-key-tool-switch); Interface (Color
  Theme swatch, UI Font Size, Show Menu Colors, Show Tool Tips, Auto-Collapse Icon Panels,
  Auto-Show Hidden Panels, Restore Default Workspaces, Show Transformation Values); File Handling
  (Save In Background, Automatically Save Recovery Information — "Auto recover" default **ten
  minutes**, Maximize PSD/PSB, Recent File List); Performance (GPU Settings, Enable OpenGL
  Drawing); Cursors; Transparency & Gamut (grid size/color, gamut warning); Units & Rulers
  (Point/Pica, Type units); Guides/Grid/Slices (colors, Gridline Every, Subdivisions, Show Slice
  Numbers); Plug-ins (Additional Plug-ins Folder, restart required); Type (Show Font Names In
  English, East Asian (CS6) / Show Asian Text Options (CS5), Middle Eastern, Missing Glyph
  Protection default on, Use Smart Quotes); 3D (Render Tile Size, Shadow Quality, High Quality
  Threshold).
- `https://www.photoshopforphotographers.com/pscs6/downloads/Photoshop-interface.pdf` — Martin
  Evening, *Adobe Photoshop CS6 for Photographers* (sample chapter): the Interface preferences
  hold four themes, UI font size (changes apply only after a relaunch), **Enable Text Drop
  Shadows**, and a **canvas colour per screen mode** (Standard Screen / Full Screen with Menus /
  Full Screen); the dark default's canvas is almost black; Mac keeps a `Window > Application
  Frame` toggle.
- `https://www.photoshopessentials.com/basics/essential-photoshop-preferences-beginners/` —
  secondary, cross-version (CC/CS6): Export Clipboard (default on), Color Theme (default 2nd
  swatch), UI Font Size (default Small; Tiny/Small/Medium/Large; restart), Show Tool Tips
  (Interface in CS6, Tools in CC), Use Shift Key for Tool Switch (default on; General in CS6),
  Auto Save (10-minute default, CS6), Recent File List (default 20, max 100), Memory Usage
  (70 % default, restart), History States (CS6 default 20; CC 50), Scratch Disks.
- `https://www.photoshopessentials.com/basics/interface-cs6/` — secondary: four Color Theme
  swatches with the default second from the left, and the canvas/pasteboard palette.
- `https://retouchingacademy.com/how-to-set-preferences-memory-usage-for-peak-performance-photoshop-cs6-cc/`
  — secondary, CS6/CC: Memory Usage guidance (PC 50–55 %, Mac 70–75 %), History States default
  20, **Cache Levels default 6**, scratch-disk ordering, Purge behavior, 32-bit memory cap.
- `https://www.howtogeek.com/309022/how-to-extend-history-states-in-photoshop-and-ctrlaltz-forever`
  — secondary: History States by default is 20.
- `https://macperformanceguide.com/OptimizingPhotoshopCS6-configuring.html` — secondary, CS6:
  Performance-pane screenshot; memory ~70–72 %; cache tile 1024 K; recommends disabling the GPU.
- `https://photoshopguides.github.io/Performance` — secondary (CC): Cache Levels default **4**,
  tile sizes 128 K / 132 K / 1024 K, and the three auto-optimize presets.
- `https://www.properproof.com/photoshop/guides/Adobe%20Photoshop%20%20%20Default%20keyboard%20shortcuts.htm`
  — reproduction of the CS6 default key list: `Ctrl+1` is Magnify 100 % and there are **no
  documented Preferences-pane shortcuts** (relevant to the per-pane shortcut question).
- `https://community.adobe.com/questions-712/how-to-reset-preferences-if-i-have-the-legacy-version-1140349`
  — Adobe Community (CS6 13.0 x64): Ctrl+Alt+Shift reset gesture and Windows preferences path
  `…\AppData\Roaming\Adobe\Adobe Photoshop CS6\Adobe Photoshop CS6 Settings`.
- `https://community.adobe.com/t5/photoshop-ecosystem-discussions/how-do-i-save-preference-changes-in-ps6/m-p/8864474`
  — Adobe Community (search result; not directly fetched): macOS preferences path
  `~/Library/Preferences/Adobe Photoshop CS6 Settings`.
- `https://www.oreilly.com/library/view/photoshop-cs6-visual/9780132983037/ch24.html` — search
  result only (HTTP 403, not fetched): the CS6 Visual QuickStart Guide's chapter 24 lists the pane
  order General, Interface, File Handling, Performance, Cursors, …. Used only to cross-check pane
  order and naming, not as a fact source.
- Qt 6 documentation `https://doc.qt.io/qt-6/accessible.html` and Context7 `/websites/doc_qt_io_qt-6_8`
  (`QAccessible`) — Qt6 widget/dialog and accessibility guidance used for the design proposals.

## Open questions

- **Remaining CS6 defaults not yet pinned:** Image Previews, Maximize PSD/PSB compatibility,
  transparency grid size/colour, and the guide/grid/slice colours. *Resolve:* read a known-fresh
  CS6 profile or a VM install.
- **Cache Levels default is contested:** CS6-era sources say **6**, Adobe's current KB says **4**.
  *Resolve:* inspect a fresh CS6 Performance pane (a screenshot or VM).
- **Per-pane keyboard shortcuts** (`Ctrl+2`…`Ctrl+9`) are **not documented** in the CS6 Help, and
  `Ctrl+1`…`Ctrl+5` are already Magnify 100 % / channel shortcuts. Treat any "reported" pane
  shortcut as unverified (likely non-existent). *Resolve:* a CS6 session or the exported CS6
  shortcut list.
- **Which settings require a restart in CS6.** The Help and secondary sources name Memory Usage,
  History States and UI Font Size; a definitive list is not published. *Resolve:* systematic
  toggle-and-observe on CS6.
- **GPU Advanced Settings** (Drawing Mode Basic/Normal/Advanced, vertical sync) exact options and
  defaults. *Resolve:* the CS6 GPU FAQ page (Adobe) or a machine with a supported GPU.
- **Cache Levels / History States numeric ranges** are secondary-sourced (1–8 and 1–1000).
  *Resolve:* inspect the CS6 spinbox ranges.
- **History Log first-run default.** The Help implies per-session logging to metadata is on by
  default, but the checkbox default is commonly described as off. *Resolve:* a fresh CS6 profile.
- **File Handling CS6-only additions** — whether `Save in Background` and
  `Automatically Save Recovery Information` have additional sub-options. *Resolve:* the CS6 Help
  File Handling page or a capture.
- **Persistence format and interop:** should Kooka Pictura offer any CS6 `.psp` import? Legal and
  legal review needed (`00-overview/licensing-and-provenance.md`).
- **Linux preference location:** confirm XDG (`$XDG_CONFIG_HOME`) vs. Qt `QSettings` native
  format; decide JSON vs. TOML for the spec's reference store.
