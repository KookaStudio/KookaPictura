# Tasks

## 1. Test plan

- [x] 1.1 Write `test-plan.md` with one row for each scenario in the `meta/flatpak-release-build` delta; verify `python3 scripts/check-test-plan.py` reports no missing rows.

## 2. Documentation refresh

- [x] 2.1 Check off the installers item in `ROADMAP.md` and name the shipped set; verify `grep -n "Installers" ROADMAP.md` shows `[x]` with AppImage, `.deb`/`.rpm`, snap, Flatpak, Windows portable/installer, and macOS DMG.
- [x] 2.2 Add the #703/#710 entry to `docs/dev/STATE.md`; verify it names the six hardened packagers, the archived `release-build-hardening` change, and the reverted dry-run widening.
- [x] 2.3 Mark `docs/dev/flatpak-offline-memo.md` superseded; verify the status line points at `flatpak/io.github.KookaStudio.KookaPictura.yml` and the packaging sections of `DEVELOPING.md`.

## 3. Validation and archive

- [x] 3.1 Verify `openspec validate --all --strict` passes with the delta applied.
- [x] 3.2 Archive the change with `openspec archive release-packaging-docs`, then verify `bash scripts/check-openspec-archived.sh` and `bash scripts/guard.sh` pass.
