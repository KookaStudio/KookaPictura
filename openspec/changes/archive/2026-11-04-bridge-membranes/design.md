# Design

## Context

See `proposal.md` for why a literal single bridge module is not viable. The
constraints that shape this design:

- `cxx-qt-build 0.10` allows one bridge module per file and generates one header
  per module: 82 bridge files generate 82 headers, 81 of which are consumed.
  `pictura_app/src/cxxqt_object/impl_paint/tool_recording.cxxqt.h` has no C++
  consumer.
- `scripts/file-size-allowlist.txt` only shrinks; `cxxqt_object.rs` is at 1,204.
- The CMake bridge stamp is currently attached by scanning each `.cpp` for
  `pictura_app/src/`, which no longer matches once callers include membranes.
- The existing `ui/view-seam-adapters` contract already allows named deferrals;
  this change removes that allowance for generated headers, except the type
  adapter's own header (`cpp/api/type_api.h`), which the existing type-adapter
  requirement and `type_bridge_headers_stay_in_adapter` test keep as the only
  surface over `type_tools*.cxxqt.h`.

Measured consumer fan-out (include sites from the current tree):

```
god header (cxxqt_object)        274 include sites
layer_target_bridge               32
filter_tools                      71 (44 parent + 27 submodules)
impl_layers/layer_masks           34
paint_tools/fills                 30
```

These are per-translation-unit parse surfaces, not rebuild fan-out. The single
global bridge stamp already recompiles every generated-header consumer on any
bridge edit, and this change keeps that behaviour.

## Goals / Non-Goals

**Goals:**

- Every consumed generated header is reachable through exactly one membrane or
  adapter header under `cpp/api/`; the type bridge headers are owned by
  `cpp/api/type_api.h`; a generated header with no consumer is exempt.
- Bridge edits still recompile their consumers in the same build (global stamp).
- The bridges that expose at least one `extern "Rust"`/`extern "RustQt"`
  function and two or fewer are folded into neighbours, leaving 69 bridge
  files; import-only bridges are exempt.
- A shrink-only check prevents the seam from drifting back.

**Non-Goals:**

- One Rust `#[cxx_qt::bridge]` module (blocked by cxx-qt and the size policy).
- One universal umbrella header (regresses the per-translation-unit parse
  surface; it does not change rebuild fan-out).
