# Security and Sandboxing

- **Spec ID:** `XC-005`
- **Status:** `Draft`
- **Parity tier:** `Core` (memory safety, untrusted-input hardening, path handling); **`Non-goal (CS6)`** for in-process parity with Adobe plug-in trust — Kooka Pictura proposes an out-of-process sandbox for untrusted plug-ins and a script permission model that CS6 does not have.
- **New in CS6:** `No` — CS6 has no application sandbox, no plug-in signing/verification, and no script permission model. Native 8BF plug-ins and ExtendScript run in-process with full user privileges (`ARCH-011`). Kooka Pictura's sandboxing is a Linux-native addition, not CS6 parity.
- **Depends on:** `01-architecture/system-architecture.md` (`ARCH-001`), `01-architecture/plugin-and-scripting-abi.md` (`ARCH-011`), `01-architecture/file-formats.md` (`ARCH-011` provisional), `01-architecture/build-and-packaging.md`, `11-cross-cutting/error-handling.md` (`XC-004`), `11-cross-cutting/localization.md` (`XC-001`), `11-cross-cutting/logging-and-telemetry.md` (`XC-003`), `00-overview/licensing-and-provenance.md`.

> Sandbox design below is a **design proposal**. Rust crate names are provisional. Rust codec safety posture reflects the published crates at the versions cited; it is not a guarantee and must be re-verified per release via `cargo-audit`.

## CS6 behavior

### Trust model (what CS6 actually does)

Photoshop CS6 is a **single, unsandboxed desktop process** running with the user's full privileges (`ARCH-001`). That has direct security consequences:

- **Native plug-ins are in-process.** CS6 loads `.8bf`/`.8li`/… modules into its address space; a plug-in can read/write arbitrary files, open sockets, and corrupt memory. Adobe's trust model is "locally installed = trusted"; there is no signing, no capability declaration, and no sandbox. The SDK is license-gated but not verified at load time (`ARCH-011`).
- **Scripts are in-process too.** ExtendScript (`File`, `Folder`, `Socket`, `ExternalObject`) can perform filesystem and network I/O. The documented CS6 security surface for plug-ins is a single preference — *Plug-ins → Allow Extensions To Connect To The Internet* — plus *Additional Plug-ins Folder* (`UI-010`), which only controls where plug-ins are found, not what they may do.
- **Documents are trusted input.** PSD/PSB/TIFF/RAW/PDF/EPS parsers process attacker-controllable files. CS6's parsers are native code; malformed image files are a classic memory-corruption/exploit vector. Adobe periodically shipped security updates for Photoshop's file parsing, but no CS6-era hardening posture is documented here.
- **No privilege separation.** There is no helper process, no seccomp/AppArmor profile, no container; Windows Vista+ low-integrity/Job objects and macOS sandboxing are host-level concerns not applied to Photoshop CS6.

### Consequences for parity

Kooka Pictura cannot be "CS6-parity secure" because CS6 has no security architecture to match. The correct posture is: **match CS6's user-facing trust affordances where they exist** (a plug-ins folder, an internet-access toggle) and **exceed CS6 on the Linux-native properties that matter** (memory safety, untrusted decoders, sandboxing, script permissions).

## UI surface

| Location | Type | Shortcut | Notes |
|---|---|---|---|
| `Edit > Preferences > Plug-ins > Additional Plug-ins Folder` | Preference | — | CS6 parity; second plug-in search path. |
| `Edit > Preferences > Plug-ins > Allow Extensions To Connect To The Internet` | Toggle | — | CS6-era extension internet access; default off *(secondary)*. |
| Plug-ins manager (proposed, `ARCH-011`) | Dialog | — | List/disable, show granted capabilities, surface sandbox mode. |
| Script permission prompt (proposed, `ARCH-011`) | Dialog | — | Per-script filesystem/network grant. |
| Open-file / save-file dialogs | Portal-backed | `Ctrl/Cmd+O`, `Ctrl/Cmd+S` | Under Flatpak, use portals rather than broad home access. |
| Security status indicator (proposed) | Status/dialog | — | "Running sandboxed: yes/no", network permission state. |
| Plug-in trust prompt (proposed) | Dialog | — | First-load confirmation for an out-of-process plug-in. |

## Parameters & ranges

