## 1. Fix macOS packaging

- [x] Write the package marker before the shared bundle action signs the app.
- [x] Verify the signed bundle from the mounted DMG.
- [x] Add a mounted-DMG check for the Applications shortcut.

## 2. Verify the fix

- [x] Check that strict verification rejects a post-signing resource addition.
- [x] Verify signing, the Applications shortcut, and the headless self-test from a locally mounted DMG.
- [x] Restore the release-only macOS workflow trigger.
