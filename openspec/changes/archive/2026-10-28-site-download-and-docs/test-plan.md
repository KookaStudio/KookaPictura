# Test Plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| The site root is a static index | Open `site/index.html` served statically. | Page renders; no third-party script host is required. | manual: static serving check in a browser |
| The spec corpus is not published | List the Pages artifact. | Only the docs-src copy, no dev/. | manual: workflow shell step, checked locally |
| A visitor downloads the app | Use the fallback link. | Opens releases/latest. | manual: unchanged link |
| A site change deploys | Push a site change to master. | Pages workflow runs. | manual: needs CI |
| The deploy is run by hand | Dispatch the workflow. | Site deploys. | manual: needs CI |
| The docs corpus is published as Markdown | Run the workflow copy step locally and list `site/docs-src`. | `docs/` files are present, `dev/` is absent. | manual: workflow shell step, checked locally |
| A visitor picks Linux | Click Download, then Linux. | Links for rpm, deb, AppImage, snap, flatpak point at the release assets. | manual: logic covered by `node --test scripts/site-download.test.mjs` (the plan checker only resolves Rust and Qt refs); reveal UI checked by hand |
| A visitor picks macOS or Windows | Click macOS, then Windows. | dmg / setup exe link with the install notes beneath. | manual: logic covered by `node --test scripts/site-download.test.mjs` (the plan checker only resolves Rust and Qt refs); reveal UI checked by hand |
| The release lookup fails | Block api.github.com and reload. | A "Download from GitHub" link to releases/latest appears. | manual: fallback logic covered by `node --test scripts/site-download.test.mjs`; blocked-API UI checked by hand |
| A visitor reads a doc page | Open docs.html, follow a link. | Linked page renders in place. | manual: needs a browser |
| A visitor reports a problem | Click an issue control. | Goes to the issues page. | manual: unchanged link |
| The nav button label | Load the page; hover the button. | White label, also on hover. | manual: visual check |
| A docs change deploys | Push a docs change to master. | Pages workflow runs. | manual: needs CI |
