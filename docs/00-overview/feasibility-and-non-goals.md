# Feasibility and Non-Goals

- **Spec ID:** `OVR-003`
- **Status:** `Draft`
- **Parity tier:** `N/A` — this is an honest assessment, not a feature.
- **New in CS6:** `N/A`.
- **Depends on:** `OVR-001`, `OVR-002`, `OVR-004`, `01-architecture/system-architecture.md`, `01-architecture/color-management.md`, `01-architecture/plugin-and-scripting-abi.md`.

This document states what reaching CS6 behavioral parity realistically involves,
what is hard, and what is explicitly **not** attempted. Difficulty ratings are
judgement calls based on the cited sources and are marked as such. Exclusions
carry a rationale and a revisit condition; nothing here is dropped silently.

## CS6 behavior

For this assessment, "CS6 behavior" is the parity target defined in `OVR-001`:
same user-visible results within a stated tolerance for documented operations.

### What is realistic

| Area | Assessment | Basis |
|---|---|---|
| Document model, layers, groups, masks, blending | Realistic. The PSD/PSB format and blend-mode set are documented. | Format spec; `ARCH-002` |
| Core raster tools (move, marquee, lasso, brush engine, eraser, gradient, paint bucket, clone/pattern, type, pen/paths, shape) | Realistic for behavioral parity; brush dynamics and tip rendering need empirical matching. | CS6 Help; `03-tools/` |
| Adjustments and blend modes | Realistic. Formulas are standard and partly documented. | Photoshop blend-mode literature; `04-image-ops/` |
| File I/O for open standards (PNG, JPEG, TIFF, OpenEXR) | Realistic. | Standards; `pictura-codec` |
| PSD/PSB **read** | Realistic for the documented sections; unknown resources are preserved opaquely. | Format spec |
| Undo/history, panels, preferences, workspaces, shortcuts | Realistic. | `02-ui-ux/` |
| Scripting/automation replacement | Realistic as a **substitute**, not as ExtendScript compatibility. | `09-automation/` |

### What is hard

| Hard problem | Why | Difficulty (judgement) |
|---|---|---|
| **Color-management parity** | Adobe's color engine (ACE) is closed; transforms, rendering intents, black-point compensation, and soft-proofing will differ from an ICC-based engine (e.g. Little CMS) at the pixel level. Adobe-supplied CMYK profile names/behavior are licensing-sensitive. | High |
| **PSD/PSB write fidelity** | The published spec leaves parts undocumented (e.g. duotone specification; descriptor semantics; some layer keys). Byte-exact round-trips against CS6 are not a safe target; preserving unknown blocks and matching common readback is. | High |
| **GPU parity (Mercury Graphics Engine)** | MGE algorithms and scheduling are closed. An independent GPU compositor (wgpu/QRhi) can meet performance and behavior budgets but not reproduce Adobe's exact output. | High |
| **Full filter set** | Many filters (Artistic, Brush Strokes, Sketch, Texture; Oil Paint; Lighting Effects; exact Liquify) are closed algorithms and partly media-specific. | High |
| **Content-Aware Fill/Move/Patch and Refine Edge** | Closed, patent-sensitive algorithms; only approximate behavioral results are defensible. | High |
| **Type engine** | CS6's text layout, OpenType features, and font metrics engine are closed; only Apple/Windows system text engines produced the metrics. | High |
| **Raw processing parity (ACR 7)** | ACR's demosaic, noise, and tone pipelines are proprietary. | High |
| **Actions / ExtendScript compatibility** | The JSX object model and recorded-action semantics are Adobe-specific. A substitute API can be provided, not binary/script compatibility. | Medium–High |
| **Performance parity on large PSB documents** | Achievable in principle via tiling and scratch, but requires measurement against CS6 budgets. | Medium |
| **3D (Extended)** | A full proprietary 3D engine plus authoring workflow. | Very high |

These are *hard*, not impossible. The failure mode to avoid is claiming exact
parity where the implementation is closed; specs use "behavioral parity only,
algorithm TBD" for those cases.

### What could be revisited

- 3D may be revisited as an optional module using a standard glTF/PBR pipeline;
  exact CS6 3D parity would remain a non-goal.
- Raw processing may be revisited with open raw libraries and best-effort
  matching, not exact ACR emulation.
- Video may be revisited for decode/import even if the CS6 timeline is not
  reproduced.
- Printing may be revisited for basic ICC/CUPS output even though Adobe print
  fidelity is not a target.

