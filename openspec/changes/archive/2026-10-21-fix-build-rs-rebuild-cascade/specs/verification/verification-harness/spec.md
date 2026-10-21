# Spec Delta

## ADDED Requirements

### Requirement: The local gates detect a stale build tree

The local verification gates SHALL detect a build tree that is older than the
sources it is asked to test. The full gate SHALL fail when the application
binary in the build tree predates a tracked source under the application
crate, or when a CTest-registered suite has no executable in the tree, and
SHALL name the file or the suite that proves the staleness. The fast gate
SHALL report the same staleness, because it runs the same tests against the
same tree.

#### Scenario: a stale tree fails the full gate [vh_stale_tree]

- **WHEN** the application binary in the build tree predates a tracked source under the application crate
- **THEN** the full gate fails and names the source that is newer than the binary

#### Scenario: a missing suite executable fails the full gate [vh_missing_suite_binary]

- **WHEN** a CTest-registered suite has no executable in the build tree
- **THEN** the full gate fails and names the suite
