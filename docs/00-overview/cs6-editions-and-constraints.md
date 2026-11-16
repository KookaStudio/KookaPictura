# CS6 Editions and Constraints

- **Spec ID:** `OVR-002`
- **Status:** `Draft`
- **Parity tier:** `Core` (Standard feature set) with `Extended-only` entries marked inline.
- **New in CS6:** `N/A` — this document is the release/edition baseline for every other spec.
- **Depends on:** `OVR-001`, `OVR-003`, `OVR-004`, `01-architecture/`.

This document fixes the parity target: **Adobe Photoshop CS6, version 13
(2012)**, in both Standard and Extended editions, and records the documented
feature differences between the editions and between CS6 and CS5. Anything
marked "inferred" or "unverified" is flagged and repeated under
`## Open questions`.

## CS6 behavior

### Release timeline (version 13)

| Event | Date | Source |
|---|---|---|
| Public beta released | 22 March 2012 | Adobe press release (Adorama reprint) |
| Announced (CS6 + Creative Cloud launch event) | 23 April 2012 | Adobe press release; The Verge |
| General availability (pre-order delivery estimate) | 7 May 2012 | The Verge; ProDesignTools |

CS6 is the last Photoshop release offered with a **perpetual license** (Adobe
stopped selling perpetual CS6 titles on 9 January 2017), and the last to support
Windows XP SP3 and Mac OS X Snow Leopard. The bundled RAW engine is **Adobe
Camera Raw 7 (ACR 7)**. ([Fandom CS6](#sources); [Wikipedia](#sources).)

### Editions: Standard vs. Extended

Both editions ship the same application binary family and the same core editing
feature set. The Extended edition adds:

| Capability | Standard (CS6) | Extended (CS6) |
|---|---|---|
| 2D image editing, layers, filters, type, color | Yes | Yes |
| Vector tools, 3D-type improvements (vector strokes/dashes/gradients) | Yes | Yes |
| Video / motion editing and the Timeline | **Yes** (moved to *all* editions in CS6) | Yes |
| 3D: extrusion, painting on 3D, lighting, shadows, reflections, 3D animation | No | Yes |
| Mercury Graphics Engine for 3D | No | Yes |
| Measurement and counting tools, measurement log / data export | No | Yes |
| DICOM import and analysis | No | Yes |
| Quantitative image analysis | No | Yes |
| Estimated street price (2012) | US$699 | US$999 |

The key nuance: **video/time-based editing was Extended-only in CS5 and earlier,
but became a feature of every CS6 edition.** 3D, measurement/counting, and DICOM
remained Extended-only in CS6. The standalone trial and the Student & Teacher
edition were always the Extended build. ([ProDesignTools](#sources); [Adobe press
release](#sources).)

3D features in CS6 Extended were **not supported on Windows XP**.
([ProDesignTools](#sources).)

### Feature set, and what was new in CS6 vs. CS5

The lists below separate features that are **new in CS6** from capabilities that
already existed in CS5 and changed. The release-specific feature names and menu
paths are the contract that the feature specs expand.

#### New in CS6

| Feature | What it is | Confidence |
|---|---|---|
| Content-Aware Move | Move a selected region; the vacated area is synthesized. New mode under the Spot Healing Brush / Content-Aware tool group. | Sourced |
| Content-Aware Patch | Like Content-Aware Fill but the user chooses (paints) the source sample area. | Sourced |
| Perspective Crop | Crop tool mode that corrects perspective by dragging corner handles, producing a rectangular result. | Sourced |
| Blur Gallery (`Filter > Blur > Field Blur / Iris Blur / Tilt-Shift`) | On-image, in-context blur controls with a focal-point model; GPU/OpenCL-accelerated. | Sourced |
| Adaptive Wide Angle (`Filter > Adaptive Wide Angle`) | Corrects wide-angle/fisheye distortion by placing on-image constraints. | Sourced |
| Oil Paint (`Filter > Oil Paint`) | Painterly stylization filter; bundled with CS6 (previously a separate/Pixel Bender extra). | Sourced |
| Redesigned Crop tool | Non-modal crop with straighten, rule-of-thirds, and option-bar size/resolution controls. | Sourced |
| Dark UI and new icon set | Default dark application frame with an optional lighter appearance. | Sourced |
| Auto-recovery / background save | Periodic auto-save and the ability to keep working while saving large documents. | Sourced |
| ~~Artboards (CS6)~~ | **Correction (2026-09): not a CS6 feature.** Artboards were added in Photoshop CC 2015; CS6 documents have no artboard container. | Verified / corrected |
| Layer search / filtering | Filter the Layers panel by name, kind, effect, mode, attribute, color. | Sourced |
| Vector shape and type improvements | Type/Paragraph/Character styles, vector strokes (dotted/dashed), gradients on vector objects, snap-to-pixel. | Sourced |
| Mercury Graphics Engine (MGE) | Unified GPU acceleration (OpenGL + OpenCL) across many interactive features. | Sourced |
| Color Lookup / 3DLUT adjustment | Apply `.cube`/3D LUTs as an adjustment. | Sourced |
| Properties panel | Context panel for adjustment-layer and other selected-object properties. | Sourced |
| Color Range: skin tone and face detection | New Color Range presets. | Sourced |
| ~~Camera Raw as a filter (`Filter > Camera Raw Filter`)~~ | **Correction (2026-09): not a CS6 feature.** The Camera Raw *filter* was introduced in Photoshop CC (2013, ACR 8). CS6 uses ACR 7 only for opening raw files / opening as Smart Object. | Verified / corrected |
| Improved Printing UI | Reworked print dialog and print settings. | Sourced |
| Mini Bridge | Bridge access as a panel inside Photoshop. | Unverified — see open questions |

#### Carried from CS5 but changed in CS6

| Feature | CS6 change |
|---|---|
| Video editing | Moved from Extended-only to all editions; expanded tooling. |
| Crop | Replaced by the redesigned crop (destructive crop is now deferred/default in CS6's crop). |
| Type | Styles, better vector text handling. |
| 3D (Extended) | New engine, on-canvas controls, "drag-able" shadows/reflections, MGE for 3D. |
| Camera Raw | ACR 7 with new tone-mapping/processing controls. |
| Content-Aware Fill | Extended by the new Move and Patch modes. |

Edition- and version-specific behavior is the **contract** for every feature
spec: a spec whose feature was Extended-only in CS5 but shipped to all editions
in CS6 must say so, and a spec whose feature is Extended-only in CS6 must carry
the `Extended-only` parity tier.

## UI surface

Where the edition and version differences become visible to the user. Menu paths
marked *(inferred)* were not confirmed against a primary screenshot and need
verification.

| Location | Type | Edition | Notes |
|---|---|---|---|
| `Window > 3D` | Panel | Extended | 3D panel; hidden in Standard. |
| 3D menu items (e.g. `3D > New 3D Extrusion from …`) | Menu | Extended | Present only in Extended / for 3D-capable layers. |
| `Window > Measurement Log` | Panel | Extended | Measurement/count records. |
| `Window > Timeline` | Panel | Standard + Extended | Video/animation; present in all CS6 editions. |
| Count tool | Tool/results | Extended | Count markers and the count/measurement log. |
| `Analysis > …` | Menu | Extended | Measurement and data-export commands. |
| `Filter > Blur > …` (Gallery) | Submenu + on-canvas overlay | Standard + Extended | Blur Gallery. |
| ~~`Filter > Camera Raw Filter`~~ | Menu | n/a | Not CS6 (CC 2013). |
| `Window > Mini Bridge` / Bridge panel | Panel | Standard + Extended | Version of introduction unverified. |
| ~~`Window > Artboards` / `Layer > New Artboard`~~ | Menu + layer type | n/a | Not CS6 (CC 2015). |
| Layers panel search field | Panel control | Standard + Extended | Layer filtering. |

## Parameters & ranges

`None.` This is a release/edition baseline. Per-feature controls and their ranges
belong to the feature specs.

## Algorithms & pipeline

`None.` This document does not define algorithms. Two engine-level facts that
constrain other specs:

- The **Mercury Graphics Engine (MGE)** is new in CS6 and drives GPU-accelerated
  paths; exact algorithms are closed. Treat accelerated features as "behavioral
  parity only" unless a public algorithm is documented. (See
  `01-architecture/system-architecture.md`.)
- **ACR 7** processing is closed; `OVR-003` classifies exact RAW parity as a
  non-goal.

## Rust module mapping

No new modules. Edition handling is a model-level flag so that a single build can
gate Extended surfaces:

- `pictura-core::Edition` (`Standard | Extended`) — implemented; `PICTURA_EDITION`
  selects the surface at startup (default Extended), and the tool/command
  catalogues mark the Extended-only rows so the shell hides them in Standard.

The precise wiring belongs to `01-architecture/rust-core-design.md`; the gate
is implemented in `pictura-core` and the shell catalogues.

## Qt6 component mapping

No new components. Edition gating is a presentation concern: menu actions and
docks for Extended-only features are disabled or hidden when `Edition ==
Standard`. Proposed to live in the Qt action/dock registry described in
`01-architecture/qt6-ui-design.md`, not scattered through feature code.

## Data-model impact

- **PSD/PSB autosave fields.** The published format spec lists CS6 image
  resources `1086` (*Auto Save File Path*, Unicode string) and `1087` (*Auto Save
  Format*, Unicode string). Implementations should preserve these opaque strings
  on round-trip.
- **Measurement data.** Measurement Scale (`1074`, introduced CS3) and Count
  Information (`1080`, introduced CS4) are image-resource descriptors; they are
  Extended-relevant and should round-trip even when the Standard build cannot
  edit them.
- **3D and video layers.** 3D layers (Extended) and video layers (all editions
  in CS6) are layer kinds the document model must represent and preserve, even if
  a build cannot fully render them.

## Edge cases

- **Cross-edition interchange.** A Standard build must open a CS6 Extended PSD
  containing 3D layers and measurement data without corrupting them; it should
  report that the Extended features are unavailable rather than dropping data.
- **3D on unsupported OS.** CS6 Extended 3D was not supported on Windows XP.
  Kooka Pictura has no XP target; the analogous case is "no capable GPU," which
  must degrade explicitly (`OVR-003`, `01-architecture/`).
- **Video across editions.** A spec author must not assume video is
  Extended-only; that was the CS5 arrangement, not CS6.
- **Version drift.** Menu paths and defaults here are for v13; service releases
  (e.g. 13.0.6) may differ. Record the exact version when verifying.
- **Feature-name ambiguity.** "Content-Aware Fill" (CS5) vs. "Content-Aware
  Move/Patch" (CS6) must not be conflated in traceability.

## Parity acceptance criteria

1. Given a build configured as `Standard`, every command in the `Window > 3D`
   and `Analysis` surfaces is absent or disabled, and no Extended-only panel can
   be opened; given `Extended`, they are present.
2. Given a CS6 Extended PSD containing 3D layers and Count Information, a
   Standard build opens it, preserves both on save, and reports the unavailable
   features — no silent data loss.
3. Given the CS6 menu tree, every command tagged `Extended-only` in the corpus
   matches the Extended/Standard split in the "Editions" table above.
4. Given a feature that changed from CS5, its spec states the CS6 behavior and
   does not describe the CS5 arrangement.

## Sources

- `https://www.adorama.com/alc/adobe-photoshop-cs6-creative-cloud-officially-launched/` — republication of Adobe's 23 April 2012 press release: public beta 22 March 2012, feature list, CS6 vs CS6 Extended additions, pricing, Creative Cloud.
- `https://www.theverge.com/2012/4/23/2968192/adobe-cs6-pricing-availability-creative-cloud-announcement` — CS6 announced 23 April 2012; pre-order estimated delivery 7 May 2012; $699 / $999 pricing.
- `https://prodesigntools.com/whats-the-difference-photoshop-cs6-vs-photoshop-cs6-extended.html` — Standard vs Extended differences; video moved to all editions in CS6; 3D / measurement / DICOM remain Extended; Extended-only trial and education editions; 3D unsupported on Windows XP.
- `https://adobe.fandom.com/wiki/Adobe_Photoshop_CS6` — CS6 = v13; feature list (UI redesign/dark UI, auto/background saves, Content-Aware Patch/Move, Blur Gallery, Color Range skin/face, ACR 7, crop/straighten, Properties panel, video, Oil Paint, Adaptive Wide Angle, Paragraph/Character styles, Middle Eastern support, printing UI, 3DLUT, vector tools, snap-to-pixel, 3D UI); last perpetual license; last XP / Snow Leopard release.
- `https://en.wikipedia.org/wiki/Adobe_Photoshop` — version history framing; CS3–CS6 shipped in Standard and Extended editions; PSD/PSB facts; plug-in model; Camera Raw.
- `https://html.duckduckgo.com/html/?q=Photoshop+CS6+Standard+vs+Extended+differences+3D+video+measurement` — search results corroborating the Standard/Extended split.
- `https://search.brave.com/search?q="Photoshop+CS6"+Adobe+press+release+May+7+2012+Creative+Suite+6+announced` — search results corroborating announcement and 7 May 2012 availability.
- `https://search.brave.com/search?q=Photoshop+CS6+new+features+content-aware+move+adaptive+wide+angle+oil+paint` — search results (dpreview, Adobe helpx, Macworld, PCWorld, SitePoint) corroborating the new-feature list; `helpx.adobe.com/photoshop/using/whats-new-cs6.html` is the official page.
- `https://web.archive.org/web/20231122064257/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — CS6 image-resource IDs 1086/1087 (Auto Save), 1074 (Measurement Scale), 1080 (Count Information); format facts.

## Open questions

- **Was Mini Bridge new in CS6 or carried from CS5?** The brief lists it as new;
  public sources are ambiguous and the official `whats-new-cs6` page is
  inaccessible. *Resolves with:* the archived CS6 Help PDF
  (`https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`) or
  a CS5/CS6 side-by-side of the `Window` menu.
- **Was Camera Raw as a filter (`Filter > Camera Raw Filter`) introduced in CS6
  or later?** Widely attributed to CS6, not confirmed against a primary source
  here. *Resolves with:* the CS6 Help PDF's Filter reference.
- **Exact CS6 menu placement of Artboards and Content-Aware tools** (tool-slot
  grouping changed in CS6) needs a screenshot-level check. *Resolves with:* an
  archived CS6 UI reference or first-hand capture.
- **Which CS6 service release is the parity baseline** (13.0, 13.0.6, …) affects
  defaults and bugfix behavior. *Resolves with:* a decision recorded here and in
  `11-cross-cutting/update-and-versioning.md`.
- **DICOM fidelity requirements for Extended** are not yet scoped. *Resolves
  with:* `10-workflow-io/` and a decision on whether DICOM is a non-goal
  (`OVR-003`).
