# Kooka Pictura — Photoshop CS6 parity specification

The behavioural contract for **Adobe Photoshop CS6 (v13, 2012)**, which this
repository reimplements as a native Linux application using **Rust** (core) and
**Qt6** (UI), targeting behavioral parity with CS6.

> The implementation lives in `crates/`. Every `## Rust module mapping` / `## Qt6
> component mapping` section is a design proposal rather than a description of
> the shipped code, and `docs/dev/doc-code-parity-gaps.md` records what shipped
> and what did not.

## Scope

- **Parity target:** Photoshop CS6, Standard edition, plus the Extended-only
  features the corpus marks and the project has chosen to build (the Count
  tool). Edition gating is not implemented (#650).
- **Platform:** Linux (native), X11 + Wayland.
- **Stack:** Rust core, Qt6 UI (Widgets and/or QML), GPU compositing.
- **Out of scope for parity:** OS-specific integration with no Linux equivalent
  (e.g. some print paths), 3D and video where the cost is prohibitive, and Adobe's
  proprietary cloud services. Each exclusion is documented with rationale rather
  than silently dropped.

## How to read this

1. [`00-overview/`](00-overview/) — what we are building and why, feasibility, non-goals, legal.
2. [`01-architecture/`](01-architecture/) — the constraints that shape everything else. Read first.
3. [`02-ui-ux/`](02-ui-ux/) — application frame, panels, preferences, shortcuts.
4. `03-`…`10-` — the feature domains (tools, image ops, layers, filters, color, selection, automation, workflow).
5. [`11-cross-cutting/`](11-cross-cutting/) — concerns that cut across domains.
6. [`TRACEABILITY.md`](TRACEABILITY.md) — CS6 feature → spec → Rust module → Qt component.
7. [`SPEC_TEMPLATE.md`](SPEC_TEMPLATE.md) — the canonical structure every spec follows.
8. [`GLOSSARY.md`](GLOSSARY.md) — terms and CS6-specific vocabulary.
9. [`dev/doc-code-parity-gaps.md`](dev/doc-code-parity-gaps.md) — what the corpus
   specifies but the code does not yet implement, and the issue tracking each gap.

## Status legend

| Status | Meaning |
|---|---|
| Stub | Heading exists, content not yet written. |
| Draft | Written from sources but not cross-reviewed. |
| Spec'd | Complete, sourced, cross-reviewed against the template. |
| Verified | Sources re-checked; parity criteria executable in principle. |

## Source policy

- Primary: Adobe's official **Photoshop CS6 Help reference PDF**
  (`help.adobe.com/archive/en/photoshop/cs6/photoshop_reference.pdf`), Adobe
  file-format and SDK documentation.
- Secondary: reputable books/tutorials, standards (ICC, TIFF, PNG, JPEG), and
  the Qt6 / Rust ecosystem docs for the technology mapping.
- Unverified claims live under `## Open questions`, never asserted as fact.
- `helpx.adobe.com` returns HTTP 403; use the archived PDF or `support` mirrors.

## Index

See [`INDEX.md`](INDEX.md) for the full file-by-file listing and current status.

## Legal

Adobe, Photoshop, and related marks are trademarks of Adobe Inc. This is a
documentation-first behavioral specification produced without Adobe source code, binaries,
or assets. See [`00-overview/licensing-and-provenance.md`](00-overview/licensing-and-provenance.md).
