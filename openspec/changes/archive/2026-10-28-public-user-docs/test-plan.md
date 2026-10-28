# Test Plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| The site root is a static index | Serve `site/` and open the root. | Page renders without a build step. | manual: static serving check in a browser |
| The spec corpus is not published | Run the workflow copy step locally and list `site/docs-src`. | Only `public_docs/` files are present. | manual: workflow shell step, checked locally |
| The docs corpus is published as Markdown | Same listing. | `README.md` and the guide pages are present. | manual: workflow shell step, checked locally |
| A site change deploys | Push a site change to master. | Pages workflow runs. | manual: needs CI |
| The deploy is run by hand | Dispatch the workflow. | Site deploys. | manual: needs CI |
| A docs change deploys | Push a `public_docs/` change to master. | Pages workflow runs and the page updates. | manual: needs CI |
| A visitor reads a doc page | Open Documentation, pick a page from Contents, follow a link inside it. | The page renders in place; the sidebar marks the current page. | manual: needs a browser |
