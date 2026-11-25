## Why

The v0.4.0 macOS DMG contains an app that Finder reports as damaged. The packaging step added a resource after the bundle was signed, which invalidated the resource seal.
The disk image also omits the Applications shortcut used for drag-to-install.

## What Changes

- Write the package marker before applying the ad-hoc signature.
- Verify the app signature from the mounted DMG before keeping or publishing the artifact.
- Include and verify the Applications shortcut beside the app in the DMG.

## Capabilities

### Modified Capabilities

- `meta/macos-release-build` requires an intact ad-hoc signature and a verified drag-to-install shortcut.

## Impact

The shared macOS bundle action and the DMG workflow change. The same verification covers the Release PR dry run and the release build.
