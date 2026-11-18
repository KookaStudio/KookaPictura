

## REMOVED Requirements

### Requirement: Paint present-tiling self-check

**Reason**: Whole-pyramid rebuild counts and mandatory legacy self-test growth no longer describe the production path.

**Migration**: Use Paint tile-refresh verification; existing scenario tags remain traceable in the replacement.

## ADDED Requirements

### Requirement: Paint tile-refresh verification

Automated paint checks SHALL apply several dabs between two presents and assert
that pending tile damage is refreshed at the frame flush rather than once per
dab. The checks SHALL compare exact tiles with an independent whole rebuild and
committed pixels with the same stroke committed through the CPU path. GUI checks
SHALL live in Qt Test, and engine checks SHALL use the native unit-test runner;
this contract SHALL NOT require a new legacy self-test entry or exit code.

#### Scenario: The frame tile refresh is observed [vh_pyramid_selfcheck]

- **WHEN** the paint present-tiling self-check runs
- **THEN** its counters show one pending-damage refresh at the flush rather than a refresh per queued dab

#### Scenario: The committed pixels match the CPU path [vh_pyramid_commit]

- **WHEN** the same stroke is committed on the checked path and on the CPU path
- **THEN** their committed images are byte-identical

#### Scenario: The collapsed commit blit equals the level-0 crop [vh_commit_blit]

- **WHEN** the unit check commits a single-region stroke and compares its blit
  image with the level-0 crop of the same rectangle
- **THEN** every pixel channel is identical
