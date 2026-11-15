# Spec Delta

## ADDED Requirements

### Requirement: The Flatpak build updates a stale image runtime on any hosted runner

The Flatpak build job SHALL update a runtime that the pinned container image
already carries whenever Flathub has a newer build of that runtime, and SHALL
do so successfully on every GitHub-hosted runner regardless of the host
kernel's overlay filesystem rename support. The Release-PR dry run SHALL pass
both with an empty action cache and with the restored action cache. Before the
build starts, the job log SHALL name the resolved `flatpak` and
`flatpak-builder` versions, the installed runtime refs and commits, and the
runner kernel.

#### Scenario: A stale image runtime updates on any runner [fp_runtime_update]

- **WHEN** Flathub has a newer `org.kde.Sdk//6.11` than the pinned container
  image carries and the Flatpak build job runs on an AWS- or Azure-hosted
  runner
- **THEN** `flatpak-builder` updates the runtime and retires the image's
  checkout without an invalid cross-device link error

#### Scenario: The build log names the environment [fp_versions_named]

- **WHEN** the Flatpak build job starts
- **THEN** its log names the `flatpak` and `flatpak-builder` versions, the
  installed runtime refs and commits, and the runner kernel

#### Scenario: The dry run passes with an empty and a restored cache [fp_dry_run_cache_paths]

- **WHEN** the release-please Release PR runs the Flatpak dry run once with no
  restored action cache and again with the restored action cache
- **THEN** both runs build the bundle and keep it as a workflow artifact, and
  neither fails at the runtime update
