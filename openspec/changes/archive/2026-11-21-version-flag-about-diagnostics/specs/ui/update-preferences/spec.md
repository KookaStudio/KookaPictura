# Spec Delta

## MODIFIED Requirements

### Requirement: The version and package method are visible for support

The About command SHALL open without a document and show the running
application version, detected package method, build type, Qt runtime version,
and active renderer. On macOS the command SHALL remain eligible for the native
Application Menu. About SHALL show the available GPU adapter name and driver in
separate fields from the active renderer. Each unavailable diagnostic field
SHALL read `Unavailable`. The System Info report SHALL include a `Package:`
line with the same method.

#### Scenario: About shows the version and method [up_about_version]

- **WHEN** the person invokes the About command
- **THEN** the dialog shows `Kooka Pictura <version>` and the package method

#### Scenario: System Info reports the package method [up_system_info_package]

- **WHEN** the person copies the System Info report
- **THEN** the report contains a `Package:` line with the detected method

#### Scenario: About shows build and runtime information without a document [up_about_runtime]

- **WHEN** the person invokes the About command with no document open
- **THEN** the dialog shows the build type, the running Qt version, and the
  active renderer

#### Scenario: About distinguishes the active renderer from the GPU adapter [up_about_gpu_adapter]

- **WHEN** adapter name and driver information are available and GPU compute
  is disabled
- **THEN** About shows the active CPU renderer along with the adapter name and
  driver in their own fields

#### Scenario: Missing adapter information is readable [up_about_gpu_unavailable]

- **WHEN** there is no usable adapter or the adapter provides only partial
  information
- **THEN** About shows the active renderer and displays `Unavailable` for each
  missing field
