# Open Questions — Consolidated Register

- **Spec ID:** `XC-020`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No`
- **Depends on:** every spec in the corpus.

## CS6 behavior

This file is the single register of unresolved questions. Each feature spec has
its own `## Open questions`; this file collects the ones that either (a) affect
more than one spec, (b) are version/attribution corrections that change parity
scope, or (c) block the architecture. Entries are grouped, and every entry names
the artifact that would resolve it.

## UI surface

`None.`

## Parameters & ranges

`None.`

## Algorithms & pipeline

`None.`

## Rust module mapping

`None.`

## Qt6 component mapping

`None.`

## Data-model impact

`None.`

## Edge cases

`None.`

## Parity acceptance criteria

Every question below is closed by producing the named artifact. A spec may not
move to `Verified` while a question it depends on is open.

## A. Version / attribution corrections (change parity scope)

These were discovered during research and contradict the original plan or early
drafts. All are now corrected in the affected specs; listed here so reviewers can
audit the decisions.

| # | Claim in original plan | Verified reality | Affected specs | Resolver |
|---|---|---|---|---|
| A1 | Artboards are NEW in CS6 | Artboards added in **CC 2015**; not in CS6 | `LAY-023`, `ARCH-008`, `GLOSSARY`, `00-overview/cs6-editions-and-constraints.md`, `05-layers/align-and-distribute.md` | Adobe PSD spec labels the block "Artboard Data (Photoshop CC 2015)". |
| A2 | Linked smart objects are CS6 | Added in **CC 2014 (14.2)** | `LAY-024` | Adobe PSD spec labels "Smart Object Layer Data (Photoshop CC 2015)". |
| A3 | Camera Raw as a filter is CS6 | Camera Raw **filter** added in **CC 2013 (ACR 8)**; CS6 has ACR 7 for open/open-as-Smart-Object only | `FILT-100`, `00-overview/cs6-editions-and-constraints.md` | Adobe What's-New / ACR 8 release notes. |
| A4 | Shake Reduction is CS6 | Added in **CC 2013** | `FILT-093`, `FILT-020` | Adobe CC release notes; absent from CS6 Help PDF. |
| A5 | Extract / Pattern Maker removed in CS6 | Removed from default install in **CS4**; optional plug-ins in CS6 | `FILT-104` | CS4 release notes; CS6 Help "optional plug-ins". |
| A6 | Blur Gallery has Field/Iris/Tilt-Shift/Path/Spin | CS6 has **Field, Iris, Tilt-Shift only**; Path/Spin Blur and Blur Effects noise/edge-glow are **CC**. Noise, Path Blur, and Spin Blur are implemented anyway by owner decision, labelled non-parity | `FILT-092` | CS6 Help Blur Gallery topic vs CC Help. |
| A7 | Flame / Tree / Picture Frame are CS6 Render filters | Added in **CC 2014.2** | `FILT-060` | CS6 Help Render menu vs CC 2014.2 update. |
| A8 | Face-Aware Liquify is CS6 | Added in **CC 2015.5** | `FILT-090` | CC release notes. |
| A9 | Refine Edge expanded in CS6 | Smart Radius/Decontaminate are **CS5**; Select-and-Mask "Remember Settings"/Mask view are **CC** | `SEL-003` | CS5 Help vs CS6 Help. |
| A10 | Rotate View is new in CS5 | Introduced in **CS4** | `TOOL-043` | CS4 materials. |
| A11 | Mixer Brush is CS6 | Introduced in **CS5** | `TOOL-022`, `BRU-004` | CS5 What's-New. |
| A12 | Design workspace preset exists in CS6 | CS6 presets are Essentials/Painting/Photography/Typography (+3D) | `UI-003` | CS6 Help workspace topic. |
| A13 | Color Lookup / 3DLUT adjustment is CS6 | Attributed to CS6 by the plan; the CS6 Help prose omits it though it carries a PSD key. **Unresolved.** | `LAY-012`, `ADJ-*` | CS6 release note or screenshot. |
| A14 | Timeline/video is Extended-only in CS6 | Video timeline shipped in **all CS6 editions** (Extended-only in CS5), though the CS6 Help PDF retains stale Extended labels | `PAN-...`, `3d-tools`, `cs6-editions` | Adobe CS6 press release vs Help PDF; needs a CS6 capture. |
| A15 | Group layer styles are CS6 | Help says groups excluded; a community CS6 reference says added. **Unresolved.** | `LAY-011` | CS6 screenshot / release note. |

## B. Adobe-closed algorithms (behavioral parity only)

