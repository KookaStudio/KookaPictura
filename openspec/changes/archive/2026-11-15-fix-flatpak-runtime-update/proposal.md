# Proposal

## Why

The Flatpak release dry run fails intermittently while updating
`org.kde.Sdk//6.11`:

```
Error: Failed to update org.kde.Sdk: Invalid cross-device link
```

In the 13 most recent runs of `.github/workflows/flatpak.yml`, every failure
(6) ran on an AWS-hosted runner and every pass (7) ran on an Azure-hosted
runner. The pinned action (`v6.8`) and container digest (the current `kde-6.11`
tag) are both the newest available, and upstream (`flatpak/flatpak-github-actions#267`)
has no fix, so bumping either is not a path forward. The failure blocks the
0.4.0 release PR's required Flatpak check.

The runtime the image carries is older than Flathub's, so `flatpak-builder`
updates it on every run. flatpak retires the old checkout by renaming its
directory, and that directory is an overlayfs lower layer of the container
image. The rename needs the host kernel's overlayfs `redirect_dir` support,
which the AWS runner fleet does not have, and the update fails with `EXDEV`.
The `xvfb-run` segfault (exit 139) is the wrapper tearing down after
`flatpak-builder` already failed.

## What Changes

- Put the container's `/var/lib/flatpak` installation on a Docker volume, so
  the runtime checkouts live on the runner's own filesystem and the retirement
  rename succeeds on every hosted runner.
- Name the resolved `flatpak` and `flatpak-builder` versions, the installed
  runtime refs and commits, and the runner kernel in the job log.
- Annotate the pinned container digest with its `kde-6.11` tag.
- No manifest, app, or documentation changes.

## Capabilities

### New Capabilities

<!-- None. -->

### Modified Capabilities

- `meta/flatpak-release-build`: adds a requirement that the build succeeds
  regardless of the runner's overlay filesystem and that the job log names the
  build environment (tool versions, runtime commits, kernel).

## Impact

- **CI**: `.github/workflows/flatpak.yml` only. The change applies to the
  Release-PR dry run, the release-tag build, and manual dispatch alike.
- **Runtime**: the anonymous volume is seeded from the pinned image per job
  (about 3 GB, tens of seconds at container start), offset by a faster runtime
  update.
- **Verification**: the Release-PR dry run with a clean and a restored action
  cache, then the 0.4.0 release tag build; the 0.4.0 release is the first
  tagged build to use the fixed workflow.
- **Out of scope**: refreshing the image digest, changing the runtime branch,
  or replacing the `flatpak-builder` action.
