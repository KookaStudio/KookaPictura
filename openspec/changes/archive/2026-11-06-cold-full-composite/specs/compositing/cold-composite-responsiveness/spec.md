# Spec Delta

## Purpose

Provides reproducible first-composite and application first-edit diagnostics,
with exact rendering and deterministic comparison outcomes for later optimization.

## ADDED Requirements

### Requirement: Engine-cold and warm rendering are reported independently

The opt-in engine profile SHALL start each sample/backend in a fresh process,
decode a document before any rendering preparation, and report embedded-source
decode, first full composite, repeated full composites, and bounded region
timings separately. It SHALL distinguish embedded-source decoding, effects,
masks, plan construction, and pixel execution when they execute. Nested phases
MUST be labelled as nested; absent phases MUST differ from measured zero.
Digests MUST be outside timers and MUST be testable through a deterministic
digest hook that excludes digest work from the measured span. GPU device
initialization SHALL be timed inside the first-full span and labelled as a
sub-phase, or explicitly pre-warmed and the first full labelled warm-GPU.

Engine comparisons SHALL use cold end-to-end time (decode plus first full
composite, metrics off) as the primary metric, with repeated-full median and
warm-region time as controls. Decode and first-full time SHALL still be reported
separately.

#### Scenario: The first render is not hidden by warm samples [cfc_profile]

- **WHEN** the engine profile renders a generated layered document first,
  repeatedly, and over a bounded region
- **THEN** it retains a separate first-render value and raw warm/region values,
  phase presence and nesting, a cold end-to-end primary, and output digests
  without selecting a warm minimum

### Requirement: Application opening and first edit use the production sequence

The application profile SHALL use the production document decode/apply path,
including opening compositing when it is performed, target/history/document
preparation, then a real pixel-changing full edit and repeated equivalent
pixel-changing edits. It MUST preserve caches between open and edits and report
complete opening wall time, opening-composite time when performed, first-edit
wall time, first-edit full-composite phase time, repeated full edits, and warm
region refresh separately. Decode alone MUST NOT be labelled opening. An
engine-cold sample MUST NOT be substituted for an application sample.

The first-edit boundary SHALL include the composite store and presentation
reset, or SHALL report presentation reset as a separate metric. The measured
edit recipe MUST be a fixed direction: an unmeasured reset to the starting
state, then a measured pixel-changing edit to the alternate state. Direction
MUST be recorded, and samples of opposite directions MUST NOT be combined into
one median. The opening-composite assertion SHALL use an 8-bit generated
fixture with no retained source planes, or SHALL report whether opening
composited when the production path skips it.

Application comparisons SHALL use cold end-to-end time (opening wall plus
first-edit wall, metrics off) as the primary metric, with repeated-full-edit
median and warm-region time as controls. Opening, first-edit, and the
first-edit full-composite phase SHALL still be reported separately.

#### Scenario: The application first edit follows a complete open [cfc_app_sequence]

- **WHEN** a fresh application-profile child opens a generated layered document,
  makes a visible full edit, repeats equivalent edits in a fixed direction, and
  refreshes a region
- **THEN** the open interval includes production preparation, the first edit
  changes pixels and runs exactly one full composite, and all opening/edit/region
  timings, the cold end-to-end primary, and resulting-state digests are reported
  separately without cache purges

### Requirement: Backend and local-input reporting are honest and anonymous

Profiles SHALL report requested and executed backends and fallback reasons.
A fallback MUST NOT be labelled GPU execution. The executed backend MUST be
captured from the composite result or an equivalent execution observation, not
inferred from the requested configuration.

Optional explicitly requested local input SHALL be supplied only through an
environment variable, use the application protocol, and emit no asset names,
paths, layer names, or pixels, including failures. An unset variable SHALL use
a generated document. A set-but-missing or incompatible input MUST fail
explicitly with a redacted diagnostic rather than substitute a generated
document or skip. A compatible edit target SHALL be the topmost visible,
unlocked pixel layer with opacity below 255 and nonzero area; any other
selection is incompatible.

#### Scenario: A requested GPU falls back [cfc_backend]

