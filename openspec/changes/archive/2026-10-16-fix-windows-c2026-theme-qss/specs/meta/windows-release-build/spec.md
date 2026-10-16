# Spec Delta

## ADDED Requirements

### Requirement: The C++ shell compiles within MSVC's per-literal string-literal limit

The C++ shell SHALL compile under MSVC without a `C2026` (string too big) error.
MSVC caps a single string-literal token at 16,380 single-byte characters,
applied before adjacent literals are concatenated. To stay below that cap, no
string-literal token in `crates/pictura-app/cpp` SHALL exceed 16,000 bytes. Any
embedded text larger than the limit, such as the theme stylesheet, SHALL be
delivered as multiple literals that are joined after compilation rather than as
one oversized literal. The assembled text SHALL be byte-identical to the
single-literal form.

The repository SHALL guard this limit in ordinary CI: a check SHALL fail when a
string-literal token under `crates/pictura-app/cpp` exceeds 16,000 bytes, so the
breakage is caught without a Windows runner.

#### Scenario: The stylesheet compiles on Windows

- **WHEN** the Windows build compiles the C++ shell
- **THEN** no translation unit fails with `C2026`, and the theme stylesheet
  assembled from its literals is byte-identical to the previous single-literal
  form

#### Scenario: An over-limit literal fails the guard

- **WHEN** a shell source under `crates/pictura-app/cpp` contains a single
  string-literal token longer than 16,000 bytes
- **THEN** the repository guard exits non-zero and names the file, the line, and
  the literal's length
