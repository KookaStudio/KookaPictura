# Spec Delta

## ADDED Requirements

### Requirement: Embedded nesting is bounded

An embedded payload is untrusted input: it SHALL NOT be able to make the
renderer recurse without bound. Every smart object in a decoded embedded payload
SHALL carry its nesting depth, one more than the object whose payload holds it,
with the objects of an opened document at depth 0. That depth SHALL travel with
the decoded data, so it holds when nested work runs on another worker thread.
An object at depth 8 or deeper SHALL NOT decode its payload, and SHALL render as
any unusable source does.

#### Scenario: Nesting up to the limit renders [sor_nesting_limit]

- **WHEN** a no-proxy embedded object holds payloads nested seven wrappers deep
  around a solid source
- **THEN** the composite shows the innermost source's colour

#### Scenario: Nesting past the limit stops decoding [sor_nesting_past_limit]

- **WHEN** the same source is nested eight wrappers deep
- **THEN** the object past the limit does not paint

#### Scenario: A deep payload renders on a small worker stack [sor_nesting_small_stack]

- **WHEN** a payload nested 200 deep is composited on a two-thread pool with
  512 KiB worker stacks
- **THEN** the composite finishes without overflowing a stack
