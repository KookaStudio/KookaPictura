# Update and Versioning

- **Spec ID:** `XC-011`
- **Status:** `Draft`
- **Parity tier:** `Core` for document/format/ABI/settings versioning;
  `Extended-only` for update delivery (CS6's installer/updater has no Linux
  equivalent and is re-specified here).
- **New in CS6:** `Changed` — CS6 is application version **13.0** (13.0.1
  maintenance update). PSD/PSB file-version fields and the Version Info image
  resource are unchanged from earlier releases; CS6 adds the Auto Save image
  resources (see `XC-012`). Update delivery on Windows/macOS moved to Adobe
  Application Manager / Creative Cloud, outside the application binary.
- **Depends on:** `ARCH-004` build-and-packaging, `ARCH-007` file-formats,
  `ARCH-011` plugin-and-scripting-abi, `ARCH-002` preference-storage,
  `11-cross-cutting/crash-recovery-and-autosave.md`,
  `00-overview/cs6-editions-and-constraints.md`.

> Version numbers and update mechanisms on the Linux side are **design
> proposals**. CS6 specifics not confirmed by a fetched source are marked
> *(inferred)* or *(unverified)* and collected under `## Open questions`.

## CS6 behavior

- **Application version.** Photoshop CS6 identifies itself as version **13.0**
  (with a maintenance release 13.0.1). The version/build appears in
  `Help > About Photoshop` and the splash screen. Adobe's own version numbering
  is a single major number plus a minor/patch (13.0, 13.0.1), not SemVer.
- **Document-format version.** The native file header carries a 2-byte
  `Version` field: **1 for PSD**, **2 for PSB** (Large Document Format), which
  is the only in-band discriminator the format offers. The optional
  **Version Info** image resource (ID `1057`, Photoshop 6.0+) records a
  `4 bytes version`, a `hasRealMergedData` flag, a **writer name** (Unicode),
  a **reader name** (Unicode), and a `4 bytes file version`.
- **Forward compatibility.** Photoshop generally refuses or warns on files
  written by a newer product generation because unknown layer features may not
  survive a round-trip. The exact CS6 wording and behavior for a future-version
  PSD is *(unverified)* — see Open questions. Whatever the wording, the safe
  contract is: a newer document must open read-only or with an explicit warning,
  never silently drop features.
- **Plug-in ABI version.** CS6 loads 64-bit C/C++ plug-ins built with the
  matching toolchain; the binary ABI is tied to the Photoshop release and has no
  runtime minor-version negotiation. A mismatched plug-in simply fails to load.
  Kooka Pictura replaces this with an explicit versioned ABI
  (`ARCH-011`).
- **Settings.** CS6 stores preferences in a version-specific file
  (`Adobe Photoshop CS6 Prefs.psp`); settings are not portable across major
  versions, and deleting/renaming the file resets to defaults. There is no
  documented in-app settings migration; the observable contract is
  "wrong/corrupt store ⇒ defaults" (`ARCH-002`).