## UI surface

`None.` This document does not introduce UI.

## Parameters & ranges

This section is adapted to hold the **non-goal register**. Each entry names what
is excluded, why, and when it could return.

| Excluded from parity | Rationale | Revisit condition |
|---|---|---|
| **3D (Extended)** | Closed engine; very large scope; marginal to a Linux 2D editor's core. | A community/optional module; never exact CS6 parity. |
| **Video / Timeline** (all CS6 editions) | Exact timeline semantics and legacy codec support; video is a large domain adjacent to a raster editor. | A decode/import feature via FFmpeg, without CS6 timeline parity. |
| **Measurement engine / DICOM** (Extended) | Niche; DICOM is a separate medical-imaging domain. The Count tool ships, but the measurement scale, its exporters, and the Measurement Log panel do not. | Demand plus an open DICOM library (e.g. GDCM) and a parity decision per command. |
| **Printing fidelity** | Adobe's print path and color engine are proprietary; output depends on the OS/driver. | Basic CUPS + ICC printing as a separate feature; exact proofing parity not promised. |
| **Adobe cloud services** (Creative Cloud, Cloud Documents/PSDC, Behance, Stock, Adobe Fonts/Typekit) | Proprietary, account-bound, privately specified (`PSDC` is explicitly private in the format spec). | Not planned; separate integrations would be independently specified. |
| **Adobe Camera Raw parity** | Proprietary pipeline and camera support; patents. | Best-effort raw import via open libraries. |
| **Adobe plug-in (`.8bf`) binary compatibility** | The Photoshop SDK and host ABI are proprietary, licensed, and not safe for independent creation (`OVR-004`). | An independently specified plug-in ABI and script host instead. |
| **ExtendScript / JSX compatibility** | Adobe-specific object model and Action Manager API. | A documented substitute automation API (`09-automation/`). |
| **OS integration** (AppleScript, Windows COM, macOS resource forks, QuickTime, Windows-only 30-bit display, Adobe PDF print engine) | No Linux equivalent or platform-specific by design. | Provide Linux-native equivalents (D-Bus, CLI, CUPS) where useful. |
| **Bundled Adobe assets** (brushes, patterns, gradients, swatches, profiles, fonts, plug-ins) | Copyright/licensing; see `OVR-004`. | Independently authored replacements only. |
| **Byte-exact PSD output** | Spec is incomplete; Adobe writers are not documented to byte level. | Preserve-and-round-trip semantics; revisit only with a comparison harness. |
| **32-bit Windows build** | Not a Linux target. | Not planned. |

## Algorithms & pipeline

Difficulty is driven by what is documented versus closed. The following
classifications are proposals for spec authors to apply consistently:

- **Documented/standard (parity achievable):** PSD/PSB container; PNG/JPEG/TIFF;
  ICC transforms (via an ICC engine); blend modes (standard formulas); many
  classical filters (Gaussian, unsharp mask variants, median, motion blur).
- **Closed but observable (behavioral parity only):** Blur Gallery, Oil Paint,
  Adaptive Wide Angle, most Artistic/Sketch/Texture filters, Content-Aware,
  Refine Edge, ACR, type layout, MGE-accelerated paths.
- **Patented or licensing-sensitive:** content-aware inpainting family, some
  selection/edge algorithms, Adobe-supplied ICC profiles, and the raw pipeline.
  Treat as "design around or approximate"; do not assume an independent-creation
  reimplementation avoids patent exposure.

The pipeline itself is not the risk; `01-architecture/` already proposes a
tiled CPU/GPU architecture. The risk is the **per-feature algorithm gap**, which
each feature spec must state honestly.

## Rust module mapping

No new modules are introduced by this assessment. Feasibility constrains scope,
which is expressed through:

- feature registration and edition gating (`pictura-core::Edition`, `OVR-002`);
- the plugin/scripting ABI being an independent design, not an Adobe-compatible
  one (`pictura-script`, `ARCH-011`);
- color management being explicitly ICC-based (`pictura-color`,
  `ARCH-007`).

## Qt6 component mapping

`None.` Non-goals remove surfaces (3D panel, Timeline, Measurement Log, cloud
panels) rather than adding any.

## Data-model impact

Non-goals still affect the model, because excluded features can appear in files
we must open:

- **3D layers** (Extended) and **video layers** (all editions in CS6) must be
  representable and preservable even when not fully rendered.
