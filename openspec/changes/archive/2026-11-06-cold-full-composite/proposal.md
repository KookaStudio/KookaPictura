# Proposal

## Why

Issue #1174 reports a roughly 15.65-second first full composite versus roughly
243 ms warm, but the costly operation has not been measured. Upstream #1178,
#1179, and #1180 have since changed the cold effect/styled paths and added
narrow profiles. The upstream verification record already measures the supplied
4934x1500 workload at 808-990 ms cold and 165-194 ms warm with 16 workers, so
the reported 15-second symptom appears largely mitigated. What remains is
attribution for the residual cold cost and a reproducible comparator; none of
the upstream profiles reproduces the reported nested/clipped/smart-object stack
or compares a baseline against a candidate. A runnable attribution prerequisite
is needed before selecting a further optimization without violating the
repository's compile-last rule.

## What Changes

- Scope this change to diagnostics: separate fresh-process engine rendering
  from the application's actual open, first pixel-changing full edit, and
  repeated-edit sequence, with opening and warm-region controls.
- Attribute embedded-source decoding, effects, masks, plan construction, and
  pixel execution through coarse opt-in timing, preserving existing rendering.
- Report requested and actual CPU/GPU execution, fallback, raw samples, and
  anonymous input/configuration identifiers.
- Define a deterministic baseline/candidate comparator with doubled-integer
  medians, integer noise thresholds, and distinct process exits for improved,
  regressed, inconclusive, insufficient, and invalid outcomes.
- Add a small comparator executable reading line-oriented report files, so the
  exit contract is real and testable rather than implied by a unit test.
- Use cold end-to-end time as the comparison primary: opening plus first-edit
  wall time for the application, decode plus first-full time for the engine.
  This cannot be gamed by moving work between opening and the first edit.
- Build the complete diagnostic implementation and its tests only after all
  its code is written; collect attribution after that build. No algorithm
  selection or optimization is required to finish this prerequisite.

## Capabilities

### New Capabilities

- `compositing/cold-composite-responsiveness`: diagnostic protocols and exact
  comparisons for first-composite and first-edit responsiveness, without a
  claim that the underlying latency has already been reduced.

### Modified Capabilities

None. Existing rendering, cache, invalidation, and parity contracts remain.

## Impact

- `pictura-render`: opt-in coarse phase attribution, a public `cold_profile`
  report/comparator module, a `cold_compare` executable, and engine
  profile/report tests, using existing metrics conventions and dependencies.
- `pictura-app`: a Rust application-boundary profile in
  `tests_profiles/cold.rs` using the production decode/apply/edit path and
  generated documents; no GUI flow change.
- Tests use deterministic generated content. Optional local input is consumed
  without emitting asset filenames, paths, layer names, or image content.
- No new dependency, public rendering API break, persisted format change,
  render algorithm change, or speculative core optimization is planned.

## Readiness and Follow-on

The diagnostic prerequisite is ready to implement. The optimization for #1174
is not implementation-ready and is not shipped by completing this change.
After the diagnostic build and measurements, a separate follow-on proposal
must name the confirmed costly operation, matching synthetic regression input,
exact CPU reference, scoped algorithm change, and measured acceptance target.
Keep #1174 open until that optimization is implemented and verified against
both the application sequence and its synthetic reproducer. No follow-on change
is created or implicitly authorized by this planning update.