- **Update delivery.** On Windows/macOS, CS6 updates are delivered by Adobe's
  updater infrastructure, not by an in-app downloader. Linux has no such
  channel; delivery is re-specified for Flatpak, AppImage, and distro packages
  below. *(The exact CS6 updater product name is unverified here and noted in
  Open questions.)*

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| About dialog | Dialog | `Help > About Photoshop` | Shows app version, build, Qt, GPU backend, codec versions (proposed extension). |
| Splash screen | Splash | — | Displays version/build during cold start. |
| `--version` | CLI | — | Build/app/format/ABI versions for support. |
| Open/Save version warning | Dialog | — | Future-version document, or feature loss on downgrade save. |
| `Edit > Preferences > Updates` | Preference pane | — | Update channel and check policy (proposed; no CS6 equivalent). |
| Update available notification | Banner/dialog | — | Non-intrusive; never on first launch before the user sees a document. |
| Plugin manager | Dialog | — | Shows plug-in requested vs host ABI major/minor; refuses incompatible. |
| AppStream/metainfo | Metadata | — | Release notes and version history for software centers. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| App version | SemVer | `0.1.0` | `MAJOR.MINOR.PATCH[-pre][+build]` | Proposal; CS6 used `13.0[.1]`. |
| Build metadata | string | git describe | any | Exposed in About/`--version`, not in the document. |
| Document format version | u16 pair | `(1,0)` | PSD `1`, PSB `2`; extension version independent | Mirrors the file header. |
| `opDocumentVersion` | u16 | `1` | `0`…`65535` | Private/XMP field for Kooka Pictura-written documents. |
| `minReaderVersion` | u16 | `1` | `0`…`65535` | Newer value ⇒ read-only open. |
| Plugin ABI major | u32 | `1` | host-enforced | Different major ⇒ refuse load. |
| Plugin ABI minor | u32 | `0` | host-enforced | Host hides fields newer than its own. |
| Settings schema version | u32 | `1` | monotonic | Drives migration chain. |
| Update channel | enum | `stable` | stable, beta, nightly | Flatpak branch or AppImage update-info. |
| Check-for-updates | enum | `on-launch` | off, manual, on-launch | On by default with an opt-out; no identifier is sent. |
| Update method | enum | `package` | flatpak, snap, appimage, deb, rpm, windows, macos | Detected at runtime from the package marker, environment, path, or package database. |
| Keep previous version | bool | on (AppImage) | on/off | AppImage `.zs-old` backup semantics. |

## Algorithms & pipeline

### Application versioning

- **Scheme.** Semantic versioning `MAJOR.MINOR.PATCH`, pre-release and build
  metadata as Cargo/tooling define. Cargo's SemVer rules (major = breaking,
  minor = additive, patch = fix) govern the public Rust crates; the *application*
  version is a product number and does not have to match crate versions.
- **Source of truth.** `CARGO_PKG_VERSION` baked at build time via `build.rs`;
  `git describe` appended as build metadata in non-release builds. One constant
  is the single source for About, `--version`, AppStream, and crash reports.
- **Channels.** stable / beta / nightly, each mapping to a package source
  (Flatpak branch, AppImage update-info, or a distro repo). Channels never mix:
  a nightly does not update to stable (`AppImage` channel rule).
- **Compatibility promise.** Within a MAJOR line, documents and plugins remain
  loadable (forward within minor: additive only). A MAJOR bump may raise
  `minReaderVersion`.

### Document-format versioning and migration

- **In-band.** PSD `Version` 1 / PSB `Version` 2 are the only native
  discriminators; we never repurpose them.
- **Extension.** Kooka Pictura writes `opDocumentVersion` and `minReaderVersion`
  into a private image-resource block (in a reserved plug-in resource range) and
  mirrors them into XMP. Unknown to CS6 and other readers, which preserve the
  block opaquely (`ARCH-007`).
- **Read policy:**

  | Condition | Behavior |
  |---|---|
  | `opDocumentVersion ≤ ours`, `minReaderVersion ≤ ours` | Open read-write; migrate model in memory if needed. |
  | `opDocumentVersion > ours`, `minReaderVersion ≤ ours` | Open read-write, preserve unknown blocks; warn once. |
  | `minReaderVersion > ours` | Open **read-only**; save-as requires a new version or explicit lossy-copy confirmation. |
  | Native `Version` not 1/2 | Refuse with a clear error; do not guess. |

- **Write policy.** A document is written at the lowest version that can
  represent its current features; features introduced later raise
  `opDocumentVersion` and set `minReaderVersion` accordingly. Unknown blocks are
  carried forward verbatim (`ARCH-007`).
- **Migration.** A chain of pure functions `migrate_vN_to_vN+1(model)`; the
  chain is total (never panics) and idempotent. Downgrade is *not* attempted;
  a "save a copy compatible with an older version" path flattens/omits features
  with an explicit warning, and is a separate command.