- **Measurement/Count descriptors** (`1074`, `1080`) round-trip opaquely.
- **Print info / print style resources** (`1082`, `1083`) round-trip even though
  printing fidelity is not targeted.
- **Private cloud format (`PSDC`)** is out of scope; opening `.psdc` is not
  attempted.
- **Unknown layer keys and image resources** are preserved verbatim on save so a
  non-goal feature does not cause data loss.

## Edge cases

- **Opening files that use non-goal features** must not fail or corrupt them;
  preserve-and-report is the rule (see `ARCH-002`).
- **Legal edge cases** (patents, trademark, EULA anti-analysis) are
  not technical and are handled in `OVR-004`; feasibility work must not assume
  they are solved.
- **"Approximate" must be visible.** Where parity is behavioral-only, the
  implementation should not present the result as identical to CS6.
- **Tolerance creep.** If color/raster tolerances are loosened to pass an
  inpainting comparison, that must be recorded, not hidden in a test constant.
- **Dependency and asset licensing** can invalidate a "feasible" plan late; the
  license inventory is a gating artifact (`OVR-004`).

## Parity acceptance criteria

1. Given any feature spec, its parity claim is one of: exact (documented
   algorithm), behavioral-with-tolerance (observable, algorithm TBD), or
   non-goal. No fourth category.
2. Given the non-goal register, each entry has a rationale and a revisit
   condition; a maintenance review can re-evaluate each without re-reading the
   whole corpus.
3. Given a CS6 file that contains a non-goal feature, opening and re-saving it
   preserves that feature's data and reports it as unsupported.
4. Given a color-critical operation, a documented tolerance (e.g. maximum ΔE or
   per-channel error) is stated and testable before the feature is marked
   `Spec'd`.
5. Given the plugin decision, the corpus contains no requirement — explicit or
   implicit — that `.8bf` Adobe plug-ins load.

## Sources

- `https://web.archive.org/web/20231122064257/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — the published PSD/PSB spec: "does not explain how to interpret the data"; explicitly excludes the private `PSDC` cloud format; documents duotone specification as undocumented; lists sections/resources. Basis for the PSD write-fidelity and cloud non-goals.
- `https://en.wikipedia.org/wiki/Adobe_Photoshop` — plug-in model (`8bf` filter plug-ins, third-party ecosystem), Camera Raw, PSD/PSB limits. Basis for the plug-in and raw non-goals.
- `https://en.wikipedia.org/wiki/Clean-room_design` — independent-creation design does not defend against patents. Basis for the patent-risk classification.
- `https://prodesigntools.com/whats-the-difference-photoshop-cs6-vs-photoshop-cs6-extended.html` — 3D/measurement/DICOM as Extended-only; scope of the 3D and measurement non-goals.
- `https://www.adorama.com/alc/adobe-photoshop-cs6-creative-cloud-officially-launched/` — Adobe press release: Mercury Graphics Engine, 3D engine, Creative Cloud. Basis for the GPU-parity and cloud non-goals.
- `https://search.brave.com/search?q=Photoshop+CS6+new+features+content-aware+move+adaptive+wide+angle+oil+paint` — search results confirming the presence and breadth of the closed-algorithm feature set (Content-Aware, Blur Gallery, Adaptive Wide Angle, Oil Paint).
- `(web search)` — search results on independent-creation limits (Cornell LII, Finnegan, Sedona Conference, Reed Smith), including that independent-creation rewriting does not answer trade-secret or patent claims.

## Open questions

- **What pixel/color tolerance is acceptable for "parity"?** *Resolves with:* a
  decision and a comparison harness design in
  `11-cross-cutting/testing-strategy.md`.
- **Is 3D worth an optional module** or permanently a non-goal? *Resolves with:*
  an ADR after the Core tier stabilizes.
- **Which raw formats must open at all** (regardless of ACR parity) is unscoped.
  *Resolves with:* `10-workflow-io/camera-raw-workflow.md`.
- **Can the type engine reach acceptable parity** using system fonts, or does it
  require its own layout engine? *Resolves with:* a text-layout spike and
  `03-tools/type-tools.md`.
- **Does any candidate algorithm fall under in-force patents** in target
  markets? This is a legal question, not an engineering one. *Resolves with:*
  freedom-to-operate review by counsel (`OVR-004`).
- **What replaces the Adobe plug-in ecosystem**, and is a third-party ABI
  required at all? *Resolves with:* `ARCH-011` and `09-automation/plugin-sdk.md`.
