# Other Filters

- **Spec ID:** `FILT-070`
- **Status:** `Draft`
- **Parity tier:** `Core` (High Pass, Offset, Custom, Maximum, Minimum) + `Extended-only` for **HSB/HSL** (optional plug-in, not in a default install)
- **New in CS6:** `No` — the Other submenu is unchanged from CS5. (The `Preserve: Squareness/Roundness` option on Maximum/Minimum arrives later, in CC; it is not in CS6.)
- **Depends on:** `06-filters/filters-overview.md`, `06-filters/blur-filters.md`, `06-filters/sharpen-filters.md`, `06-filters/noise-filters.md`, `06-filters/pixelate-filters.md`, `04-image-ops/bit-depth-and-conversion.md`, `04-image-ops/image-modes.md`, `04-image-ops/adjustments/threshold.md`, `01-architecture/color-management.md`, `01-architecture/document-model.md`, `05-layers/smart-filters.md`, `05-layers/blend-modes.md`

> All crate, module, widget, and type names below are **design proposals**. The
> repository contains a working implementation; this file stays the behavioural
> contract for the feature, and `docs/dev/doc-code-parity-gaps.md` records what
> shipped and what did not. Behavior and ranges are from the fetched sources
> unless marked *(inferred)*. Adobe's exact kernel rounding and its closed HSB/HSL
> conversion are marked **behavioral parity only, algorithm TBD**.

## CS6 behavior

`Filter > Other` groups filters that let you build your own filters, modify masks, offset a selection within an image, and make quick color adjustments. The CS6 Other submenu:

| Filter | Menu path | What it does (CS6 Help) |
|---|---|---|
| Custom | `Filter > Other > Custom` | A user-defined 5×5 **convolution** matrix. Each pixel is reassigned from its neighbors' weighted sum. Filters can be saved and loaded. |
| High Pass | `Filter > Other > High Pass` | Retains edge detail in the specified radius and suppresses the rest; removes low-frequency detail. Help: a radius of 0.1 px keeps only edge pixels; effect is opposite to Gaussian Blur. Useful before Threshold or a Bitmap-mode conversion, and for extracting line art. |
| Maximum | `Filter > Other > Maximum` | Morphological **spread/dilation**: spreads white areas and chokes in black (a choke applied to masks). Replaces the pixel with the highest brightness within the radius. |
| Minimum | `Filter > Other > Minimum` | Morphological **choke/erosion**: spreads black areas and shrinks white. Replaces the pixel with the lowest brightness within the radius. |
| Offset | `Filter > Other > Offset` | Moves a selection horizontally/vertically, leaving empty space, which is filled with the background color, another part of the image, or wraps. |
| HSB/HSL | `Filter > Other > HSB/HSL` | Converts between RGB and HSB/HSL *as raw channel data*, letting H/S/B or H/S/L be manipulated as if they were R/G/B channels. Part of Adobe's **optional** multi-plugin, so it is absent unless installed. |

**Custom is convolution.** Help describes it as reassigning each pixel from the values of its surrounding pixels, similar to the Add and Subtract channel calculations. Custom is saved/loaded as presets. (Sourced.)

**Maximum/Minimum modify masks.** Help frames both explicitly as mask tools. In CS6 they expose **Radius** only. *(The later CC "Preserve: Squareness/Roundness" menu is not part of CS6; confirmed by the archived Adobe reference which dates the Preserve menu to "In Photoshop CC".)*