### Plugin ABI versioning

Follows `ARCH-011` directly: one entry symbol
`kookapictura_plugin_entry_v1`, an `abi_major`/`abi_minor`/`struct_size` header,
capability negotiation, and refusal on major mismatch. Within a major, the host
reads only the struct prefix it knows and treats absent trailing function
pointers as unsupported. A new incompatible revision adds `_v2` rather than
mutating `_v1`. The plugin manager surfaces the mismatch reason.

### Settings migration

- The store carries `prefs_version`. On load:
  1. If absent, treat as version 0 and run the full migration chain.
  2. If **older**, run `migrate_vN_to_vN+1` in order; unknown keys are preserved
     (forward compatibility for rolling back a beta).
  3. If **newer** (downgrade), do not rewrite; load known keys, keep the file
     intact, and warn.
  4. If **corrupt**, back up the file and start from defaults (CS6 contract).
- Migrations are pure key/value transforms with defaults supplied by the
  `PrefsSchema` (`ARCH-002`), so defaults live in exactly one place.

### Update delivery on Linux

Each package type owns a different update path:

| Format | Update owner | Mechanism | Version visibility |
|---|---|---|---|
| **Flatpak** | Flatpak/OSTree + Flathub | `flatpak update`; OSTree deduplicated/delta pulls; branch = channel; signed remote; rollback supported | OSTree commit + app version in metainfo |
| **Snap** | snapd + the store | `snap refresh`; channel = track/risk | Snap revision |
| **AppImage** | The application | Full verified asset download; SHA-256; atomic replace beside the old file; `.old` backup; relaunch | Embedded version + release asset |
| **Debian `.deb` / RPM `.rpm`** | The application + the package manager | Download the verified release package, install it with `pkexec` through apt/dnf (or rpm), then relaunch | Package manager database + the package marker |
| **Windows setup** | The application + the NSIS installer | Download the verified setup, launch it, quit; the installer relaunches the app | Installer registry + the package marker |
| **macOS DMG** | The application + the DMG | Download the verified DMG, open it, and offer to quit so the app can be replaced | App bundle marker + release asset |

- **Flatpak.** Builds are published to an OSTree repository; `flatpak update`
  pulls changes. Branch names encode the channel (`stable`, `beta`, `nightly`).
  Rollback is a documented Flatpak capability.
- **AppImage.** Update information is embedded (`appimagetool -u`, or
  `UPDATE_INFORMATION`/`UPD_INFO` for `linuxdeploy`); the updater downloads only
  changed bytes (zsync) and writes a new file next to the old one, keeping the
  old file as backup. Follow the AppImage golden rules: **never download without
  explicit consent**, respect system/user/ENV "do not check/do not update" flags,
  do not nag on first launch, use a GUI-consistent update experience, and keep
  the app usable during the update.
- **Debian/RPM packages.** The app downloads the release `.deb`/`.rpm`, verifies
  its SHA-256 digest, and asks the system package manager to install it with
  elevation (`pkexec apt-get install -y` or `pkexec dnf install -y`). The
  package manager remains the database owner; the app never rewrites package
  files directly. If `pkexec` or the package manager is unavailable, the app
  offers the release page instead.
- **Flatpak/Snap packages.** The store owns the files; the app never downloads
  or replaces them. It shows `flatpak update …` or `snap refresh …` and the
  release page.
- **Privacy.** Update checks are on at launch by default, with a
  **Check for updates automatically** opt-out in the dialog and Preferences.
  The request sends no unique identifier; GitHub sees the client IP.
- **Runtime detection.** Every package carries a `package-method` marker next to
  its executable (`appimage`, `flatpak`, `snap`, `deb`, `rpm`,
  `windows-setup`, `windows-portable`, `macos-dmg`). The app reads that marker
  first, then the sandbox environment, the executable path, the Windows install
  marker, the macOS bundle, and the package database. If none applies, it treats
  the build as development and offers only the release page.

## Rust module mapping

