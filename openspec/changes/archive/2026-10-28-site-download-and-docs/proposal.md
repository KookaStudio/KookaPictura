## Why

Issue #797: the landing site sends every visitor to the GitHub release page, shows an unreadable GitHub label in the nav, and has no user documentation.

## What Changes

- The nav GitHub button label is white.
- One `Download` control reveals Linux, macOS and Windows; each resolves the installers of the latest GitHub release at view time and links them directly. macOS and Windows show install notes under the link.
- A Documentation page renders the public `docs/` corpus, copied into the Pages artifact at deploy time and rendered in the browser with a vendored markdown library.

## Capabilities

### Modified Capabilities

- `meta/project-website`: download controls resolve the latest release; the artifact may carry a copy of `docs/`; a vendored markdown renderer is the one allowed runtime dependency.

## Impact

`site/`, `.github/workflows/pages.yml`, `scripts/site-download.test.mjs`. New vendored dependency: `marked` (MIT).
