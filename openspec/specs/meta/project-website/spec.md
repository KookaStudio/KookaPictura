# meta/project-website Specification

## Purpose
Defines the repository's public landing site: a self-contained static bundle in
`site/` that presents the project, its capabilities, its downloads, and its
issue tracker, and is published to GitHub Pages by a workflow.

## Requirements

### Requirement: The landing site is a self-contained static bundle

The repository SHALL publish the project's landing page from a `site/`
directory of static files (HTML, CSS, images, fonts) that requires no build
step and no third-party runtime dependency. The published artifact SHALL contain
only files under `site/`, so the specification corpus in `docs/` is never
served as part of the site.

#### Scenario: The site root is a static index

- **WHEN** the published site root is requested
- **THEN** `site/index.html` is served and renders without executing a build or
  fetching a required script from a third-party host

#### Scenario: The spec corpus is not published

- **WHEN** the files uploaded to Pages are listed
- **THEN** none of them come from `docs/`

### Requirement: The site presents the project and its capabilities

The landing page SHALL describe what Kooka Pictura is, list its capabilities,
offer a way to download the installers, and offer a way to submit issues. The
capabilities presented SHALL reflect the features the project documents in
`README.md`.

#### Scenario: A visitor learns what the project is

- **WHEN** a visitor reads the page
- **THEN** they find a description of the editor, a capabilities section, a
  download section, and an issue section

### Requirement: The site features the kookaburra mascot and product imagery

The landing page SHALL feature the project's kookaburra mascot
(`assets/icons/app.png`) and the application screenshot
(`assets/screenshot01.jpg`).

#### Scenario: The mascot is visible

- **WHEN** the page renders
- **THEN** the kookaburra mascot appears, and the application screenshot is
  shown

### Requirement: Download and issue controls target the project's GitHub

Every download control SHALL link to the project's GitHub Releases page at
`https://github.com/KookaStudio/KookaPictura/releases`, and every issue control
SHALL link to its issue tracker at
`https://github.com/KookaStudio/KookaPictura/issues`.

#### Scenario: A visitor downloads the app

- **WHEN** a visitor activates a download control
- **THEN** they are taken to the project's GitHub Releases page

#### Scenario: A visitor reports a problem

- **WHEN** a visitor activates an issue control
- **THEN** they are taken to the project's GitHub issue tracker

### Requirement: The site links to the project's community chat

The landing page SHALL offer a link to the project's Discord community at
`https://discord.com/channels/1553843620501528786/1559643682897010741`, from the
community section and the footer.

#### Scenario: A visitor joins the chat

- **WHEN** a visitor activates a Discord link
- **THEN** they are taken to the project's Discord channel

### Requirement: Local assets are referenced relatively

The site SHALL reference its local assets with relative URLs, because GitHub
Pages serves a project site under the `/KookaPictura/` subpath and a
root-absolute reference would not resolve there.

#### Scenario: The site works under the project subpath

- **WHEN** the page is served from `https://kookastudio.github.io/KookaPictura/`
- **THEN** the stylesheet, images, icons, and fonts all load

### Requirement: GitHub Pages deployment is automated

The repository SHALL deploy `site/` to GitHub Pages through a GitHub Actions
workflow that runs on pushes to `master` that change `site/` and can be run by
hand. The workflow SHALL request the `pages: write` and `id-token: write`
permissions and SHALL serialize deployments with a concurrency group.

#### Scenario: A site change deploys

- **WHEN** a commit that changes a file under `site/` is pushed to `master`
- **THEN** the workflow uploads `site/` as the Pages artifact and deploys it

#### Scenario: The deploy is run by hand

- **WHEN** the Pages workflow is dispatched manually
- **THEN** it deploys the current `site/` to GitHub Pages

### Requirement: The site is responsive and readable

The landing page SHALL lay out cleanly across desktop and mobile viewports
without horizontal overflow, and its text SHALL meet a readable contrast ratio
against its background.

#### Scenario: The page is viewed on a narrow screen

- **WHEN** the page is rendered at a mobile viewport width
- **THEN** the columns stack, navigation collapses to the primary action, and
  no content overflows horizontally
