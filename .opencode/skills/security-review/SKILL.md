---
name: security-review
description: Perform a security audit of the repository, a crate, a diff, or a feature. Use when the user says "security review", "security audit", "threat model", "review attack surface", "check for malicious files", "harden parsers", "review unsafe/FFI", or asks whether untrusted input can crash or compromise the app.
allowed-tools: Bash(gh:*), Bash(git:*), Bash(cargo:*), Bash(cargo-deny:*), Bash(timeout:*), Bash(ulimit:*)
license: MIT
compatibility: Requires git; authenticated gh CLI only for issue/PR context; run from the repository root.
metadata:
  author: KookaPictura
  version: "1.0"
---

# Security review

Audit the code that consumes untrusted input and the boundaries around it.
Produce a prioritized, evidence-backed report. **Do not edit code or docs**
unless the user also asks for fixes. A review is a read-only task.

## 0. Guardrails

- `AGENTS.md` applies. Any `docs/` edit needs `TASK-ALLOWS-DOCS`; never touch
  `docs/` for a review.
- Use the `gh-reader` subagent for every GitHub read (issue/PR bodies, labels,
  CI status). Do not read them yourself.
- Never open a malicious file with the normal build. For dynamic checks use a
  throwaway file, a hard timeout, and resource limits:
  `timeout 10s bash -c 'ulimit -v 2000000; ./build/pictura --headless ...'`.
- Do not claim remote code execution for a panic/abort in safe Rust. Say
  "denial of service (panic/OOM/abort)". Reserve RCE for proven native or
  FFI memory corruption.
- Distinguish **verified** (source or reproduction proves it) from
  **unverified** (needs testing). Never present a guess as a finding.
- Prefer `grep`/`read` tools or `git grep`; `rg` is not installed here.
- Keep exact `file:line` references so the report is actionable.

## 1. Define the target

State what is in scope and the commit:

```bash
git status --short --branch
git log -1 --oneline
```

Typical targets: whole repo, one crate, one PR diff (`git diff base...head`),
or one feature (e.g. PDF import). For a diff, review changed code **and** the
unchanged callers/parsers it reaches.

Threat model to state up front:

- **Primary:** a malicious document the user opens (PSD/PSB, PDF/EPS, raster,
  metadata, ICC, camera-raw settings, recovery files).
- **Secondary:** a same-user process (control socket), a hostile output
  directory, CI/supply chain.
- **Out of scope by default:** a remote network service (there is none), a
  user deliberately running untrusted code, kernel/compiler bugs.

## 2. Map trust boundaries and entry points

Find every path from bytes/paths/argv/socket to a parser or native process.

```bash
git grep -n "std::fs::read\|File::open\|read_to_end\|read_to_string\|metadata()" -- crates
git grep -n "Vec::with_capacity\|vec!\[0\|resize(" -- crates
git grep -n "as usize \*\|as u32\|as i32\|checked_mul\|saturating_mul" -- crates
git grep -n "unsafe\|extern \"C\"\|as_hal\|from_raw\|from_raw_parts\|assume_init" -- crates
git grep -n "Command::new\|QProcess\|openUrl\|QLocalServer\|QTcpServer\|QTemporary" -- crates
git grep -n "RandomState\|SystemTime\|thread_rng\|rand::\|getrandom" -- crates
git grep -n "serde_json\|read_xml\|quick_xml\|roxmltree\|libloading\|dlopen" -- crates
```

List the boundaries explicitly:

| Boundary | Where | Input |
| --- | --- | --- |
| Open document | `crates/pictura-app/src/cxxqt_object/open_jobs.rs` | path + bytes |
| Import dialog | `.../pdf_export/import.rs`, `helpers.rs` | path + geometry |
| Embedded PSD in PDF | `crates/pictura-codec/src/pdf/read.rs` | PDF streams |
| Smart object | `crates/pictura-codec/src/smart_object.rs` | PSD tagged blocks |
| Control socket | `crates/pictura-app/cpp/control_server*.cpp` | JSON + paths |
| Recovery | `.../impl_core/recovery.rs` | manifest + snapshots |
| Native rasterizer | `crates/pictura-codec/src/ghostscript.rs` | PDF/EPS |
| Qt image plugins | `crates/pictura-app/cpp/decode_image.cpp` | raster bytes |

## 3. Review every parser

For each parser, trace one record end to end:

