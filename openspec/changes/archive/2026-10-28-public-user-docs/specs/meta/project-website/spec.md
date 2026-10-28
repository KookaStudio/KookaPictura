## MODIFIED Requirements

### Requirement: The landing site is a self-contained static bundle

The repository SHALL publish the project's landing page from a `site/`
directory of static files (HTML, CSS, JavaScript, images, fonts) that requires
no build step. The only third-party runtime dependency SHALL be a vendored
Markdown renderer stored under `site/vendor/`. The published artifact SHALL
contain the files under `site/` and a copy of the user guide in
`public_docs/` placed under `site/docs-src` by the deploy workflow.

#### Scenario: The site root is a static index

- **WHEN** the published site root is requested
- **THEN** `site/index.html` is served and renders without executing a build or
  fetching a required script from a third-party host

#### Scenario: The spec corpus is not published

- **WHEN** the files uploaded to Pages are listed
- **THEN** the only documentation files are the Markdown copy of `public_docs/`
  under `docs-src`; nothing from `docs/` is among them

#### Scenario: The docs corpus is published as Markdown

- **WHEN** the files uploaded to Pages are listed
- **THEN** the `public_docs/` files appear under `docs-src`

### Requirement: GitHub Pages deployment is automated

The repository SHALL deploy the site to GitHub Pages through a GitHub Actions
workflow that runs on pushes to `master` that change `site/` or `public_docs/` and can
be run by hand. The workflow SHALL request the `pages: write` and
`id-token: write` permissions and SHALL serialize deployments with a
concurrency group.

#### Scenario: A site change deploys

- **WHEN** a commit that changes a file under `site/` is pushed to `master`
- **THEN** the workflow uploads the site as the Pages artifact and deploys it

#### Scenario: The deploy is run by hand

- **WHEN** the Pages workflow is dispatched manually
- **THEN** it deploys the current `site/` to GitHub Pages

#### Scenario: A docs change deploys

- **WHEN** a commit that changes a file under `public_docs/` is pushed to `master`
- **THEN** the workflow redeploys the site with the updated user guide

### Requirement: The site renders the documentation

The site SHALL provide a Documentation page that renders the Markdown copy of
`public_docs/` in the browser, resolving relative links and images between pages.

#### Scenario: A visitor reads a doc page

- **WHEN** a visitor opens the Documentation page and follows a link to another
  doc
- **THEN** the linked page is rendered in place
