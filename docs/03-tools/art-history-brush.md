# Art History Brush

- **Spec ID:** `TOOL-034`
- **Status:** `Draft`
- **Parity tier:** `Core` — available in CS6 Standard. The CS6 Help notes that most stylization is *not* needed for extended workflows; this remains a standard paint tool.
- **New in CS6:** `No` — the Art History Brush predates CS6 (introduced in Photoshop 6) and is unchanged.
- **Depends on:** `ARCH-009` undo-history, `ARCH-008` document-model, `ARCH-006` gpu-rendering-pipeline, `03-tools/history-brush.md`, `07-color-painting/brush-engine.md`, `07-color-painting/brush-presets.md`

> Module and widget names are **design proposals**. The repository contains a
> working implementation; this file stays the behavioural contract for the
> feature, and `docs/dev/doc-code-parity-gaps.md` records what shipped and what
> did not. The exact Adobe style kernels are closed; parity is **behavioral
> parity only, algorithm TBD**. Behavioral facts are from the fetched CS6 Help
> PDF and the cited secondary CS6/community sources.

## CS6 behavior

The Art History Brush (`Y`, same group as the History Brush) paints with
**stylized strokes** using the source data from a specified history state or
snapshot. Like the History Brush it uses a state/snapshot as source, but where
the History Brush *re-creates* that source data, the Art History Brush uses it
together with the style/size/tolerance options to produce different colors and
artistic effects.

- **Source** is chosen the same way: click the left column of a state or snapshot
  in the History panel; a brush icon appears next to the source. The source is
  the color each stylized stroke samples.
- The Help suggests experimenting by applying filters or filling an image with a
  solid color **before** painting, and enlarging the image (e.g. ×4) to soften
  detail.
- Options bar:
  - **Brush** from the Brush Presets picker, plus brush options.
  - **Mode** — a blending mode.
  - **Style** — controls the shape of the paint stroke. Documented examples
    include `Tight Short` (the common default), `Tight Medium`, `Tight Long`,
    `Loose Medium`, `Loose Long`, `Dab`, and the curl variants `Tight Curl` and
    `Loose Curl` (with Short/Medium/Long forms). The full CS6 enumeration is not
    stated by the fetched primary source; see Open questions.
  - **Area** — the area covered by the paint strokes; larger values cover more
    area and produce more numerous strokes.
  - **Tolerance** — limits where strokes may be applied. **Low tolerance** lets
    the user paint unlimited strokes anywhere; **high tolerance** restricts
    strokes to areas that differ considerably from the color in the source state
    or snapshot.
- The brush shape matters too; soft-edged and bristle brushes are both used
  (Evening paints a soft diffusion edge with a soft brush; `Tight Long` and
  `Loose Medium` were his most-used styles).
- The Art History Brush is **not** available at 16 bpc (it is explicitly excluded)
  and is **not** in the CS6 32-bpc HDR tool list.

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| Tools panel, History group | Tool | `Y` | Shares the `Y` slot with the History Brush |
| Options bar | Bar | — | Brush preset, Mode, Style, Area, Tolerance |
| History panel | Dock | `Window > History` | Left column selects the source state/snapshot (brush icon marks it) |
| Brush Presets panel | Dock | `F5` | Tip shape and dynamics |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Source state/snapshot | ref | current/no source | any retained state or snapshot | Selected in the History panel |
| Brush preset | preset ref | last used | any tip | Bristle and soft tips both used |
| Mode | enum | Normal | CS6 blend list | Standard paint blending |
| Opacity | int % | 100 | 0–100 | Per-stroke (paint-tool default) |
| Style | enum | Tight Short | Tight/Loose × Short/Medium/Long; Dab; Tight Curl / Loose Curl (×Short/Medium/Long) | Exact CS6 list unverified |
| Area | int px | 50 | approx. 0–500 *(upper bound unverified)* | Larger = more/wider strokes |
| Tolerance | int % | 100 | 0–100 *(unverified)* | Low = paint anywhere; high = only high-contrast regions |

Defaults for Mode = Normal, Opacity = 100%, Area = 50 px, Tolerance = 100% and
default Style = Tight Short come from a Photoshop 6-era primary tutorial and from
community documentation; the CS6 Help PDF does not restate them. Treat the exact
CS6 defaults as *(unverified)*.

## Algorithms & pipeline

The Art History Brush is a paint tool whose source is a history state/snapshot,
with stroke *shape* synthesized per style rather than a direct same-coordinate
copy.

Proposed behavioral model (exact Adobe kernels closed):

1. Resolve the source state/snapshot to a readable tile view (as in the History
   Brush, `03-tools/history-brush.md`).
