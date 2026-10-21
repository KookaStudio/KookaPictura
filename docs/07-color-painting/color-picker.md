# Adobe Color Picker

- **Spec ID:** `CLR-002`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — CS6 adds hex-field paste with `#`/`0x` prefixes (`#aabbcc`, `0xAABBCC`) and 3-digit shorthand (`#123`), and makes the Color panel resizable. The picker's model set and layout are otherwise long-standing.
- **Depends on:** `CLR-001` color-models, `ARCH-007` color-management, `01-architecture/gpu-rendering-pipeline.md`, `02-ui-ux/panels/color-panel.md`, `03-tools/eyedropper-color-sampler-ruler.md`, `07-color-painting/swatches-and-libraries.md`, `10-workflow-io/color-settings.md`, `04-image-ops/32-bit-hdr.md`, `00-overview/feasibility-and-non-goals.md`.

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Facts not confirmed by a fetched CS6 source are marked
> *(inferred)*.

## CS6 behavior

The **Adobe Color Picker** is the default modal color dialog. It sets the foreground color, background color, text color, and target colors for tools, commands, and options. It offers **four models — HSB, RGB, Lab, and CMYK** — and shows the numeric values for HSB, RGB, Lab, CMYK, and hexadecimal all at once.

### Anatomy

From the CS6 Help figure labels (A–I):

| Label | Element |
|---|---|
| A | Picked color |
| B | Original color |
| C | Adjusted color |
| D | Out-of-gamut alert icon |
| E | Not a web-safe color alert icon |
| F | "Displays only web-safe colors" (the **Only Web Colors** option) |
| G | Color field |
| H | Color slider |
| I | Color values |

The rectangle beside the slider shows the **adjusted color in the top section and the original color in the bottom**. The color box, color field, and color slider update all numeric readouts live as the user drags.

### Interaction

- To choose a color, set one component by clicking the color slider or dragging its triangle, then set the other two by moving the circular marker or clicking in the color field.
- **HSB**: hue is a **0°–360°** angle; saturation and brightness are percentages; in the field, saturation increases left→right and brightness bottom→top. Selecting **S** or **B** makes the field display that component for further adjustment (so the slider/field axis pairing follows the radio selection).
- **RGB**: choose R, G, or B; that channel becomes the slider (0 at bottom, 255 at top) and the field shows the other two components.
- **Lab**: L 0–100, a/b −128…+127; slider/field optional.
- **CMYK**: enter C/M/Y/K percentages, or use the slider and field.
- **Hexadecimal**: enter an RGB triple as `00`–`ff` pairs (`000000` black, `ffffff` white, `ff0000` red). CS6 also **accepts pasted values prefixed `#` or `0x`** and **3-digit shorthand** (`#123`).
- **Pick outside the dialog**: moving the pointer over the document window turns it into the **Eyedropper**; clicking samples the image. Holding the mouse after clicking lets the Eyedropper be dragged "anywhere on your desktop," releasing to pick.

### Web-safe and gamut handling

- **Only Web Colors** (lower-left) restricts selection to the **216-color** web-safe palette.
- A non-web color raises an **alert cube**; clicking it "select[s] the closest web color."
- An out-of-gamut color raises an **alert triangle** with a swatch showing the closest CMYK equivalent; clicking it substitutes that value. Printable colors are defined by the current **CMYK working space** (`ARCH-007`).

### Color Libraries (spot colors)

The picker's **Color Libraries** button opens the **Custom Colors** dialog, initially showing the color closest to the current picker selection:

1. Choose a **Book** from the `Book` menu.
2. Locate the color by entering the **ink number** or dragging the triangles along the scroll bar.
3. Click the desired color patch.

CS6-supported systems (from the Help): **ANPA-COLOR**, **DIC Color Guide**, **FOCOLTONE** (763 CMYK colors), **HKS** (E, K, N, Z scales), **PANTONE MATCHING SYSTEM** (1,114 colors), **TOYO Color Finder 1050** (>1,000 colors), and **TRUMATCH** (>2,000 computer-generated CMYK colors).

> **Spot-color caveat:** except in Duotone mode, Photoshop prints spot colors onto CMYK process plates; true spot-color plates require spot color channels.