Closed implementations that must be specified by behavior and calibrated, never
guessed. Each spec already marks these "behavioral parity only, algorithm TBD".

- Healing Brush / Spot Healing biharmonic and PatchMatch variants — `TOOL-031`, `TOOL-032`.
- Content-Aware Move/Patch/Fill solver — `TOOL-032`.
- All Filter Gallery kernels, Liquify mesh solver, Oil Paint, Blur Gallery bokeh, Lighting Effects BRDF, Lens Correction profile solver, Vanishing Point solver — `06-filters/*`.
- Levels/Curves gamma exponent direction and Brightness/Contrast normal-mode curve — `ADJ-001/002/003`.
- Vibrance falloff/skin-tone band; Selective Color relative/absolute path — `ADJ-005`, `ADJ-023`.
- Blend-mode integer rounding and extended-range clamps (Photoshop-only modes: Linear Burn/Dodge, Vivid/Linear/Pin Light, Hard Mix, Subtract, Divide, Darker/Lighter Color) — `LAY-010`.
- Smart-guide/transform interpolation selection heuristics.
- **Resolver:** a `13.0.1` reference-render corpus plus calibrated tolerances (`XC-010`).

## C. Undocumented defaults and control ranges

The CS6 Help prints prose, rarely numeric ranges. Values used across the corpus
are frequently from the CS6 **AppleScript/ExtendScript** references (authoritative)
or from community CS6-era sources (marked inferred). Highest-impact gaps:

- Filter Gallery per-filter defaults/ranges — no Adobe numeric source.
- Layer-style control ranges (only Size 0–250 px and Stroke cap 250 px are corroborated).
- History States default 20 vs 50; Cache Levels 4 vs 6 — sources conflict.
- Scratch-disk RAM allocation 60% vs 70% — sources conflict.
- Brush engine defaults (spacing, jitter, Wet/Load/Mix).
- **Resolver:** scripted readback from a CS6 install, or a UI capture corpus.

## D. PSD/PSB serialization gaps

- Artboard (`artb`/`artd`/`abdd`), smart-object (`SoLd`/`SoLE`), linked (`lnkD`/`lnk2`/`lnk3`), filter-mask (`FMsk`), filter-effects (`FXid`/`FEid`) descriptor internals not enumerated in the fetched spec excerpts.
- Layer Comps image resource **1065** descriptor schema.
- Data-driven graphics image resources **7000/7001** XML schema (a community attempt to write 7000 was ignored by Photoshop).
- Non-destructive crop, color-sampler, slice, note/count serialization keys.
- Duotone Color Mode Data blob format (spec says "undocumented").
- **Resolver:** dump real CS6-authored PSDs and diff against the writer.

## E. Technology-interop unknowns

- **`wgpu` ↔ Qt `QRhi`** device/shared-texture interop (and `QVulkanInstance` wrapping) is unverified and requires a Linux prototype — `ARCH-006`.
- **CXX-Qt** API churn and lack of Rust `QWidget` bindings — `ARCH-004`; needs a version-pinned evaluation.
- Whether Blur Gallery / Adaptive Wide Angle hard-refuse to run without OpenCL — `FILT-092/094`.
- Copper/Bristle brush rendering fidelity — `BRU-003`.
- **Resolver:** a throwaway prototype spike (out of scope for this docs phase).

## F. Legal / independent-creation

- Redistributing a CS6-authored reference corpus for parity testing — `XC-010`, `00-overview/licensing-and-provenance.md`.
- Whether behavioral replication of patented filters (e.g. healing) requires a patent review — flagged for counsel.
- Fonts and `.abr`/`.csh`/`.pat` presets bundled by Adobe must not be copied.
- **Resolver:** legal counsel review.

## G. Corpus-level

- Cross-references use provisional ARCH IDs; some references are path-based to avoid the earlier ID collisions. IDs were de-duplicated (ARCH-003/004/011) but older cross-refs may point to the wrong sibling — a link-audit pass is owed.
- The CS6 Help PDF is a rolling document (cover Feb 2013, modified 2017) containing some Creative-Cloud-marked material; CS6 attributions that came only from it may be CC-contaminated (flagged in several specs).

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf` — primary CS6 Help (rolling, CC-contamination caveat).
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — Adobe PSD/PSB File Formats Specification.
- Per-spec `## Sources` sections for feature-level citations.

## Open questions

This file **is** the open-question register. The list above is the authoritative
set; per-spec sections may hold finer-grained items that roll up here when they
cross spec boundaries.
