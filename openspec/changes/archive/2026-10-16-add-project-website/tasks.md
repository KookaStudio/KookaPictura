# Tasks

## 1. Scaffold the static site

- [x] 1.1 Create `site/` with `assets/`, `icons/`, and `fonts/` subfolders.
- [x] 1.2 Copy the mascot (`assets/icons/app.png`, also used as the favicon),
  the screenshot (`assets/screenshot01.jpg`), a curated subset of the
  `currentColor` UI icons, and the four bundled Noto Sans faces into `site/`, so
  the published artifact is self-contained.
- [x] 1.3 Confirm every referenced local file resolves (no missing assets).

## 2. Build the landing page

- [x] 2.1 Write `site/style.css`: warm canvas palette drawn from the mascot,
  bundled `@font-face` rules, a canvas grain overlay, serif display headings,
  buttons, the hero/medallion/framed-screenshot composition, capability and
  platform cards, the showcase band, the footer, and responsive and
  reduced-motion rules.
- [x] 2.2 Write `site/index.html`: header nav, hero (mascot medallion +
  screenshot), "the project" section with stats and a pull quote, a capabilities
  grid, a showcase of the editor, platform download cards, an open-source/issue
  section, and a footer with project and developer links.
- [x] 2.3 Feature the kookaburra mascot in the header, hero, and footer, and the
  screenshot in both the hero and the showcase.
- [x] 2.4 Link every download control to
  `https://github.com/KookaStudio/KookaPictura/releases` and the issue controls
  to `https://github.com/KookaStudio/KookaPictura/issues`.
- [x] 2.5 Reference all local assets relatively (`./…`) and pin absolute Open
  Graph URLs to the Pages origin; add image `width`/`height` and lazy-load the
  below-the-fold screenshot.
- [x] 2.6 Link the community section and the footer to the project's Discord
  channel.

## 3. Add the Pages deploy workflow

- [x] 3.1 Create `.github/workflows/pages.yml` triggering on `master` pushes
  under `site/**` (and the workflow file) plus `workflow_dispatch`, with
  `pages: write` / `id-token: write` permissions, a `pages` concurrency group,
  and the `github-pages` environment.
- [x] 3.2 Deploy with `actions/configure-pages`, `actions/upload-pages-artifact`
  (path `site`), and `actions/deploy-pages`; note the one-time "Source: GitHub
  Actions" setting in the workflow header.

## 4. Verification

- [x] 4.1 Serve `site/` locally and confirm `index.html`, `style.css`, the
  images, icons, and fonts all return 200.
- [x] 4.2 Render the page headless at desktop and mobile widths and review the
  full layout for overflow, contrast, and asset loading.
- [x] 4.3 Validate the HTML parses with balanced tags and the workflow YAML
  loads.
- [ ] 4.4 Enable Pages (Settings → Pages → Source: GitHub Actions) and confirm
  the first deploy publishes the site at
  `https://kookastudio.github.io/KookaPictura/`.