### Changing the picker

`Edit > Preferences > General` has a **Color Picker** menu; selecting an operating-system picker or a third-party plug-in replaces the Adobe Color Picker.

### HUD color picker

The heads-up-display picker lets you pick colors quickly while painting in the document window, where the surrounding image colors provide useful context.

- **Requires OpenGL.**
- Type is set in `Edit > Preferences > General` → **HUD Color Picker**: **Hue Strip** (vertical) or **Hue Wheel** (circular).
- Invoke with a painting tool active: **`Shift+Alt+right-click`** (Windows) or **`Control+Option+Command`** (macOS), then click in the document window to display it and drag to pick **hue and shade**. The pressed keys may be released after clicking; **Spacebar** temporarily holds the chosen shade while selecting a new hue (and vice versa).
- **`Alt`/`Option`** switches to the Eyedropper to sample from the image.

### HDR Color Picker (32-bpc)

With a 32-bpc document open, clicking the foreground/background box (or other color swatch) opens the **HDR Color Picker** instead. It "accurately view[s] and select[s] colors for use in 32-bit HDR images":

- The lower part behaves like the regular picker (color field, slider, HSB/RGB entry).
- An **Intensity slider** boosts/reduces brightness; the value plus intensity converts to the **32-bit floating-point** numbers written to the document. Intensity stops correspond inversely to exposure stops, so boosting document exposure by N stops and reducing intensity by N stops preserves appearance.
- A **Preview area** shows swatches at different exposures; **Preview Stop Size** sets the increment (e.g. 3 → −9, −6, −3, +3, +6, +9) and **Relative to Document** anchors them to the document exposure.
- Exact 32-bit RGB values can be typed directly into the 32-bit value fields.
- **Add to Swatches** adds the color to the Swatches panel.

### Recently used swatches

The CS6 Help documents **no "recently used" swatch row inside the Adobe Color Picker or Color panel**. Persistence of ad-hoc added colors is via the Swatches panel, which writes new colors to the Photoshop preferences file so they persist between editing sessions. A recent-colors/Color-Themes UI is a later Creative Cloud feature and is treated as **non-goal** here (see Open questions).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox fg/bg color boxes | Button | — | Opens the picker |
| Color panel Set Foreground/Background Color | Button | — | Opens the picker |
| Options-bar color swatches | Swatch | — | Opens the picker |
| Adjustment dialog eyedroppers | Swatch | — | Opens the picker |
| `Edit > Preferences > General > Color Picker` | Preference menu | — | Chooses Adobe / OS / third-party picker |
| `Edit > Preferences > General > HUD Color Picker` | Preference menu | — | Hue Strip / Hue Wheel |
| Document window (painting tool) | HUD overlay | `Shift+Alt+RMB` / `Ctrl+Opt+Cmd` | Requires OpenGL |
| Adobe Color Picker — Color Libraries | Button → dialog | — | Spot-color books |
| Adobe Color Picker — Add To Swatches | Button | — | Adds to Swatches panel |
| Adobe Color Picker — Only Web Colors | Checkbox | — | 216-color restriction |
| HDR Color Picker | Dialog | — | 32-bpc documents only |
| Swatches panel | Panel | — | Applies chosen colors (`CLR-003`) |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Model selection | radio | HSB | H/S/B, R/G/B, L/a/b, CMYK | Governs slider/field axes |
| H | degrees | 0 | 0 … 360 | Hue strip/wheel |
| S, B | percent | 100 | 0 … 100 | HSV saturation / value |
| R, G, B | int | 0/0/0 | 0 … 255 | 8-bpc |
| L | int | 0 | 0 … 100 | Lab |
| a, b | int | 0 | −128 … +127 | Lab |
| C, M, Y, K | percent | 0 | 0 … 100 | Working-space dependent |
| `#` hex | text | 000000 | `#`, `0x`, 3-digit shorthand | CS6 paste/shorthand |
| Only Web Colors | bool | off | on / off | 216-color snap |
| Color field / slider / marker | 2-D control | — | per selected model | Live numeric sync |
| New vs original swatch | read-out | — | — | Top = adjusted, bottom = original |
| Gamut alert | action | — | click triangle | Closest CMYK equivalent |
| Web alert | action | — | click cube | Closest web-safe color |
| Color Libraries Book | combo | PANTONE (inferred) | ANPA / DIC / FOCOLTONE / HKS / PANTONE / TOYO / TRUMATCH | Custom Colors dialog |
| Ink number | field / scroll | — | per book | Locates the patch |
| HUD type | enum (pref) | Hue Strip (inferred) | Hue Strip / Hue Wheel | Requires OpenGL |
| HDR Intensity | slider | 0 | stops | 32-bpc only |
| HDR Preview Stop Size | int | 3 (example) | stop increment | 32-bpc only |
| HDR Relative to Document | bool | — | on / off | 32-bpc only |
| HDR 32-bit RGB | float fields | — | float | Exact HDR values |

