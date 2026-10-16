# Proposal

## Why

Kooka Pictura ships installers (AppImage, `.deb`, `.rpm`, Windows setup/portable,
macOS DMG) but has no public landing page: a visitor arriving from a repository
link sees the README and has to infer what the project is and where to download
it. A dedicated site gives artists a clear, well-presented entry point and a
single place to reach the downloads and the issue tracker.

## What Changes

- Add a self-contained static landing site under `site/` — `index.html`,
  `style.css`, the mascot and screenshot, a small set of the project's own UI
  icons, and the bundled Noto Sans fonts. No build step and no third-party
  runtime dependencies.
- The page presents Kooka Pictura in a painterly, gallery-style layout: a hero
  built around the kookaburra mascot and the application screenshot, a "what it
  is" section, a capabilities grid drawn from the README feature list, a
  showcase of the editor, platform download cards, and an open-source/issue
  section, plus a footer with project and developer links.
- Link every download control to the GitHub Releases page and every issue
  control to the issue tracker of `KookaStudio/KookaPictura`.
- Reference all local assets with relative URLs (`./…`) so the site works when
  Pages serves it from the `/KookaPictura/` subpath.
- Add `.github/workflows/pages.yml` that publishes `site/` to GitHub Pages on
  pushes to `master` under `site/` and on manual dispatch, using the official
  Pages actions.
- No BREAKING changes; no application code, build, or release behavior changes.

## Capabilities

### New Capabilities

- `meta/project-website`: the repository serves a static public landing site
  from `site/` that presents the project, its capabilities, its downloads, and
  its issue tracker, features the kookaburra mascot, and is deployed to GitHub
  Pages by a workflow.

### Modified Capabilities

<!-- None: this is an additive repository/publishing surface with no existing
     behavior spec to change. -->

## Impact

- **Site**: new `site/` directory (HTML, CSS, images, icons, fonts). Static and
  self-contained; nothing under `crates/` or the C++ shell changes.
- **CI**: new `.github/workflows/pages.yml` (official `configure-pages`,
  `upload-pages-artifact`, `deploy-pages` actions).
- **Assets**: reuses the already-tracked mascot (`assets/icons/app.png`),
  screenshot (`assets/screenshot01.jpg`), UI icons, and Noto Sans fonts; copies
  them into `site/` so the published artifact is self-contained. Provenance is
  unchanged (same CC0-1.0 / MIT and bundled-font terms recorded in
  `assets/PROVENANCE.md`).
- **Settings**: publishing requires enabling Pages with "Source: GitHub
  Actions" once in the repository settings (documented in the workflow header).
- **Dependencies**: none added.
