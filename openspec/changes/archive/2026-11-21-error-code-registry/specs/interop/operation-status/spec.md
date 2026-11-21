# Operation Status

## Purpose

Give operation callers stable failure identities across the Rust and Qt boundary and a defined outcome for a guarded Rust panic.

## ADDED Requirements

### Requirement: Operation status values and message keys are stable

The operation status ABI SHALL use an unsigned 32-bit value. Each published value SHALL have one canonical message key independent of locale and display text. Published values and keys SHALL remain assigned to the same condition across releases. Value 106 SHALL remain reserved.

| Value | Status | Message key |
|---|---|---|
| 0 | `OP_OK` | `operation.ok` |
| 1 | `OP_ERR_VERSION` | `error.abi.version` |
| 2 | `OP_ERR_CAPABILITY` | `error.capability.denied` |
| 3 | `OP_ERR_INVALID_ARG` | `error.argument.invalid` |
| 4 | `OP_ERR_CANCELLED` | `operation.cancelled` |
| 5 | `OP_ERR_HOST` | `error.host` |
| 6 | `OP_ERR_PANIC` | `error.panic` |
| 100 | `OP_ERR_CODEC` | `error.codec` |
| 101 | `OP_ERR_IO` | `error.io` |
| 102 | `OP_ERR_COLOR` | `error.color` |
| 103 | `OP_ERR_GPU` | `error.gpu` |
| 104 | `OP_ERR_SCRATCH` | `error.scratch` |
| 105 | `OP_ERR_SCRIPT` | `error.script` |
| 107 | `OP_ERR_LICENSE` | `error.license` |

A consumer decoding an unknown or reserved nonzero value SHALL handle it as an unrecognized failure without treating it as success or aliasing it to a known code.

#### Scenario: The initial registry round-trips at its published widths and values [os_registry_stable]

- **WHEN** each published code is written and decoded through the operation status representation
- **THEN** the representation is 32 bits and each code returns the listed numeric value and message key, with unique values and keys

#### Scenario: An older consumer receives an unrecognized status [os_unknown_status]

- **WHEN** a consumer receives the reserved value 106 or an unknown nonzero value
- **THEN** it handles the value as an unrecognized failure and preserves the received number for diagnostics

### Requirement: A guarded operation returns a structured result

A migrated Rust operation SHALL deliver its expected error category to Qt as a status separate from human-readable details. A guarded Rust unwind SHALL complete as `OP_ERR_PANIC` and SHALL NOT unwind through C or C++. Qt SHALL use the operation and status for user-facing feedback and SHALL NOT use the panic payload as its message. A user cancellation SHALL carry `OP_ERR_CANCELLED` and SHALL NOT use the failure feedback path.

#### Scenario: The guard delivers both ordinary results and caught panics [os_guard_results]

- **WHEN** operations that succeed, return a typed error, and panic are invoked through a guarded boundary
- **THEN** success yields its value, the typed error retains its category and source, the panic yields `OP_ERR_PANIC` without exposing its payload to the caller, and the next guarded call still succeeds
