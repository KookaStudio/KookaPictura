# Spec Delta

## ADDED Requirements

### Requirement: A build never waits on itself

A cache lookup SHALL NOT wait on a build that its own thread owns. A lookup that
re-enters the key it is building on the same thread, for example through a
payload whose digest collides with an enclosing one, SHALL build its own copy.
The first artifact published SHALL be the one every caller returns. Embedded
source keys SHALL include the object's nesting depth.

#### Scenario: A build re-entering its own key finishes [lsc_self_reentry]

- **WHEN** a lookup outside the worker pool builds a key and, inside that build,
  looks up the same key
- **THEN** both lookups return without waiting