- `pictura_version` — `AppVersion` (`MAJOR.MINOR.PATCH`), `BuildInfo`, and
  compile-time constants from `build.rs`.
- `pictura_doc::format_version` — `FormatVersion { native: u16,
  op_document: u16, min_reader: u16 }`; `ReadPolicy` and `WritePolicy` enums.
- `pictura_doc::migrate` — `migrate(model, from, to) -> Result<Model, MigrateError>`
  as a total chain of `Migration` steps.
- `pictura_doc::version_info` — read/write of the Version Info resource (1057)
  and the private `opDocumentVersion`/`minReaderVersion` block.
- `pictura_plugin::loader` — reuses `ARCH-011`'s ABI header validation; exposes
  `AbiCompatibility { Compatible, MinorSkew, MajorMismatch }`.
- `pictura_prefs::migrate` — `PrefsSchema`-driven migration chain; preserves
  unknown keys.
- `pictura_update::method` — runtime detection (`Flatpak`, `AppImage`,
  `Distro`).
- `pictura_update::channel` — `Channel { Stable, Beta, Nightly }` and
  check/enable policy from preferences.
- `pictura_update::appimage` — thin wrapper over `libappimageupdate`
  (`checkForChanges`, background `start()`, `progress`, `.zs-old` handling) with
  consent gating.
- `pictura_update::flatpak` — queries `flatpak`/libflatpak for the installed
  commit and available update; never rewrites the OSTree repo itself.
- `pictura_update::distro` — no-op updater; surfaces the package-manager hint.

Types crossing the Rust↔Qt boundary: `AppVersion`, `BuildInfo`,
`FormatVersion`, `ReadPolicy`, `AbiCompatibility`, `UpdateChannel`,
`UpdateStatus`. No Qt types leak into Rust.

## Qt6 component mapping

| Proposal | Base | Responsibility |
|---|---|---|
| `AboutDialog` | `QDialog` | App/build/Qt/GPU/codec versions; copy-to-clipboard. |
| `NewerDocumentDialog` | `QMessageBox` | Warn on future-version or downgrade-save; offer read-only / save-as-copy. |
| `PluginCompatibilityDialog` | `QDialog` | Show why a plug-in was refused (ABI major/minor). |
| `UpdatePreferencesPage` | `QWidget` | Channel, check policy, consent; part of `PreferencesDialog`. |
| `UpdateChecker` (`QObject`) | cxx-qt | Marshals `pictura_update` into Q_PROPERTYs; emits `updateAvailable`, `progress`. |
| `UpdateBanner` | `QWidget` | Non-modal notification; never shown before first document. |
| `packageVersion` in `.desktop`/metainfo | Metadata | Static per release; generated in the packaging step. |

Widgets (not QML) because these are desktop-native dialogs and preference panes
that must match the CS6 docked-dialog feel; the update checker is a headless
`QObject`.

## Data-model impact

- **New persisted document fields** (in the private image resource and mirrored
  in XMP): `opDocumentVersion: u16`, `minReaderVersion: u16`, `writerVersion:
  String`, plus the existing native `Version` (1/2) and Version Info resource
  (1057) fields `version`, `hasRealMergedData`, `writer`, `reader`,
  `fileVersion`.
- **Opaque coexistence.** CS6 and other readers see an unknown image resource and
  preserve it verbatim; round-trip tests assert that preservation
  (`XC-010`).
- **Preferences** gain `prefs_version` and the update settings
  (`channel`, `check_policy`, `consent`), owned by `ARCH-002`, never written to
  PSD/XMP.
- **ABI metadata** is runtime-only (`PlugInRegistration` in `ARCH-011`), never
  serialized into a document.
- **Undo:** version migration and update settings are not document edits and do
  not enter the History panel. A downgrade "save a copy" that flattens features
  *is* a document command and is undoable as one step.

## Edge cases

- **Open a newer document** — do not panic or silently drop features; read-only
  + warning, and preserve unknown blocks on any subsequent save-as-copy.