- Routing every verb through a typed adapter (most domains have nothing to
  absorb; #276 forbids 1:1 forwarders).
- A generated verb manifest for "registration in one place" (deferred; a
  deliberate narrowing of #1160 that needs maintainer sign-off).
- Any runtime behavior change.
- Per-membrane stamps or any change to the global rebuild fan-out.

## Decisions

### Decision: domain membranes under `cpp/api/`, not one umbrella

Group the generated headers by **consumer co-inclusion** into membranes such as
`document_bridge.h`, `layers_bridge.h`, `paint_bridge.h`, `filters_bridge.h`,
`selection_bridge.h`, `transform_bridge.h`, `engine_bridge.h`, and a small
`misc_bridge.h`. The type domain does not get a membrane: `cpp/api/type_api.h`
remains its generated-header surface, as the existing type requirement and
`type_bridge_headers_stay_in_adapter` require. The exact split comes from a
throwaway script that counts which headers are included together; the names
above are the expected shape, not a fixed contract.

A membrane is an include-only header: `#pragma once` (or an include guard),
includes, and a comment naming the domain. No function bodies, no state, no
wrappers.

Membrane map (each consumed generated header appears in exactly one membrane or
in `cpp/api/type_api.h`):

| Membrane | Generated headers |
|---|---|
| `view_bridge.h` | `cxxqt_object` |
| `document_bridge.h` | `guides`, `image_adjust/image_mode`, `image_adjust/image_size`, `impl_core/recovery`, `impl_core/save_jobs`, `impl_history/purge`, `open_jobs` |
| `layers_bridge.h` | `adjustment_edit`, `adjustment_presets`, `channels`, `clipping`, `impl_layers/*`, `layer_arrange`, `layer_style`, `layer_target_bridge`, `layers_smart_filters`, `path_list`, `paths`, `shapes` |
| `paint_bridge.h` | `annotations`, `healing`, `magnetic`, `paint_tools`, `paint_tools/fills` |
| `filters_bridge.h` | `auto_blend`, `filter_tools*`, `image_adjust`, `image_adjust/gradient_map`, `image_adjust/image_ops`, `image_adjust/jobs`, `image_apply_image`, `image_match_color`, `image_replace_color` |
| `selection_bridge.h` | `color_range`, `impl_selection/bridge`, `impl_selection/refine` |
| `transform_bridge.h` | `align`, `crop_group`, `impl_transform/bridge`, `impl_transform/warp` |
| `engine_bridge.h` | `actions_engine`, `clipboard`, `color_management`, `engine_tables`, `export`, `pdf_export`, `pdf_export/import`, `web_export`, `decode_bridge`, `gpu_probe_bridge` |
| `cpp/api/type_api.h` | `type_tools`, `type_tools/fonts`, `type_tools/warp` |

`impl_paint/tool_recording.cxxqt.h` has no C++ consumer and is exempt.

**Alternatives rejected:** one `pictura_bridge.h` (every TU parses every
generated header); one bridge module (policy conflict); typed adapters for every
domain (pass-through adapters).

### Decision: fold the small bridges into domain neighbours

For each bridge that exposes at least one `extern "Rust"`/`extern "RustQt"`
function and two or fewer, move its declarations into the neighbour that
already owns the domain, preserving each item's `#[namespace]` and every
signature, delete the standalone bridge, and drop it from `build.rs`. Every
folded bridge also owns a `mod imp;` and an `imp/` directory; the fold must
re-home that module (or keep the file as a plain, non-bridge module) and add the
`mod`/`use` wiring to the neighbour. The spike checks struct and function-name
collisions, per-target namespaces, and that each target stays under the
1,204-line cap (do not fold into `cxxqt_object.rs` if it would exceed).

`decode_bridge::copy_bytes` is the one unnamespaced symbol: keep it unnamespaced
or update the four `::copy_bytes` call sites. The other folded symbols are
`pictura::`-qualified and keep their names.

Fold map (the target in parentheses receives the declarations):

| Small bridge | Target |
|---|---|
| `engine_tables/{adjustment_catalogue,blend_modes,tool_catalogue}` | `engine_tables.rs` |
| `command_catalogue` | `engine_tables.rs` |
| `engine_tables/color_models` (not small; moved for the header name) | `engine_tables.rs` |
| `hdr_conversion`, `image_hdr_toning` | `image_adjust.rs` |
| `mcp_entry`, `svg_export` | `decode_bridge.rs` |
| `eyedropper_sample` | `paint_tools.rs` |
| `image_adjust/new_document` | `image_adjust/image_size.rs` |
| `impl_history/display` | `impl_history/purge.rs` |
| `layer_path` | `paths.rs` |
| `save_format` | `export.rs` |

**Alternative rejected:** keeping them and only hiding their headers; the issue
explicitly asks for them gone.

### Decision: attach the bridge stamp to every C++ translation unit

Change `pictura_depend_on_bridge_stamp` to attach the stamp to every shell and
test translation unit, not just the ones whose source text names a generated
header. After the migration a unit can reach a generated header through a
header (`frame_includes.h`, `paint_tip.h`, ...), so a per-source string scan
would miss consumers; the global stamp already rebuilds them together, and
rebuilding too much is the accepted fallback. List the membranes in
`PICTURA_SHELL_SOURCES` so CMake tracks them. This keeps the adapter
translation units (`cpp/api/*_api.cpp`, `cpp/api/type_api.h`) and the
transitive-header consumers on the same-build guarantee. The stamp is still
global: any bridge edit recompiles every consumer, as it does today.

**Alternative rejected:** per-membrane stamps; they would change rebuild
behaviour and the spec does not require it.

### Decision: enforce the seam with Rust tests in `build_config.rs`

Add tests alongside the existing bridge-shape tests:

- no `#include "*.cxxqt.h"` outside `cpp/api/` (production and tests), except
  the type bridge headers governed by the existing type-adapter requirement;
- every membrane is include-only (with `#pragma once` allowed) and the membrane
  set is non-empty;
- no membrane includes every generated header;
- bridge-file count `<= 69` (post-fold), and `cxxqt_object.rs` `<= 1204` lines
  (the latter duplicates `check-file-size.sh` only as a ratchet assertion; the
  cap itself stays there);
- no bridge file that exposes at least one Rust/RustQt function exposes two or
  fewer; import-only bridges are exempt.

`verify-fast` already runs `cargo nextest`, so no new script or CI job is
needed.

## Risks / Trade-offs

- **[A membrane grows into a universal header]** -> The co-inclusion spike picks
  the split; a test asserts the membrane set is non-empty and no membrane
  includes every generated header. Split a membrane when its per-TU parse
  surface grows.
- **[Bridge edits stop rebuilding C++]** -> The stamp dependency moves to the
  membranes and still-named generated headers in the same change that swaps
  includes; adapter TUs keep the guarantee; `bp_bridge_same_build` covers it.
- **[Folding changes a generated symbol]** -> Symbols are `pictura::`-qualified
  except `decode_bridge::copy_bytes`, which is kept unnamespaced or has its four
  call sites updated. The fold is a move of declarations plus the `imp`
  re-homing, verified by an unchanged build and Qt suites.
- **[A folded bridge's struct or function name collides with the target]** ->
  The spike lists every struct and `extern` function in the folded files;
  rename nothing, and if a collision exists, fold into a different neighbour.
- **[Include-swap misses a file]** -> The new confinement test scans the whole
  tree, including `cpp/tests/`, production headers, and `frame_includes.h`.

## Migration Plan

1. Spike: script the co-inclusion table, choose the membranes, list the small
   bridges and their target neighbours, and verify namespaces, collisions,
   `imp` modules, and target file sizes. No production edits.
2. Add the membranes, each including the current generated headers. Build.
3. Swap production includes (194 files, including headers). Build after each
   domain.
4. Swap test includes (154 files, including test headers) and
   `frame_includes.h`. Build.
5. Fold the small bridges into neighbours; re-home their `imp` modules; update
   `build.rs` and the membranes; build.
6. Attach the stamp dependency to every C++ translation unit; keep the
   confinement tests green.
7. Run the full gate: `bash scripts/verify-full.sh` (CMake build + verify-fast),
   `openspec validate --all --strict`, and archive the change.

Each step is an include rename or a declaration move plus `imp` re-homing, and
can be reverted independently.

## Open Questions

- The exact membrane count and membership are decided by the co-inclusion
  spike; the spec only requires that membranes are domain-scoped and bounded.
- Whether the deferred verb manifest is worth a follow-up is decided after this
  change lands and the seam is measured; it needs maintainer sign-off to narrow
  #1160.

## Measurements

On the development build (`RelWithDebInfo`, `-j 12`):

- A bridge-module edit (`layer_style.rs`, one comment line inside the bridge
  module) recompiled **595** C++ objects; reverting it recompiled **595** again.
- An implementation-body edit (`layer_style/imp.rs`, one comment line outside
  the bridge module) recompiled **0** C++ objects; reverting it also recompiled
  **0**.
- The stamp-fallback case (an unparsable bridge module hashed whole) was not
  exercised; the extraction logic is covered by
  `bridge_modules_end_at_column_zero`, and the CMake fallback remains
  unverified.
