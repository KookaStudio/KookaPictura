# Spec Delta

## ADDED Requirements

### Requirement: CI audits oracle exclusion without throwaway builds
CI SHALL audit the normal application renderer artifact from a Cargo JSON
report emitted by the existing Corrosion `dev-app` build. The audit SHALL
verify that `test-oracle` is absent, the artifact is a normal library build, the
shared level result and constants remain importable, and the oracle imports
are absent. CI SHALL NOT start an extra Cargo build for this audit. A missing,
malformed, ambiguous, or failed report SHALL fail the headless job.

#### Scenario: The existing Corrosion build supplies the audit [vo_corrosion_reuse]
- **WHEN** `qt-headless` completes its normal Corrosion build with JSON Cargo messages enabled
- **THEN** the oracle audit consumes that build's `pictura_render` artifact without invoking Cargo again

#### Scenario: The normal artifact excludes the oracle [vo_dev_app_audit]
- **WHEN** the audit reads the `dev-app` renderer artifact
- **THEN** the oracle feature and imports are absent while the shared level result and constants remain usable

#### Scenario: A missing or invalid report fails the audit [vo_report_fails_closed]
- **WHEN** the Corrosion log has no unique successful renderer artifact or the artifact probes fail
- **THEN** the oracle audit exits non-zero and fails `qt-headless`