- **Downgrade save** — must be an explicit, lossy, warned operation; never the
  default `Save`.
- **Partial extraction of version fields** — missing `opDocumentVersion` means
  "legacy/foreign"; treat conservatively as read-write with preservation.
- **Migration chain gaps or unknown intermediate version** — refuse to migrate
  and open read-only rather than guess.
- **Corrupt/partial preferences** — back up and default, per CS6 behavior.
- **Rolling back a beta** — unknown (newer) preference keys are preserved so the
  beta can be reinstalled without losing settings.
- **Flatpak rollback** — after rollback the on-disk document may carry a newer
  `opDocumentVersion`; read-only + warning path applies.
- **AppImage in a read-only directory** — update cannot write a sibling file;
  surface a clear error, do not corrupt the running image.
- **Flatpak/Snap build offering self-update** — blocked; the store command is shown. deb/rpm update through the package manager.
- **Update while documents are open** — prompt to save/restart; never kill a
  document or an in-flight save.
- **Offline / metered** — an automatic check is silent; a manual check reports
  the failure and respects the do-not-check flags.
- **Qt/Rust toolchain skew in Flatpak** — the runtime pins Qt; version mismatch
  with a distro build must be reported, not crash.
- **Clock/timezone** — build timestamps are informational only; never used for
  ordering.
- **ABI minor skew** — load and hide unknown trailing fields; log the skew.

## Parity acceptance criteria

1. Given a fresh install, `Help > About Photoshop` and `--version` report the
   same application version, build, Qt, GPU-backend, and codec versions as the
   package metadata.
2. Given a document written by Kooka Pictura at `opDocumentVersion = N`, opening
   it with a reader whose `minReaderVersion ≤ N` succeeds and preserves all
   unknown blocks byte-for-byte on re-save.
3. Given a document with `minReaderVersion` greater than the reader's, it opens
   read-only and the first save requires an explicit save-as-copy warning.
4. Given a PSD with native `Version = 1` and a PSB with `Version = 2`, both are
   detected from the header (not the extension) and handled accordingly.
5. Given a Version Info resource (1057), its writer/reader/version fields are
   parsed, displayed where applicable, and preserved on re-save.
6. Given a plug-in built for ABI major 2 and a host at major 1, load is refused
   and the manager reports a version mismatch; a minor-skewed plug-in loads and
   hides unknown fields.
7. Given a preferences store at schema version `N-2`, launching migrates it to
   `N` without losing known values and preserves unknown keys; a store at a
   newer version is loaded without rewriting the file and a warning is shown.
8. Given the Flatpak build, `flatpak update` moves between commits/branches and
   a rollback restores the previous commit; the app reports the OSTree commit it
   is running.
9. Given an AppImage and an update available, the app downloads the verified
   asset only on explicit consent, keeps the old file as `.old`, and remains
   usable during the update.
10. Given a deb or rpm build, the app downloads the verified package, installs
    it through the system package manager with elevation, and relaunches; a
    Flatpak or Snap build shows the store command instead.
11. Given a check-for-updates setting of `off`, no network request is made at
    launch.
12. Given an update is available while a document is open, the app prompts to
    save and restart rather than killing the document.

## Sources

- `https://doc.rust-lang.org/cargo/reference/semver.html` — SemVer change
  categories for Rust/crates: major vs minor vs patch, `non_exhaustive`,
  additive compatibility; establishes the crate-level compatibility rules used
  above.
- `https://web.archive.org/web/2023id_/https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/`
  — PSD/PSB `Version` field (1 and 2), Version Info image resource ID 1057
  (4-byte version, `hasRealMergedData`, writer/reader Unicode strings, 4-byte
  file version), and the CS6 Auto Save resources 1086/1087. Primary source for
  file-header versioning.
- `https://docs.flatpak.org/en/latest/under-the-hood.html` — Flatpak is built on
  OSTree ("Git for apps"): branches, versioned updates, deduplication,
  rollback; GPG key in the `.flatpakrepo`.
