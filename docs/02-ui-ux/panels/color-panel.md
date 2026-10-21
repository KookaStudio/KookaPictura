# Color Panel

- **Spec ID:** `PAN-010`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `Changed` — the Color panel is now **resizable** (CS6 JDI). The slider models, spectrum options, ramp gestures, and alerts are unchanged from CS5.
- **Depends on:** `CLR-001` color-models, `CLR-002` color-picker, `CLR-003` swatches-and-libraries, `CLR-004` histogram-and-info, `ARCH-003` qt6-ui-design, `ARCH-007` color-management, `ARCH-008` document-model, `03-tools/eyedropper-color-sampler-ruler.md`, `02-ui-ux/preferences.md`.

> This document owns the **Color panel UI surface** only. The color value model,
> conversions, gamut/web-safe predicates, and picker are specified in `CLR-001`
> and `CLR-002`; the Info panel gamut flag in `CLR-004`. All crate, module,
> widget, and type names below are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. Facts not confirmed by a fetched CS6 source are marked *(inferred)*.

## CS6 behavior

`Window > Color` (shortcut `F6`) opens the **Color panel**. It displays the color values for the current foreground and background colors, lets the user edit those colors using different color models, and offers a spectrum of colors in the color ramp at the bottom of the panel.

**Default placement (CS6 Essentials workspace).** The Color panel is at the **top of the right-hand main panel column**, grouped with the **Swatches** tab; Color is the default-active tab. (Source: Photoshop Essentials, *Managing Panels In Photoshop CS6*.)

### Anatomy

From the CS6 Help figure labels (A–D):

| Label | Element |
|---|---|
| A | Foreground color box |
| B | Background color box |
| C | Slider(s) — one per component of the active model |
| D | Color ramp (spectrum) |

The panel is **resizable in CS6** — the CS6 JDI list explicitly adds a "Resizable Color panel". (The generic CS6 "Panels" help page still carries the older sentence that "the Color panel … cannot be resized by dragging"; the JDI entry is the CS6-specific statement. See Open questions for the minimum-size/reflow rules.)

### Alerts

The panel displays two warning glyphs **above the left side of the color ramp**:

- An **exclamation point inside a triangle** when the chosen color "cannot be printed using CMYK inks" (out of the current CMYK working space; clicking substitutes the closest printable value, per `CLR-001`).
- A **square** when the color "is not web-safe" (clicking snaps to the closest of the 216 web-safe colors).

Per the Help, the gamut triangle is **not available while `Web Color Sliders` are selected** (the square remains).

### Slider model