## Algorithms & pipeline

The picker is a self-contained **color state machine** plus three evaluators. Adobe's exact numerics are closed; behavior parity only.

### State

One authoritative color is held in a canonical representation (`f32` linear or the working RGB space) plus the **original** color for the B/C swatch. Every edit updates the authority through the model's conversion (`CLR-001`), then reprojects all readouts (HSB/RGB/Lab/CMYK/hex) and the warnings. Reprojection must be computed from the authority, never by chaining readout→readout, to avoid cumulative round-trip drift.

### Field / slider mapping

- The selected radio chooses which component the **slider** controls; the **field** is the 2-D cross-product of the other two.
- HSB field: horizontal = saturation, vertical = brightness (bottom→top).
- RGB slider: 0 at bottom, 255 at top.
- The circular marker's position is the quantized field coordinate; a click sets it, and keyboard entry snaps it.

### Warning evaluators

- **Gamut**: `rgb → cmyk(profile) → rgb'`; the alert is shown when the color is not representable in the current CMYK working space, and the swatch is the profile's closest CMYK value (`ARCH-007`).
- **Web-safe**: 216-color cube membership; nearest-color snap uses per-channel rounding to the web-safe lattice *(inferred)*.
- Recompute on every authoritative change.

### Color Libraries

The dialog is a **static lookup table** per book: an ordered list of `(ink_number, name, color_value)`. The "closest color" shown on open is a nearest-neighbor search in Lab *(inferred; the Help says only "the color closest to the color currently selected")*. Book data is bundled preset data, not document data (`CLR-003`).

### HUD

A transient, canvas-anchored overlay. **Hue Strip** maps vertical position to hue; **Hue Wheel** maps angle to hue; a second (shade/value) axis is read from the perpendicular drag. It writes the foreground color directly and closes. Requires an OpenGL/painting context (`01-architecture/gpu-rendering-pipeline.md`); without OpenGL the preference and HUD are unavailable.

### HDR

The 32-bpc path is the regular field/slider producing an 8/16-bit-like color, multiplied through the **Intensity** stop scale into an unbounded `f32` RGB triple. Preview swatches are the same color shown at `± k·stop_size` exposure stops; `Relative to Document` folds in the document's 32-bit exposure preview (`04-image-ops/32-bit-hdr.md`).

## Rust module mapping

- `pictura_color::picker` — `PickerState { authority: ColorValue, original: ColorValue, model: ColorModel }`; pure conversion/validation, no UI.
- `pictura_color::picker::readouts` — `Readouts { hsb, rgb, lab, cmyk, hex }`; recomputed atomically from `authority`.
- `pictura_color::library` — `SpotBook { id, name, entries: Vec<SpotEntry> }`, `SpotEntry { ink_number, name, value: Lab }`, `nearest(&ColorValue) -> &SpotEntry`; loader for the bundled book data.
- `pictura_color::hud` — `HudModel { kind: HueStrip | HueWheel, hue, shade }` and gesture-to-color mapping.
- `pictura_color::hdr_picker` — `HdrColor { rgb32: [f32; 3], intensity_stops: f32, preview_stop_size, relative_to_document }`.
- `pictura_core::prefs` — `ColorPickerChoice { Adobe, System, Plugin(PluginId) }`, `HudKind`.
- `pictura_core::command` — `SetForegroundColor { value }` / `SetBackgroundColor { value }`; `AddSwatch { name, value }` routed to `CLR-003`.