**HSB/HSL is optional.** Adobe ships it in the **Optional Multiplugin**, which converts RGB to HSL and to HSB inside Photoshop so that hue, saturation, and luminosity can be edited as independent channels. It appears under `Filter > Other` only after installation. This means an independent-creation CS6 parity build should treat it as an optional plugin and gate its menu entry, not as a built-in filter. (Sourced — Adobe optional-plugins KB, echoed by tutorial sources.)

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Filter > Other > Custom` | Menu / modal dialog | — | 5×5 grid, Scale, Offset, Save/Load |
| `Filter > Other > High Pass` | Menu / modal dialog | — | Radius only |
| `Filter > Other > Maximum` | Menu / modal dialog | — | Radius only (CS6) |
| `Filter > Other > Minimum` | Menu / modal dialog | — | Radius only (CS6) |
| `Filter > Other > Offset` | Menu / modal dialog | — | Horizontal/Vertical + Undefined Areas |
| `Filter > Other > HSB/HSL` | Menu / modal dialog | — | Only when the optional plug-in is installed |
| Smart Filter row | Layers panel | — | High Pass/Custom/Maximum/Minimum/Offset are Smart-Filter capable; see bit-depth list |
| `Edit > Fade` | Menu | `Ctrl+Shift+F`/`Cmd+Shift+F` | Common after High Pass or Custom |

## Parameters & ranges

Ranges marked **[AS]** are from the Photoshop CS6 AppleScript Scripting Reference; **[Help]** from the CS6 Help; *(inferred)* where neither states a limit. The Custom matrix value limit is stated by Help for the center cell (−999…+999) and the whole-matrix ordering is sourced from the AppleScript `characteristic` array.

| Filter | Control | Type | Default | Range / options | Source |
|---|---|---|---|---|---|
| Custom | Matrix | 5×5 ints (−999…+999 each) | identity *(inferred)* | 25 values, left→right, top→bottom; center = the evaluated pixel | [AS][Help] |
| Custom | Scale | int | 1 *(inferred)* | divisor applied to the weighted sum; range *(inferred)* 1–9999 | [Help] (control), range *(inferred)* |
| Custom | Offset | int | 0 *(inferred)* | value added after scaling; range *(inferred)* −9999…+9999 | [Help] (control), range *(inferred)* |
| Custom | Save / Load | buttons | — | Save/load custom filter presets | [Help] |
| High Pass | Radius | real px | 10 *(inferred)* | 0.1–250.0 | [AS] |
| Maximum | Radius | real px | 1 *(inferred)* | 1–100 | [AS] |
| Minimum | Radius | real px | 1 *(inferred)* | 1–100 | [AS] |
| Offset | Horizontal | real (unit value) | 0 | min/max depend on layer size | [AS] |
| Offset | Vertical | real (unit value) | 0 | min/max depend on layer size | [AS] |
| Offset | Undefined Areas | enum | Set To Background *(inferred)* | Wrap Around / Repeat Edge Pixels / Set To Background (AS: "set to layer fill") | [Help][AS] |
| HSB/HSL | Input Mode | enum | RGB *(inferred)* | RGB / HSB / HSL | Tutorial sources |
| HSB/HSL | Row Order | enum | HSB *(inferred)* | HSB / HSL | Tutorial sources |

**Bit-depth gate (Help "Filter basics").** The 16-bit list includes **Custom, High Pass, Maximum, Minimum, and Offset**. The 32-bit list includes **High Pass, Maximum, Minimum, and Offset** (not Custom). HSB/HSL is not listed and is 8-bit. (Sourced.)

## Algorithms & pipeline

### Custom convolution math

Custom is a discrete 5×5 convolution. Let the matrix be `K` with entries `k[i][j]`, `i,j ∈ {−2,−1,0,1,2}`, where `k[0][0]` is the center (the pixel being evaluated) and the traversal is left→right, top→bottom. For each pixel at `(x,y)` and each channel independently:

```text
sum(x, y) = Σ_{i=-2..2} Σ_{j=-2..2}  k[i][j] · src(x + i, y + j)
out(x, y) = clamp( sum(x, y) / Scale + Offset , channel_min , channel_max )
```

Notes:
- `Scale` divides the weighted sum (a normalizing divisor so brightness is preserved; e.g. a 3×3 all-ones kernel uses `Scale = 9`).
- `Offset` is added after division (mid-gray `128` shifts a high-pass-style kernel back into range, matching High Pass's behavior).
- Clamp to the channel range (`0–255` at 8-bit, `0–65535` at 16-bit, unbounded float at 32-bit if Custom were supported there).
- Out-of-bounds neighbors follow the selection/layer edge policy; Photoshop's exact edge policy is closed, so use clamp-to-edge for now *(inferred; verify)*.
- The kernel is applied to RGB and alpha? Photoshop applies Custom per channel; alpha handling is unconfirmed.

Worked kernels:

```text
Identity (pass-through):            Scale = 1,  Offset = 0
  [0 0 0 0 0]
  [0 0 0 0 0]
  [0 0 1 0 0]
  [0 0 0 0 0]
  [0 0 0 0 0]

