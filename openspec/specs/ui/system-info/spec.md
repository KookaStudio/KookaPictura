# ui/system-info Specification

## Purpose
The Help menu's environment report, its clipboard and issue-submission paths,
and the Linux/Qt/wgpu data it presents.

## Requirements

### Requirement: Help exposes System Info and Submit Issue

The Help menu SHALL present an enabled `System Info` entry and an enabled
`Submit Issue` entry directly below it. `System Info` SHALL open the report
dialog; `Submit Issue` SHALL open the repository's new-issue page with the
report in the body. Neither command SHALL require an open document.

#### Scenario: System Info is wired in the Help menu [sysinfo_menu]

- **WHEN** `tst_system_info::helpMenuWiresSystemInfo` constructs the main
  window with no document and dispatches `help.system.info`
- **THEN** the command is enabled, a `SystemInfoDialog` is visible, and the
  report text is non-empty

#### Scenario: Submit Issue is wired below System Info [sysinfo_submit_menu]

- **WHEN** `tst_system_info::helpMenuWiresSubmitIssue` inspects the Help menu
- **THEN** `Submit Issue` is enabled, follows `System Info`, and has a handler

### Requirement: The dialog reports the application and machine environment

The dialog SHALL show a plain-text report with `Application`, `System`, `CPU`,
`Memory`, `GPU`, and `Displays` sections. The report SHALL include the Pictura
version and build type, Qt version, OS type and version, kernel, architecture,
desktop and session, CPU model, physical and logical core counts, CPU clock,
RAM and swap totals and availability, system and process CPU usage, Pictura's
RAM, the GPU adapter name/type/backend/driver, and every connected display's
name/model, resolution, physical size, DPI, and refresh rate.

#### Scenario: The report has every section and value [sysinfo_sections]

- **WHEN** `tst_system_info::reportHasAllSections` formats a fixture with every
  field populated
- **THEN** the report contains each section heading and each fixture value

#### Scenario: The GPU fields come from the renderer [sysinfo_gpu]

- **WHEN** `pictura_render::gpu_adapter_info_never_panics` asks for the adapter
  summary and `tst_system_info::reportHasGpuFields` formats a fixture with
  adapter fields
- **THEN** the summary is either `None` or has a non-empty adapter name, and the
  report shows the adapter name, type, backend, and driver

### Requirement: Missing metrics render as Unavailable

A metric the platform does not expose or a read that fails SHALL render
`Unavailable`; it SHALL NOT print a blank value, crash, or omit the row.

#### Scenario: Missing fields fall back to Unavailable [sysinfo_unavailable]

- **WHEN** `tst_system_info::reportUsesUnavailable` formats a fixture whose
  optional fields are empty
- **THEN** every corresponding row reads `Unavailable` and the report still has
  every section

#### Scenario: Linux CPU and memory parsing is best-effort [sysinfo_proc_cpu]

- **WHEN** `tst_system_info::parsesCpuInfo` parses representative `/proc/cpuinfo`
  text with two physical cores and a clock
- **THEN** the model, two physical cores, and the clock are reported

#### Scenario: Memory fields parse from /proc/meminfo [sysinfo_proc_memory]

- **WHEN** `tst_system_info::parsesMemInfo` parses representative
  `/proc/meminfo` text
- **THEN** `MemTotal`, `MemAvailable`, `SwapTotal`, and `SwapFree` are converted
  to bytes

#### Scenario: Byte sizes are human-readable [sysinfo_bytes]

- **WHEN** `tst_system_info::formatsBytes` formats byte counts across the
  kilobyte, megabyte, and gigabyte ranges
- **THEN** each result carries one decimal place and the correct unit

### Requirement: The report is a refreshable snapshot

Opening `System Info` SHALL collect a snapshot at that moment. Reopening or
refreshing the dialog SHALL replace the report with a new snapshot.

#### Scenario: Reopening refreshes the text [sysinfo_snapshot]

- **WHEN** `tst_system_info::reopenRefreshesReport` sets a different fixture on
  an open dialog
- **THEN** the dialog text and its `reportForTest()` value change to the new
  report

### Requirement: Copy places the report on the clipboard

The dialog SHALL provide a `Copy` button that places the exact report text on
the system clipboard.

#### Scenario: Copy matches the displayed report [sysinfo_copy]

- **WHEN** `tst_system_info::dialogCopiesReport` clicks the dialog's `Copy`
  button
- **THEN** the clipboard text equals the dialog's report text

### Requirement: Submit Issue prefills a GitHub issue

`Submit Issue` SHALL build a `https://github.com/KookaStudio/KookaPictura/
issues/new` URL whose `body` query parameter contains the report under a
`System Info` heading. The body SHALL be capped at 6000 characters with an
explicit truncation marker, and the full report SHALL be copied to the
clipboard before the browser is opened. A failed browser launch SHALL show a
message naming the clipboard fallback.

#### Scenario: The URL carries the report [sysinfo_submit]

- **WHEN** `tst_system_info::submitUrlEncodesReport` builds the issue URL for a
  report
- **THEN** the URL is the repository's `issues/new` page and its decoded body
  contains the report inside a fenced block

#### Scenario: An over-long report is capped [sysinfo_submit_cap]

- **WHEN** `tst_system_info::submitUrlCapsLongReport` builds the URL for a
  report longer than 6000 characters
- **THEN** the decoded body is at most 6000 characters and ends with the
  truncation marker

### Requirement: The report excludes identifying data

The collector SHALL NOT include the hostname, user name, monitor serial
numbers, network addresses, environment variables beyond the desktop/session
hints, or document paths.

#### Scenario: Host and user are absent [sysinfo_privacy]

- **WHEN** `tst_system_info::collectorOmitsHostAndUser` collects a live report
- **THEN** it does not contain the machine hostname or the `USER`/`USERNAME`
  environment value