- `https://docs.flatpak.org/en/latest/using-flatpak.html` — `flatpak update`,
  remote/`.flatpakrepo`/`.flatpakref` install, identifier triples
  (`name/arch/branch`), per-user vs system.
- `https://docs.appimage.org/packaging-guide/optional/updates.html` — embedded
  update-info via `appimagetool -u` or `UPDATE_INFORMATION`/`UPD_INFO`; external
  vs self-update; `libappimageupdate`; the golden rules (consent, do-not-check
  flags, channels, stay usable, no first-launch nagging), zsync delta.
- `https://github.com/AppImageCommunity/AppImageUpdate` — **README fetched from
  the `AppImageCommunity/AppImageUpdate` repository**: decentral update via
  update-info embedded in the ISO 9660 "Application Used" field; delta updates;
  old file backed up as `.zs-old`; never overwrites before validation;
  `updater.start()`, `progress()`, `hasError()`, `pathToNewFile()`.
- `https://doc.qt.io/qt-6/qsettings.html` — on Unix, `NativeFormat` stores
  INI-style `.conf` under `$XDG_CONFIG_HOME`/XDG dirs; fallback search order;
  atomic sync/locking; INI type-loss caveat. Used for the settings-migration
  design.
- `https://docs.rs/dssim/latest/dssim/` — comparison variant (see `XC-010`).
- Internal: `docs/01-architecture/plugin-and-scripting-abi.md` (ABI
  `abi_major`/`abi_minor`/`struct_size`, `_v1` symbol), `docs/01-architecture/file-formats.md`
  (PSD/PSB, Version Info), `docs/02-ui-ux/preferences.md` (`prefs_version`,
  corrupt-store-to-defaults), `docs/01-architecture/build-and-packaging.md`
  (Flatpak/AppImage/distro targets).

CS6 application version 13.0 and the Adobe updater product name are
*(secondary/unverified)* here and listed in Open questions.

## Open questions

- **Exact CS6 forward-compatibility behavior.** The precise warning/refusal for
  a future-version PSD, and whether CS6 offers a read-only open, is unverified.
  *Resolves with:* a CS6 experiment opening a document with a bumped `Version`
  or unknown layer keys, and the CS6 Help topic on compatibility.
- **CS6 Version Info writer/reader semantics.** Who writes fields 1057 and
  whether a "Save a Copy" changes them is unverified. *Resolves with:* inspecting
  CS6-written files with a resource dump.
- **CS6 update mechanism.** The exact updater product for CS6 (Adobe
  Application Manager vs Creative Cloud) is not sourced. *Resolves with:* an
  Adobe release-notes/advisory source; otherwise keep it out of the parity
  contract.
- **Version numbering for Kooka Pictura.** Whether to mirror CS6's `13.x` or use
  an independent SemVer line. *Resolves with:* a product decision; independent
  SemVer is the working default.
- **Private resource ID range.** Which plug-in resource ID to claim for
  `opDocumentVersion`. *Resolves with:* the PSD spec's plug-in range
  (4000–4999) and a collision check against known plug-ins.
- **XMP namespace choice.** Whether to mirror version fields into a custom XMP
  namespace or a private resource only. *Resolves with:* an XMP-design decision.
- **Update-check privacy design.** Exact opt-in flow and any telemetry
  questions. *Resolves with:* `11-cross-cutting/logging-and-telemetry.md` and
  `11-cross-cutting/security-and-sandboxing.md`.
- **Flatpak vs AppImage as primary.** Which channel ships first. *Resolves
  with:* the packaging decision in `ARCH-004`.
- **libappimageupdate on Qt6 builds.** Integration cost and whether to bundle
  it. *Resolves with:* an update spike.
- **Migration test corpus.** Whether version-migration tests live with the
  `XC-010` corpus or a dedicated fixtures directory. *Resolves with:* a test
  layout decision.