1. **Header/field read** — is the length/dimension/count trusted?
2. **Allocation** — is the size checked against remaining input *and* a budget
   before `Vec::with_capacity` / `vec![0; n]` / `resize`?
3. **Decode** — is decompression bounded? Is RLE/PackBits input bounded?
4. **Index/write** — are all indices derived from checked sizes?
5. **Termination** — recursion depth, cycles, and iteration counts bounded?
6. **Output** — does a native tool's output size feed a second allocation?

Questions to answer for every field:

- What are the max/min values? Can it be zero, negative, `i64::MAX`, `u64::MAX`,
  NaN, or a float cast to an integer?
- Does release-mode wrapping turn a huge value into a small one that passes a
  guard, then panics at an index? Does debug mode panic earlier?
- Does allocation failure abort the process (Rust's default) instead of
  returning an error?
- Can a small input expand to a huge one (zlib/deflate/RLE/zip bomb)?
- Can a self-reference or `/Kids`-style link loop forever?
- Is a post-decode budget checked **before** the decode?

Prefer a shared `ResourceBudget { max_pixels, max_bytes }` with
`try_reserve` and bounded `Read::take`; reject rather than truncate.

## 4. Arithmetic and allocation checks

Look for:

- `a * b` on `usize` where both come from the file (`width * height * 4`).
- `as usize` / `as u32` / `as i32` truncation of signed fields.
- `i32` subtraction for rect extents (`right - left`).
- `chunks(0)` / `chunks_mut(0)` after a wrapped product.
- `Vec::with_capacity(n)` from `/N`, `height`, `count`, `columns`.
- `slice[start..end]` where `start`/`end` came from the file.
- `expect`/`unwrap` on crypto key lengths or parsed spans.
- `first + offset` without `checked_add`.

The safe-Rust baseline: **no undefined behavior**, but panic/abort/OOM is still
a valid denial-of-service finding.

## 5. Unsafe, CXX-Qt, and FFI

For every `unsafe` block, state the invariant and whether caller data can
violate it:

- `GenericArray::from_slice(key)` — is the key length proven 16 or 32?
- `QImage::from_raw_bytes(data, w, h, fmt)` — is `data.len()` exactly
  `w*h*4`, are `w`/`h` non-negative, and does the format match?
- `from_raw_parts` — ownership, alignment, and lifetime.
- `assume_init` — is every bit pattern valid (e.g. zeroed `u8`)?
- `as_hal` raw handles — is the wgpu object kept alive by an owning field?
- C++ `memcpy`/`QImage` — are source/destination lengths and strides checked?
- `extern "C"` — is the FFI signature stable, and are panics caught at the
  boundary?

Cross-check the Rust buffer length against the Qt/C++ length and stride. A
mismatch is a memory-safety bug even if Rust believes it is safe.

## 6. Native and external processes

For Ghostscript, Qt image plugins, lcms2, and any future native dependency:

- What flags restrict file/network access (`-dSAFER`,
  `--permit-file-read`, sandbox)?
- Is there a timeout, memory cap, CPU cap, and output-size cap?
- Is the executable selected from `PATH`/env? Can the user be tricked into a
  different binary?
- Is the input path passed as an argv element (safe) or interpolated into a
  shell/PostScript/format string (unsafe)?
- Are temporary files owner-only, unpredictable, created with "fail if exists",
  and removed on drop?
- Does the dependency have a minimum-version policy? Rust `cargo-deny` does not
  cover native CVEs; note the distro/Qt/gs update requirement.

## 7. Filesystem, IPC, and secrets

- Path traversal: are document-derived names used as paths? `completeBaseName`
  is safer than a raw title.
- Symlink/TOCTOU: predictable `.tmp` names + `fs::write`/rename in a
  user-writable directory.
- Socket permissions and peer identity: `UserAccessOption`, file mode, token.
- Arbitrary open/save over IPC: is this an intended same-user capability?
- Secrets: passwords in argv (`ps`/`/proc`), logs, or recovery files.
- Randomness: salts/IDs/scratch names must come from an OS CSPRNG, not time or
  `SipHash`.
- Crypto: key lengths, IV reuse, padding oracles, and password normalization.

## 8. Supply chain and CI

- Read `.github/workflows/*.yml` and `deny.toml`.
- Are third-party actions pinned by full SHA? Tag-only `uses:` is a
  supply-chain risk.
- Does `pull_request_target` check out the PR head? It must not.
- Are release permissions minimal? Are artifacts signed/notarized?
- `cargo deny check advisories` for Rust; name native dependencies separately.
- New dependency? AGENTS.md rule 4 requires a justification.

## 9. Verify positives

Do not only hunt for bugs. Confirm and record the good properties, e.g.:

- No network stack, no plugin/script engine, no external linked-file reads.
- No XML parser for XMP (no XXE); no SVG import.
- Fonts only from bundled resources/system.
- Descriptor/EXIF/XMP/engine-data/pattern caps.
- Scratch-directory and socket permissions.
- Existing tests/goldens that pin the behavior.

Use `git grep` to prove absence ("no matches for `libloading|dlopen`"), not
memory.

## 10. Report format

Order findings by severity, then by exploitability. For each:

```
### [H1] Short title
Severity: high | medium | low
CWE: CWE-190 (integer overflow) / CWE-400 / CWE-674 / CWE-22 / CWE-59 ...
Where: crates/.../file.rs:123-145
Verified: yes (source/repro) | unverified (needs X)
Impact: crash/OOM/hang/read/write/native corruption
Preconditions: user opens ...; same-user process; hostile directory
Path: field -> arithmetic -> allocation -> index
Fix: concrete change (checked arithmetic, budget, cap, timeout, ...)
Test: the automated check that should prove the fix
```

End with:

- **Positives** — what is already safe and should stay.
- **Recommended order** — what to fix first.
- **Verification gaps** — what was not tested and why.
- **No code changes made** (unless fixes were requested).

Severity rubric:

- **High:** small crafted input crashes/hangs the app, or a native edge can be
  driven with untrusted data; key-length/panic on open.
- **Medium:** requires a user action/path, a huge input, or same-user access;
  native decoder exposure without a practical trigger.
- **Low:** debug-only panic, defence-in-depth, compatibility, supply-chain
  hygiene with no current exploit.

## 11. Optional dynamic checks

Only after static review, and only in a sandbox:

- `cargo test -p pictura-codec` / `cargo nextest run -p <crate> <name>`.
- A temporary fuzz crate (never commit it) with `cargo fuzz` targets for
  `read_psd`, `decode_stream`, `PdfObjects::parse`, `png::decode_rgba`,
  `editing_psd`.
- ASan/UBSan on the C++ edges for a specific repro.
- `timeout` + `ulimit -v` + a disposable `/tmp` directory for a PoC.
- Never run a corpus of untrusted files against the user's real recovery store
  or output directory.

## 12. Repo hotspots (starting points)

| Area | Files |
| --- | --- |
| PSD/PSB | `crates/pictura-codec/src/read.rs`, `common.rs`, `color_mode.rs`, `patterns.rs`, `descriptor.rs`, `engine_data.rs` |
| PDF objects/crypt | `crates/pictura-codec/src/pdf/import/objects.rs`, `pdf/crypt.rs`, `pdf/read.rs` |
| PDF render/import | `crates/pictura-codec/src/pdf/import/{mod.rs,images.rs}`, `ghostscript.rs`, `png.rs`, `eps.rs` |
| Raster import | `crates/pictura-codec/src/probe.rs`, `webp.rs`, `crates/pictura-app/src/cxxqt_object/helpers.rs`, `cpp/decode_image.cpp` |
| App/Open | `.../cxxqt_object/open_jobs.rs`, `pdf_export/import.rs`, `helpers_composite.rs` |
| IPC/FS | `.../cpp/control_server*.cpp`, `pdf_export.rs`, `export.rs`, `impl_core/recovery.rs` |
| Native/unsafe | `crates/pictura-app/src/gpu.rs`, `build.rs`, `crates/pictura-core/src/plane.rs` |
| Supply chain | `.github/workflows/*.yml`, `deny.toml`, `flatpak/*.yml` |

## 13. Definition of done

- Every in-scope boundary has a verdict (safe / finding / unverified).
- Each finding has location, severity, precondition, impact, and a concrete fix.
- Positives are verified, not assumed.
- No code or docs were changed for a review-only request.
- If the user asks to file the audit, create one tracking issue with a
  `fix(security): ...` title, link related trackers (e.g. #646, #703), and put
  the full findings in the body. Use the `gh` CLI; keep it out of `docs/`.
