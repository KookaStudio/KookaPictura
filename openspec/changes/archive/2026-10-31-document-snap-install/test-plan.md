# Test Plan

Covers every scenario in the `meta/snap-release-build` delta. These are docs, site
script, and workflow behaviors, which `scripts/check-test-plan.py` cannot name:
its coverage tokens are `tst_<suite>::<case>` and `<crate>::<fn>`. Every row
therefore reads `manual:`, naming the check that applies.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `A user reads the install documentation` | Read "Getting it" in `README.md` and the Snap section of `public_docs/installing.md` | Both give `sudo snap install --dangerous ./kooka-pictura_<version>_amd64.snap` | manual: docs review |
| `The Download panel lists a snap` | Run `node --test scripts/site-download.test.mjs`, then open the site's Linux download choices | The snap entry alone has the command, with the asset's filename | manual: node test `scripts/site-download.test.mjs`, which the guard cannot reference, plus a look at the rendered panel |
| `The workflow runs for a tag` | `gh workflow run snap.yml -f tag=vX.Y.Z` on a release without the note, or wait for the next release | The release notes gain the "Installing the snap" section | manual: needs a real GitHub Release, so it is checked on the next release |
| `The workflow is re-run for the same tag` | Re-run the workflow for that tag | The release notes keep exactly one "Installing the snap" section | manual: needs a real GitHub Release, so it is checked on the next release |
| `The documented command installs the release snap` | On Fedora 43, `sudo snap install --dangerous ./kooka-pictura_0.3.0_amd64.snap`, then `kooka-pictura.pictura --headless --self-test` | The install succeeds and the self-test prints `failed=0`; run once for #1106 with `passed=471 failed=0 skipped=0` | manual: needs a real snapd host; Ubuntu is also covered by the snap workflow's smoke check |