3×3 mean (box blur), embedded:      Scale = 9,  Offset = 0
  [0 0 0 0 0]
  [0 1 1 1 0]
  [0 1 1 1 0]
  [0 1 1 1 0]
  [0 0 0 0 0]

Sharpen:                            Scale = 1,  Offset = 0
  [ 0 0  0 0 0]
  [ 0 0 -1 0 0]
  [ 0 -1 5 -1 0]
  [ 0 0 -1 0 0]
  [ 0 0  0 0 0]
```

### Other algorithms

| Filter | Algorithm family | Notes |
|---|---|---|
| High Pass | `src − Gaussian(src, r) + 128` (unsharp-style band split) | Retains the high-frequency band around mid-gray; large radius keeps broad edges, 0.1 keeps only edge pixels. Adobe's blur kernel is closed; behavioral parity only. *(inferred formula, sourced behavior)* |
| Maximum | Morphological dilation (max filter) | Per-channel max over the radius; brightens/spreads whites, chokes blacks. Mask-oriented. *[Help]* |
| Minimum | Morphological erosion (min filter) | Per-channel min over the radius; spreads blacks, shrinks whites. *[Help]* |
| Offset | Translation with edge handling | Pure integer/real translation; Wrap Around = modulo, Repeat Edge Pixels = clamp, Set To Background = fill exposed area with the background color. *[Help]* |
| HSB/HSL | Color-space re-encoding of channel data | Input RGB → target HSB/HSL: H→R slot, S→G slot, B or L→B slot (and the reverse). This is a lossy 8-bit re-encoding used to isolate hue/saturation/luminance as channels; it is not a normal-mode conversion. *(inferred internals; sourced intent)* |

**Pipeline.** Custom, High Pass, Maximum, and Minimum are neighborhood filters whose halo equals the matrix radius (2 for Custom, the radius for High Pass/Max/Min); Offset is a pure remap. A shared `NeighborhoodFilter` (already proposed in `FILT-050`) plus a shared resampler is enough.

## Rust module mapping

- `pictura_filter::other` — per-filter submodules implementing `Filter`.
- `pictura_filter::other::custom` — `CustomKernel { k: [[i32; 5]; 5], scale: i32, offset: i32 }`; `convolve()` shared with `FILT-050` Emboss/Find Edges.
- `pictura_filter::other::high_pass` — separable Gaussian via `pictura_filter::blur`, then `src − blur + mid`.
- `pictura_filter::other::maximum` / `minimum` — separable dilation/erosion (max/min over a square or round radius; CS6 squareness-only, no Preserve).
- `pictura_filter::other::offset` — integer translation + `EdgeMode::{WrapAround, RepeatEdgePixels, SetToBackground}` (needs fg/bg color).
- `pictura_filter::other::hsb_hsl` — optional plugin; `InputMode`, `RowOrder`; runs only when the optional-plugin flag is enabled.
- `pictura_filter::registry` — filter id + CS6 four-char event ids (`'Cstm'`, `'HghP'`, `'Mxm '`, `'Mnm '`, `'Ofst'`, `'HsbP'`) → implementation + `supported(mode, depth)`.

Crossing types: `Tile`, `Rect`, `CustomKernel`, `EdgeMode`, `ColorMode`, `BitDepth`, `Rgb`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `OtherOptionsDialog` | `QDialog` | Per-filter forms in a `QStackedWidget` |
| `CustomMatrixWidget` | `QWidget` | 5×5 grid of `QSpinBox`/`QLineEdit` (−999…999), Scale/Offset spins, Save/Load buttons |
| `HighPassPanel` | `QWidget` | Radius spin (0.1–250.0) |
| `MorphologyPanel` | `QWidget` | Radius spin (1–100); Preserve absent (CS6) |
| `OffsetPanel` | `QWidget` | H/V fields honoring document units + Undefined Areas combo |
| `HsbHslPanel` | `QWidget` | Input Mode + Row Order combos; entry only present when the optional plugin is enabled |
| `FilterMenuBuilder` | helper | Hides HSB/HSL unless the optional plugin is installed; greys Custom on 32-bit |
| `FilterPreviewPane` | `QGraphicsView` | Shared preview |

Widgets over QML for the numeric dialogs, consistent with `ARCH-003`. The Custom matrix benefits from a real grid widget rather than a table model.

## Data-model impact

- **Custom presets.** Custom filters can be saved and loaded as named presets, so the preset store (`10-workflow-io/presets-manager.md`) needs a `CustomKernel` record separate from normal filter parameters.
- **Undo.** All Other filters recompute from parameters except Offset, which also depends on the current background color; the undo record stores the color used. No pixel snapshots needed.
- **HSB/HSL is a plugin.** Its availability is a capability flag, not a document property; a document that used it stores the applied result on the layer. If it is a Smart Filter, its parameters are a small enum record.
- **Smart Filters.** High Pass, Custom, Maximum, Minimum, and Offset sit on a Smart Object stack as ordinary parameterized filters (`LAY-021`).
- **PSD.** No Other-specific additional-layer keys documented beyond the Smart Filter record.

## Edge cases

- **Custom at 32-bit.** Custom is not in the 32-bit support list; grey it on 32-bit documents even though High Pass/Maximum/Minimum/Offset run.
- **Custom overflow.** The weighted sum can exceed `i32` for extreme matrices on large-radius data; use `i64` accumulation, then divide, add offset, and clamp.
- **Custom alpha/multichannel.** Whether alpha is convolved is unconfirmed; default to color channels only and document.
- **High Pass radius 0.1.** Help says only edge pixels survive; the Gaussian must support sub-pixel sigma (0.1 px).
- **High Pass + Threshold/Bitmap.** The documented use case; verify the output is mid-gray-centered so Threshold behaves predictably.
- **Maximum/Minimum radius.** Max 100 px per the scriptable range; larger requests must clamp. CS6 has **no** Preserve (squareness/roundness) control.
- **Offset units.** H/V are unit values, not raw pixels; convert using document resolution. The exposed fill area must use the current background color at apply time.
- **Offset wrap on selection.** Wrap should be relative to the selection bounds, not the full document (verify against CS6).
- **HSB/HSL absent.** With no optional plugin installed the menu entry must not appear; a document using it should still open.
- **HSB/HSL rounding.** The 8-bit H/S/L re-encode is lossy and not round-trip exact.
- **CMYK/Lab.** These are channel-assignment filters; CMYK Max/Min operate per ink channel (Help's mask framing implies gray use). Gate per real CS6 behavior.
- **Huge/PSB documents.** Morphology at radius 100 px is separable and cheap; Custom is fixed 5×5. No area blow-up.
- **Undo/redo.** Parameter-only except Offset's background color.

## Parity acceptance criteria

- Given the Custom identity kernel `[k00=1]`, `Scale=1`, `Offset=0`, the output equals the input exactly.
- Given the 3×3 mean kernel with `Scale=9, Offset=0`, the output is the box blur of the input within rounding tolerance.
- Given the sharpen kernel above, edge contrast increases while flat regions are unchanged.
- Given a Custom kernel and `Scale`, doubling `Scale` halves the kernel's deviation from 0 before `Offset`.
- Given `Offset` with `Offset=0`, the output equals the input.
- Given a Custom filter, Save then Load restores the same 25 values, Scale, and Offset.
- Given High Pass radius 0.1, only edge pixels remain non-mid-gray; at a large radius, broad tonal contours survive.
- Given High Pass followed by `Image > Adjustments > Threshold`, the result is a clean line/mask extraction (the documented use).
- Given Maximum at radius `r`, every output pixel equals the maximum of its neighbors within `r`; Minimum equals the minimum. Maximum on a white-on-black mask grows the white region; Minimum shrinks it.
- Given Maximum followed by Minimum at the same radius (closing), small black holes in a white mask are filled.
- Given Offset with Wrap Around by `(w, 0)`, the image is shifted and the wrapped column re-enters on the other side.
- Given Offset with Set To Background at the edge of an image, the exposed area is the current background color.
- Given Offset with Repeat Edge Pixels, exposed columns copy the nearest edge column.
- Given a document without the optional plugin, `Filter > Other > HSB/HSL` is absent.
- Given HSB/HSL Input Mode RGB / Row Order HSB, the green channel stores saturation (the documented saturation-mask workflow).
- Given a 32-bit document, High Pass/Maximum/Minimum/Offset run and Custom is unavailable; on 16-bit all five (including Custom) run.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — Photoshop CS6 Help. Established: the Other submenu descriptions (Custom convolution and save/load; High Pass radius 0.1 keeps edge pixels and its relation to Gaussian Blur and Threshold/Bitmap; Maximum = spread/dilation and Minimum = choke/erosion for masks; Offset and its fill options); "Create a Custom filter" (grid, center value −999…+999, Scale divides the sum, Offset is added, Save/Load); "Defining undistorted areas" (Wrap Around / Repeat Edge Pixels / Set To Background); the 16/32-bit support lists (Custom/High Pass/Maximum/Minimum/Offset at 16-bit; High Pass/Maximum/Minimum/Offset at 32-bit); and that Extract and Pattern Maker are optional plug-ins.
- `https://applescriptlibrary.files.wordpress.com/2013/11/photoshop-cs6-applescript-reference.pdf` — Photoshop CS6 AppleScript Scripting Reference. Established the exact ranges: Custom `characteristic` = 25 values left→right/top→bottom plus `scaling` and `offset`; High Pass radius 0.1–250.0 px; Maximum radius 1–100 px; Minimum radius 1–100 px; Offset horizontal/vertical unit values with size-dependent bounds and undefined areas repeat edge pixels / set to layer fill / wrap around; and the four-char event IDs `'Cstm'`, `'HghP'`, `'Mxm '`, `'Mnm '`, `'Ofst'`, `'HsbP'`.
- `https://web.archive.org/web/2014id_/https://helpx.adobe.com/photoshop/using/filter-effects-reference.html` — archived Adobe "Filter effects reference". Established that the **Preserve (squareness/roundness)** control on Maximum/Minimum is a later **Photoshop CC** addition, not CS6.
- `https://mattlauder.com.au/make-vibrancy-saturation-masks` — tutorial. Established the HSB/HSL dialog controls in practice (Input Mode = RGB, Row Order = HSB) and the saturation-mask workflow where the green channel becomes saturation; also that HSB/HSL requires Adobe's Optional Multiplugin.
- `https://photoshoptrainingchannel.com/hsb-hsl-filter-saturation-mask` — tutorial. Corroborated that HSB/HSL ships with the optional plug-in ("Filter for Photoshop CC 2019 and earlier (Including CS6)") and is used to build saturation masks.
- `https://www.computerhope.com/jargon/p/photoshop-glass.htm` — used only to confirm that filters such as these are Filter-Gallery/8-bit-capable where Help is silent (not central to this spec).

