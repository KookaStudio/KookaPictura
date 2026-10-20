# Licensing and Provenance

- **Spec ID:** `OVR-004`
- **Status:** `Draft`
- **Parity tier:** `N/A` — process and legal posture, not a feature.
- **New in CS6:** `N/A`.
- **Depends on:** `OVR-001`, `OVR-002`, `OVR-003`, `01-architecture/plugin-and-scripting-abi.md`.

> **Not legal advice.** This document records engineering constraints and open
> legal questions for counsel to review. It does not decide legality. Every
> statement about copyright, trademark, patents, or licenses is a risk
> description, not a legal conclusion.

This document states the trademark posture, the independent-creation method used by this
corpus, the asset and dependency policy, the treatment of the PSD format, and
why Adobe plug-in compatibility is not an objective.

## CS6 behavior

For this document the "behavior" under discussion is **how the corpus was
produced and what it may contain**, because that determines whether the rest of
the specs are safe to implement and distribute. The parity target itself is
defined in `OVR-001`/`OVR-002`.

### Trademark posture

- **Adobe, Photoshop, the Photoshop logo, and related marks are trademarks of
  Adobe Inc.** Adobe does not approve of the marks being used generically or as
  a verb. ([Wikipedia](#sources).)
- Kooka Pictura does **not** include "Photoshop" or "Adobe" in its product name,
  and must not use Adobe logos or trade dress.
- References to "Photoshop CS6" in this corpus are **nominative** references to
  identify the compatibility target. They must not imply sponsorship,
  affiliation, or endorsement. A conspicuous disclaimer of affiliation should
  accompany any user-facing distribution that names the mark.
- Adobe-coined feature names (e.g. "Mercury Graphics Engine", "Content-Aware
  Move") may themselves be marks. User-facing strings should prefer descriptive
  names, keeping the Adobe term only where it is needed to identify the behavior
  being specified.
- **Project feature naming.** Our camera-raw feature is user-facing as the
  descriptive **Pictura Raw**. Adobe's "Camera Raw" terms appear only as on-disk
  format identifiers required for compatibility (e.g. the `Adobe Camera Raw
  Filter` descriptor class ID and smart-filter id `2683` that Photoshop writes)
  and in nominative references to Adobe's feature. See
  [`NOTICE.md`](../../NOTICE.md).
- Whether a given use is defensible fair use is jurisdiction-specific. *This is
  flagged for counsel; not decided here.*

### Independent-creation method

This corpus follows the independent-creation method: one activity establishes
behavior from public, lawful observation; a separate activity implements from
the resulting specification. The method is a defense against **copyright
infringement** via independent creation, **not** against patents, and not
against trade-secret claims if protected information is used improperly.
([Wikipedia](#sources); [Finnegan](#sources); [Reed Smith](#sources); [Sedona
Conference](#sources).)

Rules this corpus commits to:

1. **No Adobe source code.** No reading, copying, summarizing, or paraphrasing
   Adobe source, whether leaked, decompiled, or obtained under an SDK or beta
   agreement, may influence these specs.
2. **No decompilation or disassembly.** Behavior is established from documented
   interfaces, public help material, published file formats, and black-box
   observation of the shipping product — not from binaries.
3. **No Adobe confidential information.** Contributors must not contribute
   knowledge obtained under NDA or through Adobe's plug-in/developer programs.
4. **Separate specification and implementation.** This repository is the
   specification artifact. Its specs may be reviewed by counsel for inadvertent
   inclusion of protected expression before implementation begins.
5. **No protected expression.** The PSD format spec and Adobe Help text are
   documents with their own copyright. Specs must express **facts and
   interfaces**, not reproduce documentation prose verbatim.

## UI surface

`None.` Legal posture has no UI. User-facing disclaimers are a distribution
concern, not a feature spec.

## Parameters & ranges

This section is adapted to hold the **asset and dependency policy**.

| Item | Policy | Rationale / confidence |
|---|---|---|
| Adobe brush tips, patterns, gradients, swatches, shapes, actions, workspaces | **Do not bundle.** Replace with independently authored defaults. | Adobe-created presets are creative works; some are functional but the imagery/patterns are not safe to copy. |
| Adobe ICC/color profiles shipped with Photoshop | **Do not redistribute Adobe-supplied profiles.** Use open/redistributable profiles or generate our own. | Profile files carry their own licensing terms; exact terms need counsel review. |
| Adobe fonts / Typekit-Adobe Fonts | **Do not bundle.** Use system fonts via a text stack. | Font software is separately licensed; Typekit is a service (non-goal, `OVR-003`). |
| Adobe plug-ins and Photoshop SDK headers/samples | **Do not use or link.** | SDK license restricts use/redistribution; `ARCH-011`. |
| Adobe Help screenshots / text | **Do not copy.** Cite and paraphrase facts. | Copyright in documentation. |
| Open-source dependencies (crates, libraries) | **Allowed after a license review** recorded in `THIRD-PARTY-LICENSES` (generated by `scripts/third-party-licenses.py`) and enforced by `deny.toml` in CI. | Must be compatible with the project's license (GPL-3.0-or-later). |
| Test-fixture PSD files created by CS6 | Allowed if lawfully owned, but must not be redistributed if they embed Adobe assets. | Copyright in user content vs. embedded presets is a counsel question. |
| Independent default presets | **Required** for anything shipped. | Independently authored assets. |
| Ported source code (e.g. PhotoCraft) | **Allowed** from a permissively licensed project, with attribution and its license text. | [PhotoCraft](https://github.com/storytold/photocraft) (MIT OR Apache-2.0, taken under MIT): `Filter > Liquify`, `Filter > Artistic > Palette Knife`, and `Edit > Transform > Warp` (warp styles and Bezier mesh, mesh resampler, Warp mode). The license text is in `LICENSES/photocraft.txt`, the attribution in `NOTICE.md`, and the Warp details in `docs/03-tools/move-and-transform.md`. |
| Third-party icon sets (e.g. Lucide) | **Allowed** with a permissive license and attribution. | Lucide is ISC (with an MIT subset from Feather); text in `LICENSES/Lucide.txt`, attribution in `NOTICE.md`, provenance in `docs/dev/icon-provenance.md`. |

Per-component license obligations (not exhaustive, **needs legal review**):

| Component (proposed) | Typical license | Obligation to check |
|---|---|---|
| Qt6 | LGPLv3 / GPLv3 / commercial | Dynamic linking + relinkability, notices, and modifications disclosure under LGPL. |
| Little CMS (lcms2) | MIT | Attribution. |
| FreeType | FTL or GPLv2 | Pick a compatible option; GPL is viral. |
| HarfBuzz | MIT | Attribution. |
| wgpu / Rust crates | MIT / Apache-2.0 (mostly) | Attribution, per-crate verification. |
| Exiv2 | GPLv2+ / commercial | GPL may be incompatible; verify before adoption. |
| LibRaw | LGPL-2.1 / CDDL-1.0 | Relinking obligation; verify. |

This table is a starting checklist, not a determination. Each crate/library must
be pinned to a version and its license confirmed before adoption.

## Algorithms & pipeline

The provenance **process**, as applied to this corpus:

```text
public sources + lawful black-box observation
        │
        ▼
[specification team]  ── writes behavior, not expression ──▶ SPEC (this repo)
        │                                                        │
        │ no Adobe source / no decompilation                     │ counsel review (proposed gate)
        ▼                                                        ▼
[implementation team] ── independent code from the SPEC only
```

Separation requirements (proposed, matching the cited independent-creation literature):

- The implementation team should not be the same individuals who studied any
  proprietary artifact, where feasible.
- The spec must not contain copied code, decompiled structure, or protected
  creative expression.
- Provenance of every behavior claim should be recorded (the `## Sources`
  section is the intended mechanism).
- If a claim cannot be sourced publicly, it is an open question, not a fact.

The method addresses copyright only. **Patents are not avoided by independent
creation** and must be handled separately (freedom-to-operate review).

## Rust module mapping

`None` as a design. The mapping constraint is that no Rust dependency may be
adopted without a recorded license check (the license inventory). This is a
process gate, not a module.

## Qt6 component mapping

`None`. The only Qt-specific obligation is the **LGPLv3/GPLv3/commercial**
licensing of Qt6 itself, which affects linking and distribution strategy (see
`01-architecture/build-and-packaging.md`). Static linking Qt generally requires
either GPL compliance or a commercial Qt license; dynamic linking under LGPL has
notices and relinking obligations. *Counsel review required before distribution.*

## Data-model impact

`None` directly. Two preservation rules already stated in `ARCH-002` also serve
the legal posture: unknown resources and unknown layer keys are kept verbatim so
that opening a file does not destroy content we did not author.

## Edge cases

- **Format patents.** The PSD/PSB container is publicly documented, but some
  *features expressed through it* (and many image-processing algorithms) may be
  patented. Public availability of the format does not grant patent rights.
  *Counsel question.*
- **Import/export format patents.** Some codecs and containers have historical
  patent exposure; verify per codec before shipping.
- **EULA anti-analysis clauses.** Even though these specs do not
  decompile, contributors must not have used tools that breach an Adobe EULA or
  a beta agreement. *Contributor attestation recommended.*
- **Trademark genericide.** Using the mark as a generic noun or verb is exactly
  what Adobe objects to; avoid it in user-facing copy.
- **Copyleft contamination.** Adopting a GPL library (e.g. a hypothetical GPL
  raw decoder) could force the project's license. Decide the project license
  before the dependency list is frozen.
- **AI-generated assets/specs.** Provenance and copyright status of generative
  output are unsettled; do not assume generated default assets are
  royalty-free without review.
- **Contributions from former Adobe employees or SDK users.** Even public
  contributions can import confidential knowledge. *Require disclosure.*

## Parity acceptance criteria

These are process gates, not pixel tests:

1. Given any spec, its `## Sources` cites only public, lawfully obtained
   material; no source is described as Adobe source, decompilation, or NDA
   material.
2. Given the repository, it contains no Adobe binaries, plug-ins, profiles,
   fonts, brush/pattern/gradient/swatch assets, or verbatim Adobe documentation.
3. Given any third-party dependency (Rust or Qt), a recorded license check
   exists and is compatible with the project's chosen license.
4. Given any user-facing distribution naming "Photoshop" or "Adobe," an
   affiliation disclaimer is present.
5. Given the implementation decision, no requirement depends on loading Adobe
   `.8bf` plug-ins or using the Adobe SDK.
6. Given a feature whose algorithm is closed, the spec says "behavioral parity
   only, algorithm TBD" and does not imply access to Adobe internals.

## Sources

- `https://en.wikipedia.org/wiki/Adobe_Photoshop` — Adobe develops Photoshop; Adobe discourages generic/verb use of the trademark; plug-in and Camera Raw model. Basis for trademark and ecosystem statements.
- `https://en.wikipedia.org/wiki/Clean-room_design` — independent-creation design definition; Phoenix BIOS example; separate specification and implementation; independent creation is a copyright defense but does not circumvent patent restrictions. Basis for the method and its limits.
- `(web search)` — commentaries on independent-creation design and its limits: it does not answer trade-secret or patent claims. Basis for the trade-secret and patent caveats.
- `https://web.archive.org/web/20231122064257/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/` — Adobe states the format document is "provided for 3rd parties to read and write the Photoshop native file format," and that the `PSDC` cloud format is private. Basis for the format-legality and cloud-exclusion statements.
- `https://www.adorama.com/alc/adobe-photoshop-cs6-creative-cloud-officially-launched/` — Adobe press release naming Photoshop, CS6, Creative Cloud, Mercury Graphics Engine (trademark/feature-name provenance).
- `https://prodesigntools.com/whats-the-difference-photoshop-cs6-vs-photoshop-cs6-extended.html` — edition and plug-in/trial context.

## Open questions

- **Is a nominative use of "Photoshop CS6" in the project's own materials
  defensible**, and is an explicit disclaimer sufficient? *Resolves with:*
  counsel opinion; record the answer as an ADR.
- **Which Adobe-shipped ICC profiles are licensed for redistribution**, if any?
  *Resolves with:* the profile EULAs / counsel; until then, do not redistribute.
- **Are any target algorithms subject to in-force patents**, and where?
  *Resolves with:* a freedom-to-operate review by patent counsel.
- **Project distribution license — resolved.** GPL-3.0-or-later; see
  [`11-cross-cutting/adr-project-license.md`](../11-cross-cutting/adr-project-license.md).
  All current dependencies are compatible (permissive Rust crates; Qt 6 under
  LGPLv3, which GPLv3 satisfies and which permits the current dynamic linking).
- **What contributor attestation is required** (no Adobe source, no NDA
  knowledge; CLA or none)? *Resolves with:* a `CONTRIBUTING` process reviewed by
  counsel.
- **Does shipping independently authored defaults that mimic Adobe defaults**
  (e.g. the default brush set) create any risk? *Resolves with:* counsel review
  of the default-asset plan.
