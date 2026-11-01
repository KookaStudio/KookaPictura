# Proposal

## Why

The release `.snap` is unsigned because it is not published to the Snap Store, so
snapd refuses it with "cannot find signatures with metadata" unless `--dangerous`
is passed (#1106). Only `DEVELOPING.md` says so; the places users get the snap
from do not.

## What Changes

- `README.md` and `public_docs/installing.md` give the install command.
- The site's Download panel shows the command, with the real filename, beside the
  snap link.
- The snap workflow's upload job adds the command to the release notes, once per
  release.
- `DEVELOPING.md` records where the command appears.

Snap Store publishing stays out of scope.

## Capabilities

### Modified Capabilities

- `meta/snap-release-build`: adds a requirement that the install command reaches
  users.

## Impact

Documentation, `site/download.js`, `site/style.css`, and
`.github/workflows/snap.yml`. No application code changes.
