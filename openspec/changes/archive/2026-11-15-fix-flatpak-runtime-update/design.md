# Design

## Context

See `proposal.md` - Why. The constraints that shape the approach:

- `.github/workflows/flatpak.yml` runs the build in
  `ghcr.io/flathub-infra/flatpak-github-actions@sha256:b0a8a2…`, which is the
  current `kde-6.11` tag, with `options: --privileged`.
- The pinned action is `flatpak/flatpak-github-actions/flatpak-builder@7932741…`
  (`v6.8`, the newest tag). It always passes `--install-deps-from=flathub` and
  has no input that skips runtime updates.
- The image carries `org.kde.Platform`/`Sdk` 6.11 in its own layers, but
  Flathub has newer builds, so every run updates them.
- flatpak retires the replaced checkout by renaming its directory into
  `.removed/`. A directory that comes from an image layer can only be renamed
  by overlayfs when the host kernel has `redirect_dir` enabled; otherwise the
  rename returns `EXDEV` ("Invalid cross-device link").
- The 13 most recent runs split cleanly: 6 failures, all on `Cloud: AWS`; 7
  passes, all on `Cloud: Azure`. A sibling repository hit the identical
  signature (9/9 AWS failures, 49/49 Azure passes) and proved in a container
  probe that a directory created inside the container renames on both kernels
  while an image-layer directory does not.

## Goals / Non-Goals

**Goals:**

- Make the required Release-PR dry run deterministic instead of dependent on
  which runner fleet GitHub assigns.
- Keep the pinned image, action, manifest, action cache, and build steps.
- Make the job log sufficient to identify the tools, runtimes, and kernel when
  the update fails again for a different reason.

**Non-Goals:**

- Refreshing the image digest or moving the runtime branch.
- Replacing the action to skip the runtime update.
- Changing the application, manifest, or documentation.

## Decisions

**Mount an anonymous volume at `/var/lib/flatpak`.** Docker fills a new volume
with the image's content at that path when the container is created, so the
Flatpak installation sits on the runner's own ext4 filesystem and every checkout
is an upper-layer directory. The retirement rename then works with either
kernel, and the runtime sees the same pre-installed runtimes as before.

Alternatives considered:

- **Retry the job.** Rejected. The failed update has already switched to the new
  runtime before the rename fails, so a retry finds nothing to update and passes
  by accident. The old checkout stays behind and the cause remains.
- **Pre-run `flatpak update || true`.** Rejected for the same reason: it masks
  the `EXDEV` and leaves the retired checkout.
- **Skip runtime updates.** The action has no such input, and skipping would
  freeze the build to the image's runtime and its security fixes.
- **Bump the action or image.** Rejected: both are current, and upstream
  (`flatpak/flatpak-github-actions#267`) has no fix. Refreshing the image only
  postpones the failure until Flathub publishes the next runtime.
- **Build outside the container.** Rejected: it is a larger change and loses
  the pinned build image.

**One pre-build diagnostic step.** The step prints `flatpak --version`,
`flatpak-builder --version`, `flatpak list --system --runtime
--columns=ref,active` (the ref and its deployed commit), and `uname -r`. It
runs before the build step so
the versions are present even when the runtime update fails; a post-build step
would be skipped exactly when it is needed.

**Annotate the digest with `# kde-6.11`.** The repository already pins actions
with a human-readable ref comment; naming the image tag keeps the reviewed
digest identifiable.

**Verify on the 0.4.0 release path.** The workflow change merges to `master`
before the release, so release-please refreshes the Release PR and its Flatpak
check runs with the fix; the 0.4.0 tag build uses the same workflow. A dispatch
against `v0.3.0` is avoided because the upload job would rewrite that release's
existing asset.

## Risks / Trade-offs

- [The volume seed copies the image's Flatpak installation (about 3 GB) at
  container start] → the sibling repository measured 17-21 s of extra startup
  and a faster update (28-78 s down to 8-17 s), so the job is not slower.
- [The volume is anonymous and per job, so an updated runtime is not reused
  between runs] → the update already happened on every run before this change;
  this preserves current behavior.
- [The AWS/Azure split may not be permanent] → the fix does not depend on the
  cloud or kernel; either filesystem layout renames successfully.
- [The action cache key is derived from the manifest and does not change] →
  the restored-cache run is free, while the clean-cache run needs a manual
  `gh cache delete` before a rerun.

## Migration Plan

- Merge the workflow change to `master`; release-please updates the 0.4.0
  Release PR, whose checks rerun with the new workflow.
- Rollback is a revert of the workflow commit. It restores the intermittent
  failure but changes no stored state.
- The 0.4.0 tag build is the first tagged build on the fixed workflow; confirm
  it on the release.