| Control | Type | Default | Range / options | Notes |
|---|---|---|---|---|
| Plug-in trust | enum | trusted in-process | trusted / sandboxed / blocked | CS6 has no such control. |
| Capability bitset | u64 | plugin-declared | see `ARCH-011` | FS/network denied by default. |
| Network access | bool | off | on / off | Telemetry/plugin network requires explicit consent. |
| Script timeout | duration | 30 s | 0 = unlimited | `ARCH-011`; QuickJS interrupt handler. |
| Script memory limit | bytes | 64 MiB | 0 = unlimited | `ARCH-011`; `set_memory_limit`. |
| Script stack limit | bytes | 256 KiB | — | `ARCH-011`; `set_max_stack_size`. |
| Module allowlist | path list | empty (deny) | per-script | `ARCH-011`. |
| Decoder resource limits | struct | conservative | width/height/bytes/time | `image::Limits`-style guards. |
| Decompression-bomb ratio | float | 100× | 1–n | PSD ZIP / PNG / EXR expansion guard. |
| Flatpak filesystem args | finish-args | minimal | `:ro`, `xdg-*`, `--filesystem` | See `build-and-packaging.md`. |
| Sandbox helper memory cap | bytes | 512 MiB *(proposal)* | ≥1 tile | Crash isolation for decoders/plug-ins. |

## Algorithms & pipeline

### Threat model

Assets: the user's document contents and pixel data; other files on the machine; credentials/session; network; the application's integrity and availability.

| Adversary | Entry vector | Primary controls |
|---|---|---|
| Malicious document | PSD/PSB/TIFF/RAW/PDF/EPS/EXR parsing, embedded XMP/ICC/EXIF | Rust safe parsers; resource limits; fuzzing; optional out-of-process decode; XXE-safe XML |
| Malicious plug-in | Native C ABI module in the plug-ins folder | Capability bits; default in-process (trusted) vs out-of-process sandbox for untrusted; no `dlclose`; crash isolation |
| Malicious script | `.jsx`/script run by the user or an event notifier | Default-deny FS/network; per-script grant; QuickJS memory/stack/timeout limits; module allowlist |
| Misled agent | Prompt injection through a document, or a malicious MCP client | Off by default; enforcement in `ControlServer`; permission tiers, allowed folders, approval prompts (`XC-022`). Does not cover an agent that also has a shell |
| Compromised dependency | Crate/toolchain supply chain | `cargo-audit`/`cargo-deny`, lockfile, vendoring, `cargo-auditable` SBOM |
| Network attacker | Telemetry/update endpoints | HTTPS only; no silent endpoints; opt-in (`XC-003`) |
| Local attacker / another app | Same-user files, `/tmp`, X11 sniffing | Flatpak sandbox; Wayland (or `fallback-x11`); XDG dirs `0700`; portals |
| Path/symlink attacker | Crafted file paths, plug-in dir entries | Canonicalize; reject `..`/symlinks at boundaries; `O_NOFOLLOW` where applicable |

### Trust boundaries

```text
 untrusted file ──▶ pictura-codec (Rust, safe) ──▶ document model
                        │ (optional out-of-process decode helper)
 untrusted script ─▶ QuickJS (rquickjs, default-deny) ─▶ command bus
 untrusted plug-in ─▶ [C ABI] ─┬─ in-process: capability checks (trusted-by-user)
                              └─ out-of-process helper: shared-memory tiles + RPC (sandboxed)
 Qt UI ──▶ cxx-qt bridge ──▶ Rust core   (typed, catch_unwind at the boundary)
 Rust core ──▶ GPU driver (trusted, but may crash/lose device)
 OS sandbox (Flatpak) ──▶ portals (file chooser, print, notifications)
```

### Memory safety and the codec surface

