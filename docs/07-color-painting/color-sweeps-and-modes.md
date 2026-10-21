# Foreground/Background Color, Color Panel Modes, and HUD Picker

- **Spec ID:** `CLR-012`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the **Color panel is resizable** (CS6 JDI), the **Adobe Color Picker accepts hexadecimal paste** with `#`/`0x` and shorthand `#123`, and the **HUD color picker** (Hue Strip / Hue Wheel) is available while painting. The foreground/background model itself is unchanged.
- **Depends on:** `03-tools/eyedropper-color-sampler-ruler.md` (`TOOL-015`), `02-ui-ux/panels/color-panel.md`, `02-ui-ux/panels/swatches-panel.md`, `07-color-painting/color-models.md`, `07-color-painting/gradient-presets.md` (`CLR-010`), `07-color-painting/pattern-presets.md` (`CLR-011`), `01-architecture/color-management.md`.

> Module and widget names below are **design proposals**. The repository contains
> a working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Facts not confirmed by a fetched CS6 source are marked *(inferred)*.

> This file is intentionally short. The full color picker UI is covered by the
> Eyedropper/color-picker tool spec; this file covers only the *color-sweep*
> mechanics (foreground/background, D/X, panel modes/spectrum, HUD) and
> cross-references the gradient (`CLR-010`) and pattern (`CLR-011`) specs.

## CS6 behavior

**Roles.** "Photoshop uses the foreground color to paint, fill, and stroke
selections and the background color to make gradient fills and fill in the
erased areas of an image. The foreground and background colors are also used by
some special effects filters." Source: CS6 reference, "About foreground and
background colors".

- **Defaults.** Foreground black, background white. In an **alpha channel** the
  defaults invert (foreground white, background black). (Help statement.)
- **Toolbox boxes.** Upper box = foreground, lower box = background. Click either
  to open the Adobe Color Picker. **Default Colors** icon restores black/white;
  **Switch Colors** icon reverses them. The keyboard shortcuts `D` (default) and
  `X` (swap) are the widely documented defaults; they are not enumerated in the
  fetched Help shortcut tables, so treat as *(community/common)* until a CS6
  shortcut capture confirms them.
- **Setting a color.** Via the Eyedropper tool, the Color panel, the Swatches
  panel, or the Adobe Color Picker (Help list). See `TOOL-015` for the
  eyedropper's sample size, sample scope, and Show Sampling Ring behavior.
- **Color panel (Window > Color, `F6`).** Shows the current foreground and
  background values; sliders edit them per color model; a ramp at the bottom
  samples a color directly (pointer becomes the eyedropper; `Alt`-click applies
  to the non-active box). Click the foreground/background box to make it active
  — when background is active, the Eyedropper changes background by default.
  When **Dynamic Color Sliders** is off, slider colors do not update live.
  (Source: "Color panel overview" / "Select a color in the Color panel".)
