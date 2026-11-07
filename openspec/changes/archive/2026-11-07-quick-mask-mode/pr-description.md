feat: implement Quick Mask mode (#172)

Adds Q/toolbox entry and exit, a temporary paintable mask, display options, Channels integration, and a red active-layer highlight. No pixel selection starts clear; black paint protects and white paint selects. Missing layer targets block painting.

Content moves/duplicates refuse while active. Selected Areas uses the inverted editing destination for blend modes, and Quick Mask colors preserve remembered image/layer-mask pairs.

Validation: CMake build, workspace fmt/clippy, and the complete unified report pass: 8,124 passed, 124 skipped, zero failed. All 250 CTest entries pass; repository and test-plan guards pass.

Reviewer checklist:
- [ ] Where a lawful CS6 runtime is available, compare entry/exit, history steps, native-depth temporary-channel appearance, and filter availability. Record observations separately; project-contract tests do not constitute an independent CS6 oracle.

Coverage remains 8-bit and unsupported grayscale tools refuse. The command-table golden files `crates/pictura-script/tests/golden/catalogue.tsv` and `const_names.tsv` each gain exactly the new Quick Mask entry. All existing rows and assertions are preserved.