- **WHEN** a GPU request is unsupported or no device is available
- **THEN** its report identifies the decline/unavailability, the executed CPU
  backend, and the fallback reason, and retains a separately collected
  forced-CPU result

#### Scenario: Local-input failures are redacted [cfc_private_input]

- **WHEN** an explicitly supplied local input is missing or incompatible, or
  its profile child fails
- **THEN** the profile fails with a generic diagnostic and no input identifiers,
  does not substitute a synthetic result, and preserves no input content in reports

### Requirement: Timing comparison uses a deterministic rule

A comparison SHALL match protocol/version, input id, edit recipe, machine,
build configuration, thread count, requested and executed backend, and metrics
mode; revisions are reported but may differ. It SHALL require at least three
independent fresh-process samples per revision for the primary metric and every
required control. Timings SHALL be finite nonnegative integer nanoseconds.
Missing metrics, mismatched configuration, and malformed samples are invalid;
fewer than three otherwise valid samples is insufficient. Both MUST fail the
comparison without an improvement claim.

For each metric, let B and C be baseline/candidate sample sets and rB and rC
their max-minus-min ranges. Let mB2 and mC2 be doubled medians: for an even
sample count the sum of the two middle samples, and for an odd count twice the
middle sample. Let V = max(rB, rC), in integer nanoseconds. A batch is noisy if
20 * rB > mB2 or 20 * rC > mC2 for any required metric; a zero doubled median
is noisy unless its range is also zero. No sample may be discarded.

After rejecting invalid or insufficient input, noisy batches SHALL be
inconclusive. Otherwise, mC2 - mB2 > 2 * V in the primary or any control is
regressed; mB2 - mC2 > 2 * V in the primary with no regressed control is
improved; all other results are inconclusive. Equality is not improvement.
Output SHALL include all raw samples, ranges, doubled medians, V, and the
reason.

Only improved succeeds in comparison mode. Exit codes SHALL be distinct:
0 improved, 2 regressed, 3 inconclusive, 4 insufficient, 5 invalid, and
1 usage or I/O failure. Application and engine comparisons use the primary and
controls defined in their requirements. A process contributes one value per
metric; within-process repeats contribute their median, not extra samples.

#### Scenario: Clear improvement and regression have fixed outcomes [cfc_compare]

- **WHEN** valid non-noisy matched reports have a primary reduction greater than
  2 * V in doubled-median terms and no regressed control, or an increase greater
  than 2 * V in a required metric
- **THEN** comparison reports improved for the former and regressed for the
  latter, with the raw evidence and the distinct 0 / 2 exits

#### Scenario: Invalid or insufficient samples cannot pass [cfc_compare_invalid]

- **WHEN** reports contain malformed/missing metrics, mismatched cohorts, or
  fewer than three otherwise valid independent samples per metric/revision
- **THEN** comparison reports invalid or insufficient respectively with the
  reason and the distinct 5 / 4 exits, without an improvement claim

#### Scenario: Noise and overlapping changes are inconclusive [cfc_compare_inconclusive]

- **WHEN** a required metric exceeds the integer noise rule, the primary
  reduction is at most 2 * V with no primary or control regression, or the
  primary doubled medians and ranges are identical with no regressed control
- **THEN** comparison reports inconclusive with the reason and the distinct 3
  exit, retaining every raw sample rather than filtering or silently retrying

### Requirement: Diagnostics preserve rendering and remain inert when disabled

Diagnostic instrumentation SHALL change no CPU composite or saved sample.
Detailed timing state MUST NOT be retained when metrics are off. Existing
hidden-layer suppression, source invalidation, and GPU parity limits MUST remain.
This diagnostic capability does not promise a reduced first-render latency;
optimization requires a later change based on the collected evidence.

#### Scenario: Instrumentation changes no output [cfc_exact]

- **WHEN** a generated nested/clipped/masked/styled scene is opened, edited,
  rendered full and by region, and saved with diagnostic metrics on and off
- **THEN** CPU composite and saved samples are identical, region samples match
  full slices, hidden content remains unprepared, and metrics-off retains no
  detailed phase state