Crossing types: `ColorValue`, `ColorModel`, `Readouts`, `SpotEntry`, `HdrColor`, `ColorPickerChoice`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `AdobeColorPickerDialog` | `QDialog` | Modal host; OK/Cancel; Add To Swatches; Color Libraries |
| `ColorModelButtons` | `QButtonGroup` | H/S/B, R/G/B, L/a/b, CMYK radio set |
| `ColorSlider` | custom `QWidget` | Component slider with gradient and triangle handle |
| `ColorField` | custom `QWidget` | 2-D saturation/brightness (or model equivalent) field + circular marker |
| `ColorReadoutGrid` | `QGridLayout` | HSB/RGB/Lab/CMYK/hex spin/line fields with live sync |
| `ColorPreviewSwatch` | custom `QWidget` | Adjusted (top) / original (bottom) split swatch |
| `GamutAlertButton` | `QToolButton` | Triangle + closest-CMYK tooltip/swatch; click applies |
| `WebSafeAlertButton` | `QToolButton` | Alert cube; click snaps |
| `ColorLibraryDialog` | `QDialog` | Book combo, ink-number field, patch grid, scroll handles |
| `HudColorPicker` | `QQuickWidget` overlay (or canvas overlay) | Hue strip/wheel; writes foreground color |
| `ColorPickerController` | `QObject` | Marshals `PickerState` ↔ widgets; emits `colorAccepted` to Rust |
| `PickerPreferenceCombo` | `QComboBox` | Adobe / system / plugin picker selection |

Widgets for the modal dialog (dense, keyboard-numeric). The HUD is canvas-adjacent and may use QML/Qt Quick for the radial wheel; the strip can be a custom `QWidget`. This split follows `01-architecture/qt6-ui-design.md`.

## Data-model impact

- **Foreground/background color** are session/workspace state (preference-adjacent), not per-document pixels; PSD does not store them (inferred).
- **Added swatches** mutate the Swatches panel (and are persisted to preferences) — see `CLR-003`; the picker only initiates the add.
- **Spot colors** chosen here become spot channels or spot-color swatches, not document color-mode channels; converting the document to CMYK maps them to process values (except Duotone).
- **Picker preference / HUD type** live in Preferences (`10-workflow-io/color-settings.md` / `11-cross-cutting/preference-storage.md`).
- **Undo:** opening and cancelling the picker records nothing; accepting a color is not a document History state unless the color is applied destructively by a subsequent command.
- **No new document fields.** HDR values are `f32` and only meaningful in 32-bpc documents.

## Edge cases

- **8/16/32-bit** — 8/16-bit pickers produce integer-valued colors; 32-bpc opens the HDR picker with float fields and unclamped values.
- **CMYK in non-CMYK documents** — CMYK fields are a working-space projection; editing them must round-trip through the profile and may re-introduce gamut alerts.
- **Web-safe + gamut together** — both alerts can be present; each button applies only its own correction (inferred).
- **Hex shorthand** — `#123` must expand to `#112233`; invalid hex input must not corrupt state.
- **Missing OpenGL** — HUD picker unavailable; preference disabled and documented, not silently failing.
- **No CMYK profile / unsupported working space** — gamut evaluation must degrade gracefully (no false alerts).
- **Empty spot-book list / missing bundle** — Color Libraries disabled with a clear message.
- **16-bpc precision** — field/slider quantization must use the document's effective precision; no hidden 8-bit conversion.
- **Very large HDR values / NaN** — clamp non-finite input at the boundary.
- **Modal vs modeless** — the picker is modal; it must not freeze the app on cancellation and must release the sampler.
- **Linux picker substitution** — an OS/portal color picker may be selected by preference; behavior of that path is delegated to the platform (see Open questions).
- **Keyboard-only** — every control must be reachable and the numeric fields editable without a mouse (`02-ui-ux/accessibility.md`).

## Parity acceptance criteria