- **Rust core is memory-safe by default.** `unsafe` is confined to vetted modules (FFI boundary, SIMD kernels, GPU buffer mapping), each with a `// SAFETY:` rationale, and is audited with Miri where feasible. `cargo-geiger`/`cargo-deny` report `unsafe` and dependency risk. This eliminates the dominant class of CS6-style codec exploits for code written in the Rust core.
- **The `psd` crate is a parse-only, small-dependency parser.** Its published dependency list is just `thiserror` and `anyhow` (dev), and its API is a read path (`Psd`, `PsdLayer`, channels, image resources). It has no writer and no obvious native-code dependency, which is a good starting posture — but "safe Rust" does not mean "cannot panic/loop/OOM on hostile input". Parse errors must be returned, not panicked, and resource limits must be enforced by the caller.
- **The `image` crate states it "prefers safe solutions with few dependencies"** and exposes a `Limits` resource-limits type for decoders and `LimitSupport`. Use it to bound width/height/allocation before decoding. Codec coverage includes PNG/TIFF/JPEG/GIF/BMP/EXR/HDR/TGA via `png`, `tiff`, `gif`, `zune-jpeg`, `exr`, etc.
- **Treat every decoder as untrusted.** Preferred hardening order: (1) safe Rust decoder with limits; (2) fuzz the decoder (`cargo-fuzz`) with a seeded corpus; (3) for high-risk or native-backed formats, decode in an **out-of-process helper** with a memory cap and no filesystem access (`ARCH-001` core-subprocess proposal).
- **Decompression bombs.** PSD ZIP / ZIP-with-prediction, PNG/EXR compression, and TIFF LZW/ZIP can expand enormously. Enforce a max decoded byte budget and a max expansion ratio; abort with a clean error, not an allocation failure.
- **XML metadata is untrusted.** XMP/RDF and XML sidecars must be parsed with entity expansion disabled (XXE/billion-laughs). Prefer a parser that does not resolve external entities.
- **Integer arithmetic.** Size/offset math on attacker-controlled lengths must use checked arithmetic; PSB uses 8-byte length fields near the 300,000 px limit (`ARCH-011` file-formats), a classic overflow site.

### Plug-in sandboxing

Two trust tiers (`ARCH-011`):

- **Trusted, in-process (default, CS6-like):** a user-installed plug-in is loaded via `libloading`, negotiated against `OpHostApi`, and constrained by capability bits (`OP_CAP_FILESYSTEM`, `OP_CAP_NETWORK` denied by default). The `Library` is pinned/leaked — never `dlclose`d — because unloading Rust `cdylib`s with TLS/statics is not reliably safe. A panicking plug-in is caught at the boundary (`OP_ERR_PANIC`) and disabled; the host survives.
- **Untrusted, out-of-process (proposed):** a helper process hosts the plug-in behind the same C ABI adapted to RPC, with pixel buffers passed through shared memory (`memfd`/`/dev/shm`) and a small control channel. Benefits: crash isolation, a hard memory cap, and the ability to apply seccomp/AppArmor or a Flatpak-style sandbox to the helper only. A WASM path (`wasmtime`) is reserved as an alternative for pure-compute plug-ins. Neither exists in CS6 and both are proposal-only.
- **No silent filesystem grants.** Filesystem/network access is opt-in per capability and surfaced in the manager UI. Network is denied by default.
- **Unload/hot-reload** is approximated by loading a new version and draining the old (`ARCH-011`); document the per-reload memory ceiling.

### Scripting resource limits

Per `ARCH-011`: primary engine is QuickJS via `rquickjs`, with `set_memory_limit`, `set_max_stack_size`, and an `set_interrupt_handler` wall-clock timeout (default 30 s). Module imports are restricted to an allowlist; `File`/`Folder`/`Socket` are default-deny with per-script grants; symlink/`..` traversal is rejected; `ExternalObject` native loading is not implemented. A tripped limit aborts the script's transaction and leaves the document unchanged. (Rhai, if adopted, has documented DoS vectors — strings, arrays, closures, deep recursion, self-referencing modules — that must be re-checked.)

### File and path handling

- Canonicalize and validate paths at every trust boundary; reject `..` and symlink escapes when resolving plug-in directories, script `#include`, or user-specified preset folders.
- Prefer `O_NOFOLLOW`/`openat2`-style resolution where a fixed root is intended; never `chdir` into an untrusted directory.
- Never build shell commands from filenames; there is no `system()`/shell execution in the core. External tools (e.g. a PostScript renderer) are invoked via `execve` with explicit argv and a sanitized environment, ideally inside the sandbox.
- Filenames are UTF-8 and may be arbitrary; validate lengths and reject NUL/control characters where the OS requires.
- Save paths use atomic write semantics (`QSaveFile`/temp+rename) so a failed or hostile path operation cannot half-write (`XC-004`).

### Flatpak sandbox (Linux packaging)

Flatpak's default sandbox already provides the profile this spec wants: no host files except the runtime, app, `~/.var/app/$FLATPAK_ID`, and `$XDG_RUNTIME_DIR/app/$FLATPAK_ID` (the latter two writable); **no network**; no device nodes; limited syscalls; filtered session D-Bus; no host X11/system-bus/PulseAudio by default.

