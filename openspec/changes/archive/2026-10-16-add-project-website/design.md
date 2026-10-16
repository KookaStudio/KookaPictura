# Design

## Context

See `proposal.md` for motivation. The constraints that shape the approach:

- GitHub Pages is the host. Serving from `site/` (not `docs/`) keeps the spec
  corpus in `docs/` out of the published artifact, which the issue calls out
  explicitly.
- A project Pages site is served under `https://<org>.github.io/<repo>/`, so
  every local reference must be relative (`./style.css`, `./assets/…`); a
  root-absolute path would 404 under the subpath.
- The published artifact contains only what is in `site/`, so the mascot,
  screenshot, icons, and fonts are copied in rather than referenced from
  `assets/` outside the folder.
- The project already owns the visual ingredients: a kookaburra mascot
  (`assets/icons/app.png`, a dark-navy circular badge), a workspace screenshot
  (`assets/screenshot01.jpg`), a 195-icon set of `currentColor` line glyphs, and
  the Noto Sans faces the app bundles. The site reuses them, so it stays in the
  project's own visual language with no new artwork.
- The app's design system is a dark, Noto Sans UI with a blue accent; the
  mascot's palette is warm cream, umber/rust, and deep navy. The landing page
  uses the mascot's warm palette on a canvas-textured background for a
  painterly, gallery feel while staying legible.
- The icon SVGs use `stroke="currentColor"`; rendering them through CSS
  `mask-image` lets the page tint them with the accent colour without editing
  the shared assets.

## Goals / Non-Goals

**Goals:**
- A self-contained static site in `site/` that introduces the project, its
  capabilities, its downloads, and its issue tracker.
- Feature the kookaburra mascot and the application screenshot.
- Deploy automatically to GitHub Pages with the official Actions.
- Readable, responsive, and accessible on desktop and mobile.

**Non-Goals:**
- A documentation site, a blog, or a CMS; the spec corpus stays in `docs/` and
  is not published.
- A custom domain or analytics.
- Any change to the application, its build, or the release pipeline.

## Decisions

### Static files in `site/`, no build step

A single `index.html` plus one `style.css` and a folder of copied assets is
enough for a landing page, keeps the deploy workflow trivial, and avoids adding
a Node toolchain or a generator dependency to a Rust/Qt repository. It matches
the approach the issue suggests.

### Relative asset URLs

Because Pages serves the site under `/KookaPictura/`, all local references are
relative. Absolute Open Graph URLs are pinned to
`https://kookastudio.github.io/KookaPictura/` since social scrapers need
absolute URLs.

### Reuse the project's own assets

The mascot, screenshot, a curated subset of the UI icons, and the bundled Noto
Sans fonts are copied into `site/` so the published artifact is self-contained.
The icons are tinted through CSS masks, so the shared source SVGs are used
unmodified. The same assets and licenses already recorded in
`assets/PROVENANCE.md` apply; no new asset or license is introduced.

### Deploy with the official Pages actions

`.github/workflows/pages.yml` uses `actions/configure-pages`,
`actions/upload-pages-artifact` (path `site`), and `actions/deploy-pages`, with
`pages: write` / `id-token: write` permissions and a `pages` concurrency group.
It triggers on pushes to `master` touching `site/` (or the workflow itself) and
on manual dispatch. Enabling "Source: GitHub Actions" in the repository settings
is a one-time manual step noted in the workflow header.

## Risks / Trade-offs

- **Font weight.** Four Noto Sans faces add ~2.2 MB to the artifact. Acceptable
  for a static page that wants to match the app's typography offline; a system
  font stack was the lighter alternative but loses the app's look.
- **Unsigned/missing platform installers.** The download cards link to the
  releases page rather than to version-pinned asset URLs, so they stay correct
  as the version advances and do not 404 when a given platform has no asset yet.
