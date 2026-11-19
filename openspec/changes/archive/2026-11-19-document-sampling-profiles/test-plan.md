# Test Plan

Covers every scenario in the `meta/profiling-workflow` delta. This change adds
documentation and a documented build invocation; it adds no application code and
no script, so no `<script>::<fn>`, `tst_<suite>::<case>` or `<crate>::<fn>` check
exists for any row, and CI runs no second release configure to host one. Every
row therefore reads `manual:`, naming the check that applies.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| `pw_profiling_tree` | Configure `cmake -S . -B build-prof -G Ninja -DCMAKE_BUILD_TYPE=RelWithDebInfo -DBUILD_TESTING=OFF`, build with `CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`, then `readelf --debug-dump=decodedline build-prof/pictura \| grep -m1 'crates/pictura-render/src'`, the same grep for `crates/pictura-app/cpp`, and `ninja -C build-prof -t commands \| grep -- '--release'` | Both greps match a line-table range, and the cargo command carries `--release` | manual: shell checks on a locally built tree; no script exists to host them |
| `pw_shipped_unchanged` | Configure and build a throwaway tree with `-DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF` and no override; run `readelf -S build-release/pictura \| grep debug` and `ninja -C build-release -t commands \| grep -m1 pictura_app` | No debug section, and the cargo command carries neither `--profile dev-app` nor a `-C debuginfo` override | manual: requires a second full release configure, which CI does not run |
| `pw_docs_recipe` | Read the `## Profiling` section of `DEVELOPING.md` | Configure, build, and record commands appear in that order, with the debug-information override on the build command before the helper | manual: documentation review |
| `pw_docs_budget_split` | Read the subsection's build and recording commands | The build is wrapped in `scripts/cpu-budget.py exec --`; the recording is not; the affinity reason is stated | manual: documentation review |
| `pw_docs_prerequisites` | Read the subsection's prerequisites paragraph | `perf` and `samply` availability, both `kernel.perf_event_paranoid` and `CAP_PERFMON`, and which frames each permission covers are all named before the recording commands | manual: documentation review |
| `pw_frames_resolve` | On a host with `perf` or `samply`, run `perf record --call-graph dwarf ./build-prof/pictura` (or `samply record ./build-prof/pictura`) with a document loaded, apply a filter, then `perf report`; repeat against a build made without the override | The profiling build attributes hot Rust and C++ shell frames to a source path and line; the build without the override shows the symbols without those lines | manual: reviewer-run; this worktree has no `perf`/`samply` and no privilege to install them, so the row is handed over in the PR body |
