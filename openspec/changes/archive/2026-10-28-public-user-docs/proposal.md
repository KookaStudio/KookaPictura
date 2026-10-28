## Why

The site rendered `docs/`, which is the internal specification corpus and unsuitable for casual users.

## What Changes

- Add `public_docs/`, a short user guide: what Kooka Pictura is, installing, the interface, layers, blend modes and styles, adjustments, filters, selections and masking, and paths.
- The site renders `public_docs/` instead of `docs/`; the Pages workflow copies it.

## Capabilities

### Modified Capabilities

- `meta/project-website`: the published documentation is `public_docs/`, not `docs/`.

## Impact

`public_docs/`, `site/docs.js`, `site/docs.html`, `site/index.html`, `.github/workflows/pages.yml`.