Proposed `finish-args` policy (see `build-and-packaging.md`):

| Need | Grant | Rationale |
|---|---|---|
| Display | `--socket=wayland`, `--socket=fallback-x11` | Native Wayland first, X11 fallback; avoids the "very sensitive" `x11`-only grant where possible. |
| GPU | `--device=dri`, `--share=ipc` | OpenGL/Vulkan rendering; shared-memory X11 needs `ipc` on X11. |
| Config/cache/state | `xdg-config`, `xdg-cache`, `xdg-data` | Per-app XDG dirs; no broad `home`. |
| User image files | portals (open/save dialogs) | Grants only the chosen file; avoid `xdg-pictures` blanket access unless needed. |
| Printing | `--socket=cups` | Print spec. |
| Audio | `--socket=pulseaudio` | Only if audio features exist (CS6 has some). |
| Scanners/USB | USB portal (`--usb=…`) | CS6 scan/import; prefer the portal over `--device=usb`. |
| Network | `--share=network` **only** when telemetry/updates are explicitly enabled | Telemetry must not be the reason to widen the sandbox; degrade to no telemetry otherwise (`XC-003`). |

Avoid `--filesystem=home`, `--filesystem=host`, `--socket=system-bus`, and `--socket=session-bus`. Prefer `:ro` and `:create`. The app must write to its sandboxed XDG directories and must not hardcode `~/.config` (`XC-002`). Note the D-Bus caveat that network access also exposes abstract Unix sockets on the host; if network is granted, document it.

### Supply chain

- `Cargo.lock` committed; `cargo-audit` against the RustSec Advisory Database and `cargo-deny` for licenses/sources/advisories in CI.
- Vendor or pin critical dependencies; `cargo-auditable` to embed the dependency tree so shipped binaries can be audited.
- Track Qt security patches and rebuild against updated Qt; track image/codec crate releases.
- Generate an SBOM for each release; sign release artifacts and Flatpak builds.
- Treat translation catalogs (`.qm`), plug-ins, and updates as untrusted artifacts (`XC-001` warns malformed `.qm` can crash).

### Secrets and privileges

- Never run setuid/setgid; never request root. No credentials are stored (no account/license secret in the store; the telemetry install ID is random and not a secret; `XC-003`).
- Do not log secrets or user content (`XC-003` redaction).

## Rust module mapping

- `pictura-security::Limits` — width/height/decoded-byte/expansion-ratio/time bounds enforced before and during decode.
- `pictura-security::SafePath` — canonicalizing path resolver with root confinement; rejects traversal/symlinks at trust boundaries.
- `pictura-security::Capability` / `CapabilitySet` — the `ARCH-011` bitset wrapper; `deny_by_default()`.
- `pictura-security::Sandbox` trait — `InProcess` (capability checks) and reserved `OutOfProcess`/`Wasm` backends.
- `pictura-codec` — safe Rust decoders (`psd`, `image` codecs) with limits applied at the call site; fuzz targets under `fuzz/`.
- `pictura-script::sandbox` — QuickJS limits, module allowlist, per-script permissions (`ARCH-011`).
- `pictura-error` — maps security violations to stable codes (`OP_ERR_CAPABILITY`, `OP_ERR_CODEC`, `OP_ERR_INVALID_ARG`; `XC-004`).
- Types crossing Rust↔Qt: `CapabilitySet`, `SandboxStatus`, `SecurityError` (code + key).

## Qt6 component mapping

- `QFileDialog` — must route through the XDG desktop portal under Flatpak so the user's selection is the permission grant.
- `QDesktopServices` / portal URIs — opening external links without shelling out.
- `PluginManagerDialog` (`ARCH-011`) — show trust tier, granted capabilities, sandbox mode.
- `ScriptSecurityDialog` (`ARCH-011`) — per-script FS/network consent.
- `SecurityStatusDialog` (proposed) — report sandbox/network/permission state and Flatpak confinement.
- `QNetworkAccessManager` — HTTPS-only telemetry/update client with certificate validation; absent when network permission is not granted.
- Qt's own parsing (`QRegularExpression`, XML) must be used with entity expansion off for XMP/XML.

## Data-model impact