2. For each dab:
   - Sample the source color around the dab center over the **Area** neighborhood
     (an average/representative color, size/coverage grows with Area).
   - **Tolerance gate:** compare the source color at the dab origin with the
     current destination color. If the difference is below the tolerance
     threshold, skip the dab. Low tolerance (0) paints everywhere; high tolerance
     restricts painting to regions whose color differs substantially from the
     source. (This matches the Help wording: high tolerance "limits paint strokes
     to areas that differ considerably from the color in the source state.")
   - **Style** determines the stroke geometry: a family parameterized by stroke
     **length** and **curl** (tight = little curvature/jitter and short reach;
     loose/long = longer, more meandering) plus a per-dab **angle jitter**. `Dab`
     is the degenerate zero-length case (a single paint point; strokes at a
     similar angle regardless of drag direction).
   - Paint the dab with the brush tip at the style-derived position/angle, in the
     chosen mode and opacity.
3. The stroke is randomized per pass, so repeated painting over the same area
   yields different results even with identical settings. This is intentional
   (the Help encourages experimentation and "each time you brush over an area,
   the results are slightly different").

Parameterization proposal for the style table (all values are design choices,
not Adobe values): `Style { length: f32, curl: f32, angle_jitter: f32, spacing:
f32, dab_scale: f32 }`. The style + brush tip + Area + Tolerance + a seeded RNG
fully determine a dab; store the seed in the command so redo is stable.

## Rust module mapping

Proposals:

- `pictura-retouch::art_history::ArtHistoryBrush` — `ArtHistoryConfig { source:
  StateRef, mode, opacity, style: StrokeStyle, area: u32, tolerance: u8, brush,
  seed }`; the stylized dab loop.
- `pictura-retouch::art_history::StrokeStyle` — `{ length, curl, angle_jitter,
  spacing, dab_scale }` plus the named presets:
  `TightShort`, `TightMedium`, `TightLong`, `LooseMedium`, `LooseLong`, `Dab`,
  `TightCurl*`, `LooseCurl*`.
- `pictura-retouch::art_history::tolerance_gate` — source-vs-destination color
  difference test (per document color model).
- `pictura-retouch::sample::AreaSampler` — representative color over the Area
  neighborhood (box/Gaussian) from the source tile view.
- `pictura-history::tile_store` — source version read and `pin` (shared with the
  History Brush).
- `pictura-core::command::ArtHistoryStroke` — emitted on stroke release.

Crossing types: `StateRef`, `StrokeStyle`, `Seed(u64)`, borrowed tile slices. The
tool reads through `pictura-history` and writes only via a `Command`.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `ArtHistoryBrushOptions` | `QWidget` (options bar) | Brush preset, Mode, Style combo, Area spin, Tolerance spin |
| `StylePresetCombo` | `QComboBox` | Lists the stroke styles with icon previews |
| `HistoryModel` / `HistoryPanel` | `QAbstractListModel` / dock | Source selection (shared with History Brush) |
| `ArtHistoryPreview` | `QQuickItem` | Optional on-canvas preview of the current dab/style |

Widgets for the options bar; the canvas preview (if any) is a `QQuickItem`.
Because the tool is randomized, a preview is illustrative only. All writes go
through the command API; the RNG seed is captured at stroke start.

## Data-model impact

- **No PSD fields.** Source state/snapshot, style, area, tolerance, seed, and RNG
  state are session-only. History and snapshots are not serialized (`ARCH-009`),
  so the tool has no persistent state.
- **Undo:** one history state per completed stroke; the command stores pre-edit
  tiles for bit-exact undo and the RNG seed for stable redo.
- **Source pinning:** as with the History Brush, the active source must be
  retained against the History States limit, or the tool must refuse cleanly.
- **Brush presets** (including bristle tips) come from
  `07-color-painting/brush-presets.md`; the style is a tool option, not a brush
  preset field.

## Edge cases

- **8-bpc only (practically).** The Art History Brush is explicitly excluded from
  16-bpc images and from the 32-bpc HDR tool list; it must refuse on those depths
  rather than convert silently.
- **Pruned source** — same as the History Brush: pin or refuse, never read freed
  tiles.
- **Low vs. high tolerance semantics** — 0 must paint anywhere; a high tolerance
  must be able to leave regions unpainted. Verify the direction of the comparison
  (difference must *exceed* the threshold to paint).
- **Area vs. brush size** — Area is independent of brush size; a large Area with a
  small brush overlays many small dabs.
- **Randomness / non-determinism** — repeated strokes differ; only redo must be
  reproducible (stored seed).
- **Huge PSB documents** — the Area neighborhood and many dabs must stay
  tile-local and bounded; never sample a whole-canvas copy.
- **CMYK / Lab / Grayscale / Indexed** — tolerance is a color difference in the
  document model; define it per model and refuse where undefined (e.g. Indexed).
- **GPU unavailable** — CPU fallback; the tool remains functional.
- **Undo mid-stroke** — strokes atomic on release.
- **Source equals current state** — allowed; the result is a stylized repaint of
  the current pixels.

## Parity acceptance criteria

- Given a source state selected in the History panel, painting produces stylized
  strokes whose colors are drawn from that source, not the current layer.
- Given Tolerance = 0, painting applies strokes anywhere the brush travels;
  given a high Tolerance over a uniform region, strokes are suppressed where the
  source and destination colors are similar, and appear only where they differ
  considerably.
- Given a larger Area with the same brush, more/wider strokes cover a larger
  region; given a smaller Area, coverage is sparser.
- Given each Style (Tight Short/Medium/Long, Loose Medium/Long, Dab, Tight/Loose
  Curl), the stroke geometry differs consistently: `Tight` styles are straighter
  and shorter-reach, `Loose`/`Long` styles meander further, and `Dab` makes
  single-point marks.
- Given the default Style, painting matches a `Tight Short` character rather than
  a `Loose` one (default is Tight Short).
- Given two identical strokes at the same settings, the second differs from the
  first (randomized), while a redo of a recorded stroke reproduces it exactly.
- Given a completed stroke, undo restores every touched pixel bit-exactly and
  exactly one history state is added.
- Given a 16-bpc or 32-bpc document, the tool reports "unavailable" rather than
  converting.
- Given a source state pruned or exceeding the History States limit, the tool
  either keeps it pinned or refuses with a clear error.

## Sources

Fetched for this document:

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` —
  Adobe Photoshop CS6 Help reference (downloaded and text-extracted). Established:
  the Art History Brush paints stylized strokes from a specified history state or
  snapshot; contrast with the History Brush (re-creates source vs. stylized);
  source chosen in the History panel; options bar has Brush, Mode, Style, Area,
  Tolerance; Area larger = larger/more numerous strokes; low Tolerance paints
  anywhere, high Tolerance restricts to areas differing considerably from the
  source color; advice to filter/fill and enlarge before painting; `Y` shortcut;
  16-bpc exclusion (all tools except Art History Brush) and absence from the
  32-bpc HDR tool list. Primary source.
- `http://www.photoshopforphotographers.com/3101-1901/Help_guide/tp/Art_history_brush.html`
  — Martin Evening, *Adobe Photoshop CS6 for Photographers* support page.
  Established: "the art history brush strokes can be applied using abstract
  characteristics that smudge the pixels when sampling from the selected history
  state"; Tolerance = how much paint is applied based on closeness of stroke
  color to destination; Area larger = more strokes/coverage; `Tight Long` and
  `Loose Medium` used in his examples; soft-edged brush for a diffusion effect.
  Secondary, CS6.
- `https://www.bapugraphics.com/blog/adobe-photoshop-art-history-brush-tool` —
  community documentation. Established: Style menu examples `Tight short`,
  `Loose Medium`, `Dab`, `Loose Curl`; Area controls coverage independent of
  brush size; high Tolerance limits strokes to areas very different from the
  source. Secondary/unverified.
- `https://creativepro.com/photoshop-how-to-advanced-techniques-with-the-art-history-brush`
  — "Photoshop How-to: Advanced Techniques with the Art History Brush"
  (Photoshop 7-era). Established: defaults Normal mode, 100% opacity, 50-pixel
  Area, 100% tolerance; `Style = Dab` produces strokes at a similar angle
  regardless of drag direction; Tolerance 0% removes resistance to paint; each
  pass over an area differs; `Tight Medium` with a soft wet brush. Secondary
  (older version), used only for defaults/behavior.

Not parsed in this pass: `https://helpx.adobe.com/photoshop/using/painting-stylized-strokes-art-history.html`
(helpx.adobe.com returns HTTP 403); `https://www.sitepoint.com/getting-painterly-with-the-art-history-brush-in-photoshop`
(HTTP 403).

## Open questions

- **Full CS6 Style enumeration and default.** The CS6 Help PDF lists no styles
  explicitly; secondary sources give overlapping subsets (`Tight Short/Medium/
  Long`, `Loose Medium/Long`, `Dab`, `Tight Curl`, `Loose Curl` and curl
  Short/Medium/Long forms). The complete ordered list and the CS6 default
  (`Tight Short` per community sources) need confirmation from a CS6 build.
- **Exact defaults (Area, Tolerance, Mode, Opacity) in CS6.** Taken from a
  Photoshop 6/7-era source; confirm for CS6.
- **Style geometry model.** Length/curl/angle-jitter parameters and their numeric
  values are a design proposal, not Adobe data. Resolve by measuring
  reference strokes or accept behavioral parity.
- **Tolerance metric.** Whether the gate uses a Euclidean RGB distance, a
  luminance difference, or a Lab delta is undocumented. Resolve by experiment.
- **Area sampling metric.** Whether Area averages a neighborhood or merely gates
  dab density is described indirectly ("area covered by the paint strokes ... more
  numerous the strokes"). Resolve with controlled strokes.
- **Seed/redo behavior.** Whether CS6 reproduces a stylized stroke on redo
  exactly is unverified; our design stores an RNG seed to guarantee it.