"Choose a **Sliders** option from the Color panel menu" to change the model the sliders edit. The CS6 Help names the model switch but does not enumerate the menu; the CS6/CC-era menu offers **Grayscale / RGB / HSB / CMYK / LAB / Web Color Sliders** (Julieanne Kost / teachucomp; the `Hue Cube` entry is a later CC addition). Editing CMYK sliders in a non-CMYK document is a conversion through the current CMYK working space (`ARCH-007`), and `Web Color Sliders` snap to web-safe values with tick marks (the per-model value ranges are `CLR-001`'s table).

### Spectrum (color ramp)

"Choose an option from the Color panel menu":

| Option | Ramp shows |
|---|---|
| **RGB Spectrum** | Spectrum of the RGB model |
| **CMYK Spectrum** | Spectrum of the CMYK model |
| **Grayscale Ramp** | Grayscale spectrum |
| **Current Colors** | Spectrum between the current foreground and background colors |
| **Make Ramp Web Safe** | Restricts the ramp to web-safe colors only |

To change the spectrum of the color ramp quickly, **Shift-click** in the color ramp until the desired spectrum appears. Right-click the color bar (**Control-click** macOS) for the color-bar menu; **Shift-click** the color bar cycles color choices; **Alt-click** the color bar selects the background color (per "Keys for the Color panel").

### Selecting a color

1. Click the **foreground or background color box** to make it active (outlined in black). With the background color box active in the Color panel, the **Eyedropper** tool changes the background color rather than the foreground by default.
2. Do one of:
   - **Drag the color sliders** — "By default, the slider colors change as you drag." This is the **Dynamic Color Sliders** option in `Preferences > General`; deselecting it "improve[s] performance."
   - **Enter values** next to the sliders.
   - Click the **color selection box** to choose via the **Color Picker** (`CLR-002`).
   - Move the pointer over the **color ramp**, where it becomes the **Eyedropper**, and click to sample; **Alt-click** applies the sample to the **non-active** color selection box.

Sampling from the ramp is a **panel-internal** eyedropper; the separate image Eyedropper tool and the Color Picker are owned by `TOOL-015` / `CLR-002`.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Window > Color` | Panel | `F6` | Opens/toggles the Color panel |
| Toolbox fg/bg boxes | Button | `D` default, `X` swap | Cross-panel color carriers (`CLR-001`) |
| Panel fg/bg boxes | Toggle buttons | click | Selects the active color; active box outlined black |
| Color selection box | Button | click | Opens the Adobe Color Picker |
| Panel **Sliders** | Menu | panel menu | Switches the slider model |
| Panel spectrum options | Menu | panel menu | RGB/CMYK/Grayscale spectrum, Current Colors, Make Ramp Web Safe |
| Color ramp | 2-D control | `Shift`-click | Samples/marks; shift-click cycles spectrum |
| Color bar | Context control | right-click / `Control`-click | Color-bar menu |
| Color bar | Control | `Alt`/`Option`-click | Selects background color |
| Color bar | Control | `Shift`-click | Cycles color choices |
| Gamut alert (triangle) | Button | click | Substitutes closest CMYK value |
| Web-safe alert (square) | Button | click | Snaps to closest web-safe color |
| `Edit > Preferences > General` | Pane | — | **Dynamic Color Sliders** toggle |
| `Edit > Color Settings` | Dialog | `Ctrl/Cmd+Shift+K` | Defines the CMYK working-space gamut (`ARCH-007`) |

## Parameters & ranges

The panel edits the foreground/background `ColorValue`; the per-model numeric ranges are exactly `CLR-001`'s table (H 0–360; S/B 0–100%; R/G/B 0–255; L 0–100; a/b −128…+127; C/M/Y/K 0–100%; hex `00`–`ff`). Panel-specific controls:

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Active color box | enum | Foreground | Foreground / Background | Active box outlined black |
| Sliders model | enum | RGB *(inferred)* | Grayscale / RGB / HSB / CMYK / LAB / Web Color Sliders | Panel menu; list CS6/CC-sourced, first-run default inferred. |
| Slider count | int | 3 (RGB) | 1 (Gray) – 4 (CMYK) | Follows model |
| Spectrum | enum | RGB Spectrum *(inferred)* | RGB Spectrum / CMYK Spectrum / Grayscale Ramp / Current Colors / Make Ramp Web Safe | `Make Ramp Web Safe` is a toggle-style restriction |
| Dynamic Color Sliders | bool (pref) | on | on / off | General preferences; live slider gradients |
| Gamut alert | action | — | click triangle | Closest CMYK; uses CMYK working space |
| Web-safe alert | action | — | click square | Closest web-safe color |
| Panel size | resizable | CS6 default | any | **New in CS6** |

## Algorithms & pipeline

The panel owns no color mathematics; it is a **view + controller over one authoritative `ColorValue`** (`CLR-001`). Its only panel-specific computation is presentation:

- **Slider gradient rendering** — each slider paints the component's gradient against the *other* fixed components (the classic one-axis slice through the model). Recomputed on every authority change; `Dynamic Color Sliders` off freezes the gradients until the next model/authority change.
- **Ramp construction** — the spectrum is a sampled 1-D projection of the model (RGB/CMYK/Grayscale), the fg→bg interpolation for `Current Colors`, or a lattice-snapped rendering for `Make Ramp Web Safe`. "Current Colors" interpolates the current foreground and background values in the display space *(inferred: interpolation space and steps)*.
- **Warning evaluation** — delegated to `pictura_color::gamut` / `::websafe` (`CLR-001`), evaluated on every change and rendered at the ramp's left edge.
- **Ramp sampling** — the panel-internal eyedropper maps ramp x → color and writes the **active** box, or the **non-active** box on `Alt`-click.
- **Shift-click cycle** — cycles through a fixed spectrum list, matching CS6's quick spectrum change.

Behavioral parity only; Adobe's exact ramp sampling counts and interpolation are closed.

## Rust module mapping

The panel is Qt-side; Rust supplies authoritative state and predicates.

- `pictura_color::model` — `ColorValue`, `ColorModel` (shared with `CLR-001`).
- `pictura_color::gamut` / `pictura_color::websafe` — warning predicates and nearest-color snaps.
- `pictura_color::panel` — `ColorPanelModel { active: ActiveBox, model: ColorModel, spectrum: SpectrumMode, fg: ColorValue, bg: ColorValue }`; `slider_gradients()` and `ramp_image()` produce CPU/GPU textures for the view.
- `pictura_core::command` — `SetForegroundColor` / `SetBackgroundColor` (session commands; see `CLR-001`).
- `pictura_core::prefs` — `dynamic_color_sliders`, last-used `model`, last-used `spectrum`.

Crossing types: `ColorValue`, `ColorModel`, `SpectrumMode`, `ActiveBox`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ColorPanel` | `QDockWidget` | Host; fg/bg boxes, slider stack, ramp, alerts; resizable |
| `ForegroundBackgroundChips` | `QWidget` | fg/bg boxes, active-outline, default/swap affordances |
| `ColorSliderStack` | `QStackedWidget` of `ColorSlider` | One gradient slider per model component; dynamic gradients |
| `ColorSlider` | custom `QWidget` | Gradient track + handle + numeric entry; emits component change |
| `ColorRampWidget` | custom `QWidget` | Spectrum/ramp; shift-click cycle; internal eyedropper; alt-click to non-active box |
| `ColorPanelMenu` | `QMenu` | Sliders model + spectrum options |
| `GamutAlertButton` | `QToolButton` | Triangle + closest-CMYK tooltip; click applies |
| `WebSafeAlertButton` | `QToolButton` | Square; click snaps |
| `ColorPanelController` | `QObject` | Marshals `ColorPanelModel`; routes edits through `pictura_color`; broadcasts changes to toolbox/Swatches/Info |

Widgets, not QML: a small, always-docked numeric instrument, consistent with `ARCH-003`. The panel and the picker (`CLR-002`) share `ColorReadoutGrid` / `ColorSlider` styling so the two surfaces stay visually identical.

## Data-model impact

- **Foreground/background color** are session/workspace state, not per-document pixels; PSD does not store them *(inferred)*. They are **not** document History states.
- **No new document fields.** All edits project onto the existing `ColorValue` authority.
- **Preferences:** `dynamic_color_sliders`, last-used `model`, last-used `spectrum`, panel size (`02-ui-ux/preferences.md`).
- **Cross-panel sync:** changing fg/bg must update the toolbox boxes, Swatches "current colors," Color Picker original swatch, and the Info panel readout immediately.
- **Undo:** none at the panel level; a color is committed to the image only by a subsequent tool/command.

## Edge cases

- **8/16/32-bit** — the panel edits model values; in a 32-bpc document the ordinary boxes route to the **HDR Color Picker** (`CLR-002`), and slider precision must not silently quantize to 8-bit.
- **CMYK readout in non-CMYK documents** — CMYK sliders are a working-space projection; editing them round-trips through the profile and may re-raise the gamut alert (`ARCH-007`).
- **Grayscale documents** — hue/saturation are not representable; the panel should restrict to the Grayscale ramp *(inferred)*.
- **Missing/NO CMYK profile** — the gamut predicate must degrade to "no alert," never a false triangle.
- **Both alerts at once** — the triangle and square occupy the same ramp-left position; they must both remain clickable.
- **Web-safe ramp** — `Make Ramp Web Safe` restricts selectable ramp colors, but typed/other-model edits may still be non-web-safe unless the user clicks the alert.
- **Ramp sampling at the edges** — map x=0/x=max deterministically; do not sample out of range.
- **Dynamic Color Sliders off** — gradients must not update per drag (performance), but the handle/value must still move.
- **Undo/redo** — repainting after undo must resync fg/bg from the restored state.
- **Theme/contrast** — alert glyphs must remain visible on the dark CS6 theme and under a high-contrast preference.
- **Keyboard-only** — every control (boxes, sliders, ramp, alerts, menu) must be reachable and editable without a pointer.

## Parity acceptance criteria

1. Given an RGB document, the panel shows three RGB sliders; choosing **HSB Sliders** from the panel menu replaces them with H/S/B sliders over the same color.
2. Given a color outside the current CMYK working space, the triangle appears above the ramp's left edge; clicking it substitutes the same closest-CMYK value as the picker (`CLR-002`).
3. Given a non-web color, the square appears; clicking it yields a color whose channels are all web-safe.
4. Given `Make Ramp Web Safe`, sampling anywhere on the ramp yields a web-safe color.
5. Given `Current Colors`, the ramp spans the current foreground→background colors and updates when either changes.
6. Given the background box active, the Eyedropper tool changes the background color by default.
7. Given `Alt`-click on the ramp, the sampled color is applied to the non-active box.
8. Given `Dynamic Color Sliders` off, dragging a slider does not repaint its gradient but does change the value.
9. Given a CS6 install, the panel can be resized and the layout reflows without clipping the ramp or alerts.
10. Given a foreground color set in the panel, the toolbox boxes, Swatches current-color indicator, and Info readout all reflect it immediately.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — official CS6 Help reference (downloaded and text-extracted). Established: `Window > Color` and the panel's purpose; anatomy labels A–D (foreground, background, slider, color ramp); the gamut triangle and web-safe square above the ramp's left side; the **Sliders** menu for changing the color model; spectrum options (RGB Spectrum, CMYK Spectrum, Grayscale Ramp, Current Colors, Make Ramp Web Safe) and Shift-click ramp cycling; selecting a color (fg/bg box activation with black outline, background-active Eyedropper behavior, slider drag with **Dynamic Color Sliders** preference, value entry, picker via the color selection box, ramp eyedropper and Alt-click to the non-active box); the gamut triangle being unavailable with Web Color Sliders; "Keys for the Color panel" (Alt-click color bar = background, right/Control-click = Color Bar menu, Shift-click = cycle); `F6` for Show/Hide Color panel; "Resizable Color panel" under the CS6 JDI list.
- `https://docs.merkulov.design/choose-colors-in-the-color-and-swatches-panels` — secondary mirror of the Photoshop help page, consulted to corroborate the panel anatomy, the two alerts, the Sliders/spectrum menu, and the Dynamic Color Sliders preference.
- `https://jkost.com/blog/2017/05/tips-for-working-with-color-in-photoshop-cc.html` (fetched) — Adobe's Julieanne Kost naming the Color-panel slider menu set (Grayscale, RGB, HSB, CMYK, LAB, Web Color Sliders) and the `Hue Cube` CC-only entry.
- `https://www.teachucomp.com/the-color-panel-in-photoshop` — corroborates the selectable "RGB Sliders / CMYK Sliders" models and the color-gamut display options.
- `https://www.photoshopessentials.com/basics/managing-panels-in-photoshop-cs6` (fetched) — CS6 Essentials default workspace: Color is the top group of the right-hand main column, tabbed with Swatches.

Not used in this pass:

- `helpx.adobe.com` color-panel pages (HTTP 403 from this environment); the archived CS6 PDF was used instead.

## Open questions

- **Default slider model and spectrum.** Assumed RGB and RGB Spectrum. *Resolves with:* a CS6 first-run capture.
- **`Current Colors` interpolation space and steps.** Not documented. *Resolves with:* a CS6 ramp capture and sampling test.
- **Panel resize behavior and minimum size.** "Resizable" is sourced from the CS6 JDI list, but the reflow/min-size rules are not. *Resolves with:* a CS6 capture.
- **Grayscale/Bitmap/Indexed panel restrictions.** *Resolves with:* a CS6 mode-specific capture.
- **Exact alert click semantics when both alerts are present.** *Resolves with:* a CS6 experiment.
- **Whether CMYK sliders remain editable in a non-CMYK document** (they appear to convert through the working space) — CS6-era sources imply yes but a capture is needed.