- **Color panel models & spectrum.**
  - **Sliders menu**: choose the model — RGB, HSB, CMYK, Lab, Grayscale, and
    **Web Color Sliders** (Help enumerates "Choose a Sliders option" and
    "Web Color Sliders"). Exact menu contents are *(inferred)* beyond the
    documented Web option.
  - **Spectrum**: RGB Spectrum, CMYK Spectrum, Grayscale Ramp, or **Current
    Colors** (the spectrum between the current foreground and background);
    **Make Ramp Web Safe** restricts the ramp to web-safe colors; `Shift`-click
    the ramp cycles spectra. (Source: "Change the spectrum displayed in the
    Color panel".)
  - Alerts above the ramp: a triangle for **non-printable (out-of-CMYK-gamut)**
    colors and a square for **not web-safe**; click the alert to snap to the
    nearest printable/web-safe color.
- **Adobe Color Picker.** Choose colors in **HSB, RGB, Lab, and CMYK** models
  (plus a hexadecimal `#` field), via a color slider + color field; the swatch
  shows the new color on top and the original below; alerts for non-web-safe and
  out-of-gamut. **Only Web Colors** restricts the field to web-safe colors. Hex
  accepts pasted `#aabbcc`, `0xAABBCC`, and shorthand `#123` (CS6 JDI). The
  picker can sample from anywhere on screen (pointer becomes the eyedropper).
  PANTONE/Trumatch/Focoltone/TOYO/ANPA/DIC **Color Libraries** are available.
  (Source: "Adobe Color Picker overview" / "Choose a color…" / "Choose web-safe
  colors" / JDI "Color Picker".)
- **HUD color picker (paint-time).** Requires **OpenGL**. Preference
  `General > HUD Color Picker` = **Hue Strip** (vertical) or **Hue Wheel**
  (circular). Invoke with `Shift+Alt+right-click` (Windows) or
  `Ctrl+Option+Cmd` (Mac) while a painting tool is active, then drag to pick hue
  and shade; after clicking, keys may be released; `spacebar` holds the shade
  while changing hue or vice versa; `Alt`/`Option` temporarily invokes the
  Eyedropper. (Source: "Choose a color while painting".)
- **Sweeps in fills/strokes.**
  - `Edit > Stroke` uses the **foreground** color for the border.
  - `Edit > Fill`: `Use` = Foreground Color, Background Color, Black, 50% Gray,
    White, Color (picker), Pattern, History, Content-Aware (CMYK note: filling
    with Black sets all channels 100% black). `Alt+Backspace` fills foreground,
    `Ctrl+Backspace` fills background (add `Shift` to preserve transparency).
    (Source: "Fill a selection or layer with color", "Keys for painting".)
  - Gradients default to using the gradient's own stops; the pattern/gradient
    engines are covered by `CLR-010` / `CLR-011`.
  - **D** with a layer mask active sets the mask's foreground/background to the
    grayscale defaults *(community/common)*.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Toolbox | Foreground/background boxes | — | Upper/lower; click opens picker |
| Toolbox | Default Colors icon | `D` *(community)* | Black/white (inverted in alpha channel) |
| Toolbox | Switch Colors icon | `X` *(community)* | Swap foreground/background |
| `Window > Color` | Panel | `F6` | Model sliders + ramp |
| Color panel menu | Sliders menu | — | RGB/HSB/CMYK/Lab/Gray/Web |
| Color panel menu | Spectrum options | — | RGB/CMYK/Grayscale/Current Colors, Make Ramp Web Safe |
| Color panel | Ramp | `Shift`-click cycles | Eyedropper; `Alt`-click = other box |
| Color panel | Alerts | — | Out-of-gamut triangle, not-web-safe square |
| Adobe Color Picker | Dialog | — | HSB/RGB/Lab/CMYK, hex, Only Web Colors, Color Libraries |
| `Edit > Preferences > General` | HUD Color Picker | — | Hue Strip / Hue Wheel |
| Canvas (painting tool) | HUD picker | `Shift+Alt+right-click` | OpenGL required |
| `Edit > Fill` | Dialog | `Shift+F5` | Foreground/Background/Color/Pattern/… |
| `Edit > Stroke` | Dialog | — | Foreground color |
| Canvas | Quick fill | `Alt+Backspace` / `Ctrl+Backspace` | `+Shift` preserves transparency |
| Swatches panel | Panel | — | Click = foreground; `Ctrl`/`Cmd`-click = background |
| Alpha channel | Context | — | Inverted black/white defaults |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Foreground color | color | Black | any color | White in an alpha channel |
| Background color | color | White | any color | Black in an alpha channel |
| Sliders model | enum | RGB *(inferred)* | RGB / HSB / CMYK / Lab / Grayscale / Web | Help documents "Sliders option" |
| Spectrum | enum | RGB *(inferred)* | RGB / CMYK / Grayscale / Current Colors | Shift-click cycles |
| Make Ramp Web Safe | bool | Off | On/Off | Restricts ramp |
| Dynamic Color Sliders | bool | On | On/Off | General preferences |
| HUD picker type | enum | Hue Strip *(inferred)* | Hue Strip / Hue Wheel | General preferences |
| Picker models | enum | HSB *(inferred)* | HSB / RGB / Lab / CMYK + hex | |
| Only Web Colors | bool | Off | On/Off | Picker |
| RGB components | int | — | 0–255 | Picker |
| HSB components | int | — | H 0–360°, S/B 0–100% | Picker |
| Lab components | int | — | L 0–100, a/b −128…+127 | Picker |
| CMYK components | percent | — | 0–100% each | Picker |
| Hex | string | — | `#rrggbb`, `0xrrggbb`, `#rgb` | CS6 paste support |
| Fill Use | enum | Foreground *(inferred)* | Foreground/Background/Black/50% Gray/White/Color/Pattern/History/Content-Aware | |
| Web-safe palette | set | — | 216 colors | |

## Algorithms & pipeline

- **Color state.** A single `foreground`/`background` pair lives in app state
  (not the document), except alpha channels which swap the defaults for display.
- **Panel ↔ picker ↔ swatches** are views over that pair; setting one box makes
  it the eyedropper's default target.
- **Model conversion.** HSB/RGB/Lab/CMYK/hex values are views of one color in
  the document working space; conversions route through `pictura_color` and the
  ICC transform (`01-architecture/color-management.md`). The picker's CMYK
  values are device/profile dependent, hence the out-of-gamut alert.
- **Web-safe mapping.** A non-web-safe color maps to the nearest of the 216
  palette entries for the alert; the ramp snap and slider ticks use the same
  palette.
- **HUD picking.** A GPU overlay maps pointer position to hue (strip/wheel axis)
  and shade (perpendicular axis), with `spacebar` locking one axis. Requires
  OpenGL/QRhi; CPU fallback = picker dialog.
- **Fill/stroke sweeps.** Foreground/background are the paint color source for
  fill, stroke, Paint Bucket (foreground), Eraser restoration (background), and
  the Historial/Art History brush source color where applicable.

## Rust module mapping

Proposed:

- `pictura_core::color_state` — `ColorState { foreground: Color, background:
  Color }`, `default_colors()`, `swap()`, `set_default()`.
- `pictura_color::convert` — model conversions shared with the picker and panel.
- `pictura_color::websafe` — `nearest_web_safe(Color) -> Color`, palette type,
  `is_web_safe`.
- `pictura_ui::hud_picker` — pure geometry/state machine for Hue Strip/Wheel;
  the GPU overlay lives in `pictura_render`.
- Boundary types: `Color`, `ColorSpace`, `ColorModel`, `WebSafePalette`.

## Qt6 component mapping

- `ColorStateModel` (`QObject`) — observable foreground/background pair; emits
  `colorsChanged`; shared by toolbox, panel, picker, and swatches.
- `ToolboxColorSwatch` (`QWidget`) — foreground/background boxes + Default/Switch
  icons.
- `ColorPanel` (`QWidget`) — sliders + ramp + alerts; `QComboBox` for Sliders and
  spectrum menus; resizable (CS6).
- `ColorPickerDialog` (`QDialog`) — color field/slider, model spin boxes, hex
  field (with paste validation), Only Web Colors, Color Libraries.
- `HudColorPickerOverlay` (QML/QQuickItem or GPU overlay) — OpenGL/QRhi-gated;
  hidden when unavailable.

## Data-model impact

- Foreground/background live in **application state**, persisted with
  preferences (`11-cross-cutting/preference-storage.md`), not the document.
- Alpha-channel display state inverts the defaults; this is view state only.
- No document serialization impact. Fill/stroke actions are history states.
- The picker "original color" is transient dialog state.

## Edge cases

- **Alpha channel active**: defaults invert (white fg / black bg).
- **Out-of-gamut / not-web-safe**: alerts must appear and snap correctly;
  Make Ramp Web Safe and Only Web Colors restrict input.
- **Web Color Sliders**: alert triangle unavailable (Help note).
- **Dynamic sliders off**: values update only on commit.
- **32-bit HDR**: the Extended HDR Color Picker applies to 32-bpc documents
  (see `04-image-ops/32-bit-hdr.md`).
- **GPU unavailable**: HUD picker disabled; the dialog picker still works.
- **CMYK document**: picker CMYK values must reflect the active profile.
- **Hex paste**: malformed or out-of-range hex is rejected, not truncated.

## Parity acceptance criteria

1. Given default state, foreground is black and background white; `D` restores
   them and `X` swaps them; with an alpha channel active the defaults invert.
2. Given the Color panel with background active, the Eyedropper changes the
   background color.
3. Given a non-web-safe foreground, the alert square appears and clicking it
   yields a web-safe color; with Only Web Colors (picker) or Make Ramp Web Safe
   (panel) on, only web-safe colors are selectable.
4. Given a pasted hex `#aabbcc`, `0xAABBCC`, or `#abc`, the RGB fields update to
   the correct values and the swatch matches.
5. Given an RGB spectrum, Shift-clicking the ramp cycles through the documented
   spectra and Current Colors spans foreground→background.
6. Given a painting tool and OpenGL available, `Shift+Alt+right-click` shows the
   HUD picker in the preference-selected form; dragging changes hue/shade and
   `spacebar` holds one axis.
7. With OpenGL/QRhi unavailable, the HUD picker is disabled and no overlay
   appears.
8. Given `Edit > Stroke`, the border uses the current foreground color; given
   `Alt+Backspace`, the selection fills with foreground, and `Ctrl+Backspace`
   with background (add `Shift` preserves transparency).

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  official CS6 Help reference, downloaded and text-extracted. Establishes:
  "About foreground and background colors" (paint/fill/stroke vs gradient/
  eraser roles, default black/white, alpha-channel inversion, four setting
  routes); "Choose colors in the toolbox" (boxes, Default Colors, Switch
  Colors); "Choose colors with the Eyedropper tool" (sample size, sample scope,
  Alt for background, temporary eyedropper); "Adobe Color Picker overview" and
  "Choose a color with the Adobe Color Picker" (HSB/RGB/Lab/CMYK/hex, slider +
  field, new-vs-original swatch, gamut/web-safe alerts, Only Web Colors,
  Color Libraries, screen sampling); "Choose web-safe colors" (216 colors,
  alert cube, Color panel web options); "Color panel overview" (foreground/
  background boxes, sliders, ramp, alerts, spectrum options, Make Ramp Web
  Safe, Shift-click cycle, Dynamic Color Sliders); "Choose a color while
  painting" (HUD picker, OpenGL requirement, Hue Strip/Wheel preference,
  Shift+Alt+right-click, spacebar axis lock, Alt eyedropper); "Fill a selection
  or layer with color" (Use menu, CMYK Black note, fill shortcuts); "Stroke a
  selection or layer with color" (foreground); "Keys for the Color panel"
  (Alt-click color bar = background, Shift-click = cycle); JDI "Color Picker"
  (hex paste `#`/`0x`, shorthand `#123`) and "Resizable Color panel".
- `https://photoshoptrainingchannel.com/tips/photoshop-default-foreground-background-colors`
  and `https://graphicdesign.stackexchange.com/questions/99863/photoshop-shortcut-for-swapping-foreground-and-background-colour`
  — secondary sources confirming the `D` (default) and `X` (swap) shortcuts,
  which the fetched Help tables did not enumerate.

## Open questions

- **`D`/`X` shortcut confirmation in the CS6 shortcut set** and the alpha-channel
  inversion interaction. *Resolves with:* a CS6 keyboard-shortcut capture.
- **Exact Color panel Sliders menu contents** (does CS6 expose separate
  Grayscale/Lab/web entries beyond what the Help documents?). *Resolves with:*
  a CS6 Color panel menu capture.
- **Default sliders model and spectrum** on a fresh CS6 install. *Resolves with:*
  first-run capture.
- **CMYK picker behavior** under different working-space profiles (values and
  gamut alert thresholds). *Resolves with:* controlled CMYK tests.
- **HUD default type** and whether the preference persists per workspace.
  *Resolves with:* CS6 observation.
- **Swatch click mapping** (click = foreground, `Ctrl`/`Cmd`-click = background)
  is Help-stated; confirm on Linux/Qt6 parity intent.
