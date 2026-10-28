## Decisions

- **Release lookup.** The browser calls the unauthenticated GitHub API (`releases/latest`) and classifies assets by file suffix, because asset names embed the version. The response is cached in `sessionStorage` to stay under the rate limit. On any failure the control falls back to a link to `/releases/latest`.
- **Docs.** `pages.yml` copies `docs/` (minus `docs/dev/`) to `site/docs-src` before upload, so the site always matches `master`. `docs.html` fetches the Markdown and renders it with vendored `marked`; relative links and images are rewritten to stay inside the viewer. `docs/INDEX.md` is the navigation page.
- **Rejected:** pre-rendering in CI (adds a build step); fetching from raw.githubusercontent (rate limits, broken relative links).
