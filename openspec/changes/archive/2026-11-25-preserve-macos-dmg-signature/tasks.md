## 1. Fix macOS packaging

- [x] Write the package marker before the shared bundle action signs the app.
- [x] Verify the signed bundle from the mounted DMG.
- [x] Add a mounted-DMG check for the Applications shortcut.

## 2. Verify the fix

- [x] Check that strict verification rejects a post-signing resource addition.
- [ ] Run the macOS DMG build on the fix PR and restore the permanent trigger.
