# Gap Analysis

- **Spec ID:** `XC-021`
- **Status:** `Draft`
- **Parity tier:** `Core`
- **New in CS6:** `No`
- **Depends on:** every spec in the corpus; `INDEX.md`, `TRACEABILITY.md`.

## CS6 behavior

Assessment of what the corpus covers, what it does not, and where the risks sit.
This is the "are we done speccing?" document. It is intentionally blunt.

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

The corpus is "spec-complete" when: (1) every CS6 menu item and tool maps to a
spec, (2) every spec passes the 11-heading template check, (3) every
`TRACEABILITY.md` row has a real spec path, and (4) no spec asserts an unverified
CS6 behavior without an `## Open questions` entry.

## Coverage summary

| Area | Files | Coverage | Notes |
|---|---|---|---|
| `00-overview` | 4 | Complete | Product scope, editions, feasibility, legal. |
| `01-architecture` | 13 | Complete (draft) | All major subsystems specced; interop/GPU need prototype validation. |
| `02-ui-ux` | 7 + 20 panels | Complete | Menus/shortcuts exhaustive; some menu-enable matrices `(to verify)`. |
| `03-tools` | 30 | Complete | Every CS6 tools-panel tool (Standard + Extended) has a spec. |
| `04-image-ops` | 8 + 26 adjustments | Complete | All CS6 adjustments and mode/depth docs specced. |
| `05-layers` | 17 | Complete | Blend modes, styles, smart objects, comps; artboards/linked marked non-CS6. |
| `06-filters` | 23 | Complete | All CS6 filter families + specials; post-CS6 filters walled off. |
| `07-color-painting` | 13 | Complete | Color model, picker, brush engine, presets. |
| `08-selection` | 10 | Complete | Selection model, Refine Edge, channels/paths. |
| `09-automation` | 10 | Complete | Actions, scripting, plugin SDK, Rust replacement design. |
| `10-workflow-io` | 14 | Complete | Lifecycle, save/export, metadata, color settings, print, memory. |
| `11-cross-cutting` | 10 | Complete | i18n, prefs, logging, errors, security, testing, recovery, this analysis. |
| **Total** | **~202** | | |

## What is genuinely missing

| Gap | Why it matters | Action |
|---|---|---|
| Prototype validation of `wgpu`↔`QRhi`, CXX-Qt, OpenCL-gated filters | Architecture feasibility rests on unproven interop | Out of scope (docs-only); logged in `ARCH-006`/`ARCH-004` and `open-questions.md` §E |
| CS6 reference-render corpus | "Behavioral parity" cannot be tested without reference outputs | Legal question first (`open-questions.md` §F); then build locally |
| Per-feature numeric defaults | Many controls have inferred defaults | Scripted CS6 readback or UI capture |
| PSD descriptor internals | Write fidelity for modern blocks | Dump/diff CS6-authored PSDs |
| Menu enable/disable matrix | `UI-002` items marked `(to verify)` | CS6 capture |
| Full link audit of provisional ARCH IDs | Cross-refs may point to siblings | One documentation pass |
| `.8bf` plugin binary compatibility | Adobe ABI cannot load on Linux | Declared non-goal (`AUTO-012`, `ARCH-011`) |

## Non-goals (documented, not silently dropped)

- **3D** — Extended-only; Adobe itself removed 3D in 22.5 (2021). Proposed `Non-goal (Linux)`. (`03-tools/3d-tools.md`, `02-ui-ux/panels/3d-panel.md`)
- **Artboards** — not CS6 (CC 2015). (`05-layers/artboards.md`)
- **Linked smart objects** — not CS6 (CC 2014). (`LAY-024`)
- **Adobe cloud / Stock / Bridge / Mini Bridge** — proprietary or Flash-based. (`10-workflow-io/bridge-and-interop.md`)
- **Adobe plugin binaries (`.8bf`)** — wrong ABI and license-gated. (`AUTO-012`)
- **Native print parity** — CUPS/Qt6 path proposed; some CS6 print features (DEVMODE/NSPrintInfo, 16-bit data) are explicit non-goals. (`WF-013`)
- **Post-CS6 features that contaminated the brief** — Shake Reduction, Face-Aware Liquify, Flame/Tree, Color Themes panel, live-shape corner editing, linked SO, artboards. All documented as non-parity. Four of them shipped anyway, by explicit decision and labelled non-parity in the code: the Camera Raw filter (as Pictura Raw), Path Blur, Spin Blur, and Picture Frame.
- **Snap and AppImage packaging** — **not** dropped. Both ship alongside **Flatpak + native `.deb`/`.rpm`**: Flatpak was chosen for confinement, not as a replacement. (`snap/snapcraft.yaml`, `.github/workflows/snap.yml`, `.github/workflows/appimage.yml`)

## Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| Color-management parity (ICC, soft proofing) | High | lcms2 + rigorous testing (`XC-010`) |
| PSD **write** fidelity | High | Round-trip harness; preserve-unknown-blocks policy |
| GPU interop uncertainty | High | Prototype spike before committing |
| Closed algorithms (healing, content-aware, filters) | Medium | Behavioral parity + calibration, named as TBD |
| CS6 Help PDF is CC-contaminated | Medium | Cross-check date-sensitive claims (flagged) |
| Scope of ~202 specs is large | Medium | This phase is documentation; implementation sequenced later |
| Legal (independent-creation, patents, assets) | High | Counsel review before any distribution |

## Parity acceptance criteria (corpus-level)

- Template conformance: automated heading check across all spec files — currently **0 failures**.
- Spec-ID uniqueness across declared IDs — currently **0 duplicates** after de-duplication.
- Every `TRACEABILITY.md` row resolves to an existing file.
- No CS6-version claim without a source or an open-question entry.

## Sources

- `https://help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
- Per-spec `## Sources` sections.

## Open questions

- **Menu enable/disable matrix** — resolve with a CS6 capture.
- **Is the corpus spec-complete for a planning decision?** Subjective; a domain-expert review per area would raise confidence. Resolve by review sign-off per directory.
- **Reference corpus legality** — resolve with counsel.
- **Which non-goals to revisit first** — resolve with a product decision.
