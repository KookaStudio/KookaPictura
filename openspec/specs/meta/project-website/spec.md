# meta/project-website Specification

## Purpose
Defines the repository's public landing site: a self-contained static bundle in
`site/` that presents the project, its capabilities, its downloads, and its
issue tracker, and is published to GitHub Pages by a workflow.

## Requirements

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

Every download fallback link SHALL point to the project's GitHub Releases at
`https://github.com/KookaStudio/KookaPictura/releases/latest`, and every issue
control SHALL link to its issue tracker at
`https://github.com/KookaStudio/KookaPictura/issues`.

#### Scenario: A visitor downloads the app

- **WHEN** a visitor activates a download fallback link
- **THEN** they are taken to the project's latest GitHub release page

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

### Requirement: The site is responsive and readable

The landing page SHALL lay out cleanly across desktop and mobile viewports
without horizontal overflow, and its text SHALL meet a readable contrast ratio
against its background.

#### Scenario: The page is viewed on a narrow screen

- **WHEN** the page is rendered at a mobile viewport width
- **THEN** the columns stack, navigation collapses to the primary action, and
  no content overflows horizontally

### Requirement: Download controls resolve the latest release

The landing page SHALL present one Download control that reveals Linux, macOS
and Windows choices. Each choice SHALL list direct download links for the
installers attached to the latest GitHub release of
`https://github.com/KookaStudio/KookaPictura`, matched by file suffix: `.rpm`,
`.deb`, `.AppImage`, `.snap` and `.flatpak` for Linux, `.dmg` for macOS and
`-setup.exe` for Windows. The macOS and Windows choices SHALL show installation
notes beneath the link. When the release cannot be read, the control SHALL link
to `https://github.com/KookaStudio/KookaPictura/releases/latest`.

#### Scenario: A visitor picks Linux

- **WHEN** a visitor activates Download and then Linux
- **THEN** one link per available Linux format is shown, each pointing to the
  asset of the latest release

#### Scenario: A visitor picks macOS or Windows

- **WHEN** a visitor activates macOS or Windows
- **THEN** the installer link is shown with its installation notes beneath it

#### Scenario: The release lookup fails

- **WHEN** the GitHub API request fails
- **THEN** the control offers a link to the latest release page

### Requirement: The site renders the documentation

The site SHALL provide a Documentation page that renders the Markdown copy of
`public_docs/` in the browser, resolving relative links and images between pages.

#### Scenario: A visitor reads a doc page

- **WHEN** a visitor opens the Documentation page and follows a link to another
  doc
- **THEN** the linked page is rendered in place

### Requirement: The nav GitHub button is legible

The navigation GitHub button SHALL render its label in white.

#### Scenario: The nav button label

- **WHEN** the page renders
- **THEN** the GitHub button label is white on its dark background, including on hover
