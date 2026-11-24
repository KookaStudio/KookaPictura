# Design

## Context

The authoritative tree has 729 direct failure calls across the self-test
sources, 37 delegated propagators, and 503 `ST_BEGIN` sites. Two indexed failure
sites expand to four code values, so the ordered failure-code inventory has 731
entries. The current allocation has 527 distinct active codes after this change
adds code 555. Twenty retired codes remain reserved.

The issue's 822-site figure is historical. The current source inventory separates
token-emitting failures, delegated propagators, startup failures, and ordinary
helper returns. Only the first three carry self-test failure identity. Helper
returns such as `false`, `nullptr`, and `QString()` are predicate results, not
exit-code sites.

The reporter already merges captures by `(suite, name)`. The existing work made
`finish()` idempotent and added a run guard, but codes, suites, and fixture
ownership were still implicit.

## Goals

- Make every failure code and check name declarative and mechanically checked.
- Give every failure token the name of the check that owns its code.
- Use one lifecycle for setup, reporting, and cleanup.
- Keep green token bytes and raw failure codes stable.
- Make truncated or inconsistent captures fail the unified report.

## Non-goals

- Continue after a failed check.
- Renumber or reuse retired codes.
- Move the remaining self-test into Qt Test.
- Change pass details or their order.

## Decisions

### One append-only code registry

`selftest_codes.h` defines every active code once through an X-macro. The macro
creates named constants and a `CodeSpec` table. A separate `kRetiredCodes` array
reserves codes removed by earlier Qt Test migrations.

All 729 direct failure calls use those constants. The two `codes[i]` tables in
`selftest_paint_live.cpp` use constants too. Code 555 is the only new allocation
and belongs to the profile-open startup failure. Codes 552 through 554 stay
retired.

Thirty-six existing codes are shared by 75 sites. The registry lists every owner,
so the sharing is explicit rather than accidental. Removing those shared values
would require renumbering and would break the raw-code preservation requirement.
The guard rejects any site whose check is not a declared owner.

The guard rejects duplicate constants, duplicate values, non-positive values,
retired values, unknown check names, unused allocations, and raw integer failure
arguments.

### One declarative check registry

`selftest_checks.h` defines each check's name and suite. `SelfTestReport::begin`
looks up the suite from that table. The explicit `begin(name, suite)` overload
supports suite transitions and keeps the reporter testable without a production
check.

Existing checks declare `core`. This is deliberate. Real suite grouping works,
but changing every existing suite field would break the issue's byte-identity
requirement. A future check can declare a different suite without another
reporter change.

### Code ownership fixes the name before the token is emitted

Each allocated code lists the checks allowed to report it. The guard compares a
failure site's nearest preceding `ST_BEGIN` with that list. This catches the 186
misowned sites, including unbraced multi-check functions where a brace heuristic
alone is weak.

The fix moves each affected `ST_BEGIN` above its fixture. It does not change the
ordered pass names, pass formats, failure codes, or failure messages. A failure
at one of those sites now names its own check instead of an earlier check.

Dynamic `names[i]` and `codes[i]` pairs are checked element by element. Startup
`failRun(name, code, ...)` calls name their check directly.

### One run-scope fixture

`SelfTestRun` owns four things for the whole run. It holds the report guard,
disables interactive unsaved and transform prompts, records the active tool,
and tracks documents opened after entry. Its destructor restores the tool,
closes those documents, and lets the report guard emit the final summary.

`DocumentFixture` provides the same document cleanup for an individual seeded
document and removes its temporary PNG. The paint fixture derives from it, so
there is one document lifecycle rather than a paint-only copy.

The initial scratch document opened before `runSelfTest()` is not removed. The
run fixture only owns documents created during the run.

### Summary closes every path

`finish()` is idempotent. `SelfTestRun` installs a `RunGuard` before the first
check, so every direct or delegated return reaches exactly one summary.

Startup failures happen before a frame exists. `failRun` begins the named check,
emits the failure, and finishes the report. Both the state-directory failure and
the profile-open failure use this helper.

The process boundary returns codes in 1..255 unchanged. Larger codes return 1.
The `FAIL` token keeps the raw allocated value.

### The reporter requires a complete final summary

`validate_selftest_stream` accepts only one complete summary as the final machine
token. It rejects partial summaries, multiple summaries, result tokens after the
summary, malformed result tokens, and count mismatches.

`build_layers` applies that validation to every capture. If a summary reports a
failure but no parseable `FAIL` record exists, it adds a named failure. The
existing FAIL-wins merge and per-capture summary display remain.

### The guard runs in every gate path

`check-selftest-codes.py --self-test` plants registry, ownership, dynamic-table,
startup, and skip-truncation violations. The tree scan includes every
`selftest*.cpp` source plus `main.cpp`.

`verify-fast.sh` runs the guard in its docs-only branch and full check batch.
The guards workflow runs its self-test and tree scan.

## Risks and trade-offs

- Failure tokens at 186 sites now carry the correct check name. That is an
  intended correction, not byte identity with the broken output.
- Process statuses above 255 change to 1. Raw token codes do not change.
- The registries add two headers under 600 lines each. They replace magic values
  at the failure sites and make allocation review explicit.
- The shared run fixture closes documents created during the run. Checks that
  depended on those documents surviving into shutdown no longer do so.
- Runtime behavior is unverified in this phase. Builds, tests, and repository
  gates are deferred to the sequential verification phase.

## Migration plan

1. Add the registries and convert failure sites to named constants.
2. Move misowned `ST_BEGIN` sites and add the shared run fixture.
3. Add startup failure handling, strict reporter validation, and Qt Test cases.
4. Update the guard, CMake lists, fast gate, and guards workflow.
5. Update the testing contract and this change's test plan.
6. In the later verification phase, build once, run the planned checks, capture
   both self-test streams, and compare the green stream with the pre-change
   capture.

No on-disk document format changes. Rollback removes the registries, fixture,
and reporter changes together. Retired codes stay retired.

## Open questions

None blocking. Existing checks keep suite `core` for byte identity. New checks
can use the declarative suite field.
