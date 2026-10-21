# Product Overview

- **Spec ID:** `OVR-001`
- **Status:** `Draft`
- **Parity tier:** `N/A` — this document frames the whole effort; it is not a feature.
- **New in CS6:** `N/A` — describes the v13 (CS6) release as a whole.
- **Depends on:** `README.md`, `GLOSSARY.md`, `INDEX.md`, `SPEC_TEMPLATE.md`, `OVR-002`, `OVR-003`, `OVR-004`, `01-architecture/`.

Kooka Pictura is a working application **and** the specification it was built
from. It reimplements Adobe Photoshop CS6 (version 13, 2012) as a native Linux
application in **Rust** (core) and **Qt6** (UI), targeting *behavioral parity*
with CS6. The implementation lives in `crates/`; every `## Rust module mapping`
and `## Qt6 component mapping` section is a design proposal, and
`docs/dev/doc-code-parity-gaps.md` records what shipped and what did not.

## CS6 behavior

For this overview document the contract is the **product scope**, not a single
feature.

### What parity means here

Behavioral parity is scoped, not absolute. For a defined set of CS6 operations,
given the same inputs, Kooka Pictura must produce the same user-visible result
within a stated tolerance, expose the same affordances, and preserve the same
document semantics. Parity does **not** mean:

- byte-identical file output (PSD writers legitimately differ in descriptors,
  padding, and the merged composite);
- identical internal algorithms (most Adobe implementations are closed);
- identical performance or identical GPU behavior;
- a clone of Adobe branding, assets, or installers.

"Behavioral parity only, algorithm TBD" is a valid and expected outcome for any
feature whose exact Adobe implementation is not public. Specs say so explicitly
rather than guessing.

### Audience

| Audience | What they use this corpus for |
|---|---|
| Implementers | Module/component proposals, parameter ranges, acceptance criteria. |
| Spec authors / reviewers | The template, status legend, and traceability matrix show what is written and what is missing. |
| Designers | UI surface inventories, dark-UI and workspace behavior, accessibility. |
| End users / evaluators | Honest statements of what will and will not work on Linux. |
| Legal / counsel | `OVR-004` records trademark, independent-creation, and license constraints for review. |

### What "done" means

A single spec is done when its status reaches `Verified`: sourced, cross-reviewed
against `SPEC_TEMPLATE.md`, and with parity criteria that a human or an automated
harness could execute. The corpus is done when:

1. Every CS6 user-visible command in `02-ui-ux/menus.md` maps to a spec, a
   component spec, or an explicit non-goal in `OVR-003`.
2. `TRACEABILITY.md` has no `planned` rows in the Core tier.
3. Every non-goal has a named rationale and a revisit condition.
4. No spec asserts an unverified fact as fact; unknowns live under
   `## Open questions`.