- **No document-schema change.** Security metadata never serializes into PSD/XMP.
- **Persisted security state** (in `prefs.toml`/state, `XC-002`): plug-in enable/disable and trust tier, per-script permission grants, network permission, telemetry consent, suppressed first-load prompts. Keys are stable IDs, not localized text.
- **Capability grants are application state, not document data;** they are not carried with a shared document.
- **No secrets** stored; the telemetry install ID is a random identifier (`XC-003`).
- **Undo:** granting/denying a permission is not a document command and is not undoable via the History panel.
- **Plug-in metadata** (id/version/capabilities) is runtime/prefs data, not PSD (`ARCH-011`).

## Edge cases

- **Malformed PSD/PSB/TIFF/RAW/EXR:** must return a typed error, never panic/loop/OOM; enforced by limits + fuzzing.
- **Decompression bomb:** decoded-byte/ratio budget aborts cleanly (`OP_ERR_CODEC`).
- **Integer overflow in size math:** checked arithmetic; PSB 8-byte lengths.
- **XXE / billion laughs in XMP/external XML:** no external-entity resolution.
- **Path traversal / symlink swap (TOCTOU):** confine to a root, resolve with `O_NOFOLLOW`, re-check after open.
- **Plug-in directory with a hostile subpath:** resolve canonical paths, reject escapes (`ARCH-011`).
- **Plug-in panic / crash:** in-process panic → `OP_ERR_PANIC` + disable; out-of-process crash → restart the helper, report, keep the document (`ARCH-001` core-subprocess proposal).
- **Script infinite loop / memory bomb / stack blow-out:** QuickJS limits trip and abort the transaction (`ARCH-011`).
- **Script `#include`/module escape:** allowlist + traversal rejection.
- **Network exfiltration via a script/plug-in:** denied by default; only explicit grants and consent allow it.
- **Telemetry payload PII:** redaction tests assert no path/filename/content (`XC-003`).
- **Flatpak portal unavailable (e.g. non-portal environment):** fall back to a regular file dialog within the sandbox; never broaden permissions implicitly.
- **GPU driver crash/device loss:** recover, re-create, re-upload (`ARCH-001`); GPU memory contents (user pixels) are transient and not persisted by the driver.
- **Unsandboxed native install (AppImage/distro package):** the Flatpak protections are absent; the app must degrade safely and the docs must state that the sandbox is packaging-dependent.
- **X11:** no isolation between same-user X11 clients; prefer Wayland (`fallback-x11` only when needed).
- **Dependency advisory published after release:** `cargo-audit` in CI plus a documented patch cadence.
- **Language packs and plug-ins as malware:** treat as untrusted; a malformed `.qm` can crash (`XC-001`), and a native plug-in is arbitrary code even when "trusted".

## Parity acceptance criteria

1. Given a corpus of malformed PSD/PSB/TIFF/PNG/EXR files, decoding returns a typed error for each; the process does not crash, hang, or exhaust memory (fuzz harness passes).
2. Given a PSD whose embedded ZIP expands beyond the configured ratio/byte budget, decoding aborts with `OP_ERR_CODEC` before allocating the full output.
3. Given XMP containing an external-entity/expansion payload, parsing completes without reading external resources and without unbounded expansion.
4. Given a script that requests network/filesystem access without a grant, the request fails with `OP_ERR_CAPABILITY` and is logged; with a grant, it succeeds only within the granted root/endpoint.
5. Given a script with an infinite loop, the host interrupts it within the configured timeout (±1 s), rolls back its transaction, and adds no history state (`ARCH-011` criterion 8).
6. Given a plug-in whose entry panics, the host reports `OP_ERR_PANIC`, disables the plug-in, and remains usable (`ARCH-011` criterion 4).
7. Given a plug-in or script path containing `..` or a symlink escape, resolution is rejected and no file outside the intended root is touched.
8. Given a Flatpak build with the proposed `finish-args`, the app cannot read arbitrary `$HOME` files except through the file-chooser portal, and cannot make network requests unless the network permission is granted.
9. Given telemetry is disabled (default), the sandboxed app attempts no network connection at all.
10. Given a malformed translation catalog or a hostile plug-in name/error string containing markup, the UI renders it as plain, length-limited text and does not execute or inject anything.
11. Given a release build, `cargo-audit` reports no known vulnerabilities in the dependency tree, and `cargo-geiger`/`cargo-deny` reports the `unsafe` surface and dependency licenses.

## Sources

Fetched for this document:

- `https://docs.flatpak.org/en/latest/sandbox-permissions.html` — Flatpak default sandbox (no host files except runtime/app/`~/.var/app/$FLATPAK_ID`/runtime dir, no network, no device nodes, limited syscalls, filtered session D-Bus, no host X11/system-bus/PulseAudio); portals (file chooser, print, notifications, screenshots, USB); standard permissions (`--device=dri`, `--share=ipc`, `--socket=wayland`/`fallback-x11`/`x11`, `--socket=cups`, `--socket=pulseaudio`); filesystem guidance (`:ro`, `:create`, XDG dirs, avoid `home`/`host`); D-Bus `--socket=system-bus`/`session-bus` risk; USB portal; reserved paths; the caveat that `--share=network` also exposes abstract Unix sockets.
- `https://docs.rs/image/latest/image/` — `image` 0.25.10: "native rust implementations", "prefers safe solutions with few dependencies"; `Limits`/`LimitSupport` resource-limit types; `ImageDecoder`/`ImageDecoderRect`; codec list and underlying crates (`png`, `tiff`, `gif`, `zune-jpeg`, `exr`, `image-webp`, `qoi`, `moxcms`).
- `https://docs.rs/psd/latest/psd/` — `psd` 0.3.5: "Data structures and methods for working with PSD files", parse-only API (`Psd`/`PsdLayer`/channels/image resources); dependency list is `thiserror` + `anyhow` (dev); no writer/unsafe-native dependency surfaced.
- `https://rustsec.org/` — RustSec Advisory Database, `cargo-audit`, `cargo-deny`; OSV/GitHub/Debian export of advisories; used for the supply-chain controls.
- `https://doc.rust-lang.org/std/panic/fn.catch_unwind.html` — panic-catching at FFI boundaries (`catch_unwind` semantics and limits), used for the plug-in/FFI safety story.
- `https://doc.qt.io/qt-6/qtranslator.html` — the security warning that malformed translation files "may crash the application" and that "even well-formed translation files may contain misleading or malicious translations"; basis for treating catalogs as untrusted.

Found via search, **not fetched** (leads):

- `https://helpx.adobe.com/photoshop/kb/preference-file-names-locations-photoshop.html` — HTTP 403.
- Adobe security bulletins / Photoshop CS6 CVE advisories — not fetched; the claim that native parsers were a historical exploit vector is general and unverified for CS6 specifically.
- `https://docs.flatpak.org/en/latest/portals.html` and the `xdg-desktop-portal` API docs — referenced for portal details; not fetched in this pass.

Internal cross-references (not sources): `docs/01-architecture/plugin-and-scripting-abi.md` (`ARCH-011`) — capability bits, `catch_unwind`, QuickJS limits, sandbox tiers; `docs/01-architecture/system-architecture.md` (`ARCH-001`) — single-process model, core-subprocess proposal, scratch/GPU recovery; `docs/01-architecture/file-formats.md` — format matrix and crate choices; `docs/01-architecture/build-and-packaging.md` — Flatpak/AppImage/native packaging.

## Open questions

- **Is there a CS6-era CVE list for Photoshop file parsing?** Not established here. *Resolve:* Adobe security bulletin archive; would sharpen the risk ranking of each codec.
- **Out-of-process decode/plug-in host: worth the IPC cost?** *Resolve:* measure tile round-trip latency across shared memory vs. the crash-isolation benefit (`ARCH-001` open question).
- **Which decoders require native/`unsafe` backends** (e.g. `jpeg2k`/OpenJPEG, `libraw`, Ghostscript for PDF/EPS) and whether they must be sandboxed rather than merely limits-guarded. *Resolve:* per-crate audit in `file-formats.md`.
- **WASM (`wasmtime`) as a third plug-in tier** for pure-compute plug-ins. *Resolve:* prototype + ABI review (`ARCH-011`).
- **Flatpak `finish-args` final set.** This spec proposes a policy; the exact manifest is a packaging decision. *Resolve:* `build-and-packaging.md` + Flathub requirements.
- **Script permission persistence and revocation UX.** Storing grants is an attack surface (auto-run notifiers). *Resolve:* security review with `ARCH-011`'s Script Events Manager.
- **Telemetry endpoint and whether any network permission ships at 1.0.** *Resolve:* product/legal decision (`XC-003`).
- **Fuzzing coverage target and CI budget** for the codec corpus. *Resolve:* testing-strategy spec.
- **`psd` crate write path.** The crate is parse-only; a custom PSD/PSB writer is required (`file-formats.md`), and that writer's hardening is not covered by the crate's safety posture. *Resolve:* writer design + fuzzing.
