# Edition gating (Standard/Extended)

## Why

`docs/00-overview/cs6-editions-and-constraints.md` (`OVR-002`) makes the
Standard/Extended split part of the CS6 contract, but `crates/` has no
`Edition` type: the Count tool and the 3D/measurement command stubs are
reachable in every build (`docs/dev/doc-code-parity-gaps.md` OVR-2/XC-10,
#650). The project targets Standard parity plus the Extended-only Count tool,
so one build must be able to present either edition's surface without touching
document data.

## What changes

- Add `pictura_core::Edition` (`Standard | Extended`) and `Feature`, selected by
  `PICTURA_EDITION` and defaulting to Extended.
- Mark the Extended-only catalogue rows: the Count tool, the 3D commands, the
  Measurement Log and the Count overlay.
- Hide those tools and commands in Standard, in the toolbox and the command
  registry. The Timeline stays, because CS6 ships it in both editions.
- Keep the gate presentation-only: Count groups and preserved 3D layer data
  still round-trip.
- Add Rust and Qt tests, and update the docs/ROADMAP status.

## Capabilities

### New Capabilities

- `ui/edition-gating`: edition selection and the Standard/Extended surface gate.

### Modified Capabilities

- None.
