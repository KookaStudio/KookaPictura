## Signing order

After deploying Qt, write the `macos-dmg` package marker into the bundle before signing. The DMG step copies the signed bundle into its staging directory and packages it.

Use `create-dmg --app-drop-link` to add the Applications shortcut at the volume root. The pinned tool creates the shortcut independently of the Finder AppleScript, so it works on the unattended runner with `--skip-jenkins`.

## Artifact verification

Run strict, deep signature verification on the application inside the mounted, read-only DMG. This catches changes to sealed resources or nested code between bundle signing and distribution. Confirm the shortcut resolves to `/Applications` and keep the headless self-test to exercise the deployed runtime. The artifact upload step depends on the macOS job passing every check.