Not used in this pass:

- `https://helpx.adobe.com/photoshop/kb/optional-file-format-plugins.html` — live page returned HTTP 403; the optional-plugin description was taken from the fetched tutorial sources and the Help PDF's optional-plug-in note instead.

## Open questions

- **Custom matrix defaults.** The initial dialog values (identity vs all-zero) and the exact `Scale`/`Offset` ranges are inferred from Help prose. Resolve from a CS6 UI capture.
- **Custom convolution edge policy and alpha.** Photoshop's out-of-bounds policy and whether the alpha channel is convolved are undocumented. Resolve by fitting.
- **High Pass formula.** The `src − Gaussian + 128` form is inferred; Adobe's exact blur kernel, sigma-to-radius mapping, and sign/offset are closed. Resolve by fitting.
- **Maximum/Minimum kernel shape.** CS6 has no Preserve control, but the default footprint (square vs circular) and per-channel behavior in CMYK are unconfirmed. Resolve by fitting.
- **Offset wrap reference frame.** Whether Wrap Around is relative to the selection or the document is unconfirmed. Resolve with a CS6 test.
- **HSB/HSL exact encoding.** The precise H/S/B (and H/S/L) scaling to 8-bit channels and the rounding are closed; the plug-in is optional, so the independent choice may be to recreate it as an optional plugin with behavioral parity only. Resolve with the Optional Multiplugin binary's documented behavior.
- **HSB/HSL presets/serialization.** Whether the input mode/row order are stored per Smart Filter instance is unconfirmed.
- **Custom in Smart Filters.** The 32-bit gating suggests Custom is 8/16-bit; whether it can be a Smart Filter at all is unconfirmed.
- **"Set To Background" vs scriptable "set to layer fill".** Help says background color; the AppleScript reference says "set to layer fill". Reconcile against a CS6 test.