Program-level "done" for the implementation in this repository is narrower: the
**Core** editing workflow reaches parity. Extended-only capabilities are out of
the first parity target (see `OVR-003`), with the Count tool as the one that
ships ahead of that line. The corpus marks the tier; the code does not gate on
it, and edition gating is still open (#650).

### How the specs are organized

| Area | Contents |
|---|---|
| `00-overview/` | Product scope, CS6 editions and feature set, feasibility and non-goals, legal/provenance posture. |
| `01-architecture/` | Layering, Rust core, Qt6 shell, FFI, GPU, color, document model, undo, plugins, formats. Read first. |
| `02-ui-ux/` | Application frame, menus, panels, preferences, shortcuts, accessibility. |
| `03-tools/` … `10-workflow-io/` | Feature domains: tools, image ops, layers, filters, color/painting, selection, automation, I/O. |
| `11-cross-cutting/` | Localization, storage, testing, error handling, security, recovery. |
| `TRACEABILITY.md` | CS6 feature → spec → Rust module → Qt component matrix. |

The canonical section order is fixed by `SPEC_TEMPLATE.md` so specs are
diff-able and machine-checkable. Overview docs keep the same `##` headings and
adapt their content: `## CS6 behavior` carries scope, `## UI surface` is
`N/A`, and so on.

### Honesty statement

Photoshop CS6 is end-of-life and unsupported by Adobe; Adobe, Photoshop, and
related marks are Adobe trademarks. This corpus is a documentation-first behavioral
specification produced from public documentation and observation, with no Adobe
source code, binaries, decompilation, or bundled Adobe assets (`OVR-004`).
Several CS6 behaviors are documented only at the user-interface level, and the
corresponding gaps are recorded as open questions rather than filled by
inference.

## UI surface

`None.` The product-level UI is specified under `02-ui-ux/`; this document does
not introduce UI of its own.

## Parameters & ranges

`None.` User-visible controls are enumerated per feature; this document defines
no controls.

## Algorithms & pipeline

`None.` Algorithms are specified per feature. The implementation pipeline the
specs assume is the one in `01-architecture/system-architecture.md`: decode a
file into a tiled document model, apply a command, composite tiles (GPU with CPU
fallback), and persist. Overview docs do not restate it.

## Rust module mapping

No new modules. The proposed top-level workspace is defined in
`01-architecture/rust-core-design.md` and summarized here for orientation:
`pictura-core` (document graph), `pictura-color`, `pictura-codec`,
`pictura-render`, `pictura-filters`, `pictura-script`, `pictura-qt` (the only
Qt-linked crate), and the `pictura-app` binary. Names are provisional.

## Qt6 component mapping

`None.` The proposed Qt6 surface is defined in
`01-architecture/qt6-ui-design.md` (provisional `ARCH-003`).

## Data-model impact

`None.` The document model is defined in
`01-architecture/rust-core-design.md` (`ARCH-002`).

## Edge cases

- **Edition split.** Standard and Extended differ (3D, measurement, DICOM); see
  `OVR-002`. The corpus targets Standard first, with Extended entries marked
  `Extended-only`.
- **Platform-only features.** macOS/Windows integration, Adobe cloud services,
  and the Adobe plug-in ecosystem have no Linux equivalent and are handled in
  `OVR-003`, not silently dropped.
- **Unverifiable behavior.** Where Adobe's implementation is closed and no
  reliable public description exists, the spec states "behavioral parity only,
  algorithm TBD" and names what would resolve it.

## Parity acceptance criteria

1. Given the CS6 menu tree, every command resolves to a spec ID or an explicit
   non-goal; no command is unaccounted for.
2. Given any spec in the corpus, its `## Sources` lists URLs actually consulted
   and its `## Open questions` names what would resolve each unknown.
3. Given a Core-tier feature, its parity criterion is stated as "Given X, doing Y
   produces Z within tolerance T" and is checkable without access to Adobe
   software for the assertion itself.
4. Given the repository, it contains a working implementation and no Adobe
   assets; every mapping section is labeled a proposal.

## Sources

- `https://en.wikipedia.org/wiki/Adobe_Photoshop` — Photoshop is proprietary, developed by Adobe; PSD/PSB format limits; plug-in model. Establishes the subject and scope.
- `https://www.adorama.com/alc/adobe-photoshop-cs6-creative-cloud-officially-launched/` — Adobe press release text (23 April 2012) for Photoshop CS6 / CS6 Extended; feature list, editions, pricing, Creative Cloud. Establishes release identity.
- `https://web.archive.org/web/20231122064257/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — Adobe Photoshop File Formats Specification: "provided for 3rd parties to read and write the Photoshop native file format." Establishes that format interoperability is an intended use.
- `https://en.wikipedia.org/wiki/Clean-room_design` — independent-creation method and its limits (copyright vs. patents). Establishes the method referenced by the parity definition.

## Open questions

- **Is "behavioral parity" the right end goal**, or should the project target
  workflow parity and accept divergence in closed algorithms? *Resolves with:*
  a product decision recorded as an ADR in `11-cross-cutting/`.
- **Which parity tolerances are acceptable** for color and raster output (e.g.
  ΔE thresholds, max pixel error) is undecided. *Resolves with:*
  `01-architecture/color-management.md` and a comparison harness proposal in
  `11-cross-cutting/testing-strategy.md`.
- **Whether Extended features enter the first target at all** is deferred to
  `OVR-003` and `11-cross-cutting/gap-analysis.md`.