1. Given HSB `(0°, 100%, 100%)`, the picker shows RGB `(255, 0, 0)` and hex `ff0000`.
2. Given the RGB model with R selected, moving the slider top→bottom changes R 255→0 while the field shows the G×B plane.
3. Given a non-web color with `Only Web Colors` off, the alert cube appears and clicking it yields a color whose channels are all web-safe.
4. Given an out-of-gamut RGB color, the alert triangle's swatch equals the closest CMYK value for the current CMYK working space (same result as `CLR-001`).
5. Given the `Color Libraries` dialog with a chosen book, typing an ink number selects the matching patch and accepting it sets a spot color.
6. Given a 32-bpc document, the foreground box opens the HDR picker; entering 32-bit RGB values and an intensity yields the matching float color, and preview swatches follow `Preview Stop Size` / `Relative to Document`.
7. Given a painting tool with OpenGL available, `Shift+Alt+RMB` (Windows binding) shows the HUD; dragging sets hue+shade and releasing closes it; with OpenGL unavailable the HUD is not offered.
8. Given an image open, moving the pointer over the document from the picker turns it into the Eyedropper and clicking samples a color into the dialog.
9. Given a color added via **Add To Swatches**, it appears in the Swatches panel and persists across restart via preferences (`CLR-003`).
10. Given the hex field, pasting `#aabbcc` or `0xAABBCC` sets the same color, and `#123` expands as shorthand.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference. Established: the picker's four models and simultaneous HSB/RGB/Lab/CMYK/hex display; the A–I anatomy (picked/original/adjusted color, gamut alert, web-safe alert, Only Web Colors, field, slider, values); field/slider interaction per model; HSB 0°–360° and percentages with saturation left→right and brightness bottom→top; RGB 0–255 slider orientation; Lab ranges; CMYK percentages; hex 00–ff with `000000`/`ffffff`/`ff0000` examples; sampling outside the dialog and desktop dragging; the 216 web-safe colors and alert-cube substitution; the gamut triangle + closest-CMYK swatch tied to the CMYK working space; the Color Libraries / Custom Colors flow and the ANPA, DIC, FOCOLTONE, HKS, PANTONE, TOYO, TRUMATCH book list; the spot-to-CMYK print caveat; the General preference to substitute the OS/third-party picker; the HUD picker (OpenGL requirement, Hue Strip/Hue Wheel preference, `Shift+Alt+right-click` / `Control+Option+Command`, Spacebar shade lock, Alt/Option eyedropper); the HDR Color Picker (Intensity slider, preview stop size, Relative to Document, 32-bit fields, Add To Swatches).
- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — same file, "Productivity enhancements (JDI's) in CS6 → Color Picker": hex paste with `#`/`0x` and `#123` shorthand; "Resizable Color panel."
- `https://colorslurp.com/blog/a-complete-guide-to-ase-files` — secondary/community: Photoshop's Swatches panel menu uses **Import Swatches** / **Save Swatches for Exchange**, corroborating the picker's "Add To Swatches" link to library management (cross-check for `CLR-003`).

Not fetched (HTTP 403 from this environment): `helpx.adobe.com` "Customizing color pickers and swatches" page linked from the CS6 PDF.

## Open questions

- **"Recently used" swatches** — not documented for CS6. Decide whether to add a recent-colors affordance (later CC "Adobe Color Themes" behavior) or keep CS6 parity only. *Resolves with:* a CS6 picker/panel capture or the archived CS6 help page.
- **Exact slider/field axis assignment** for the S and B radio options (which component becomes the slider) is paraphrased from the Help. *Resolves with:* a CS6 picker capture.
- **Default Color Libraries Book** (PANTONE is assumed) and the exact book list ordering. *Resolves with:* a CS6 Color Libraries capture.
- **Nearest-color algorithm** for books and for web-safe snapping (Lab nearest vs channel lattice) is *(inferred)*. *Resolves with:* CS6 comparison tests.
- **Linux/portal color-picker substitution** — what `ColorPickerChoice::System` maps to on X11/Wayland, and whether it is offered at all. *Resolves with:* a platform spike.
- **HDR picker on 16-bpc documents** — whether the standard picker's fields are 16-bit. *Resolves with:* a CS6 16-bpc capture.
- **Paste-parse strictness** for `#`/`0x` (case, whitespace, invalid input). *Resolves with:* CS6 behavior tests.
- **HUD availability on Wayland/Qt6** — whether the HUD can run without a native OpenGL context. *Resolves with:* `01-architecture/gpu-rendering-pipeline.md`.
