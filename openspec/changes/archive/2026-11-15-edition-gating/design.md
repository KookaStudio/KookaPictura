# Design

## Edition model

`pictura_core::Edition` is a copy enum with `Standard` and `Extended`.
`Edition::from_env()` reads `PICTURA_EDITION` (case-insensitive `standard` or
`extended`) and defaults to Extended when the variable is unset or unknown.
`Feature` names the capabilities whose availability differs; `Edition::allows`
answers the gate, and `Feature::Timeline` records the CS6 correction that video
is not Extended-only.

## Catalogue policy

`pictura-script` owns the catalogues, so the policy sits next to them:
`tool_edition(name)` returns `Extended` for `count` and `Standard` otherwise;
`command_edition(id)` returns `Extended` for the 3D commands, the Measurement
Log and the Count overlay, and `Standard` otherwise. The FFI rows carry
`extended_only`, and the bridge exposes `edition_is_extended()` to the shell.

## Shell gate

`toolAvailable(id)` combines `implemented` with the edition flag. The toolbox
and `ToolController` use it for group membership, flyout entries, cycling and
`setActiveTool`. `addDefaultCommands` skips an Extended-only command in
Standard before it adds that command's separator. The Count handler stays
registered but is unreachable in Standard.

## Data safety

The gate is presentation-only. Opening and saving are unchanged, so a Standard
build still round-trips Count information and preserved 3D layer data.

## Alternatives

- A separate Standard and Extended binary: rejected because the corpus asks for
  one build that can present either edition.
- A Preferences control: deferred; `PICTURA_EDITION` is the smallest switch
  that the tests and packaging can drive, and it keeps the session store
  untouched.
- A field on every catalogue row: rejected for this change; the id/name mapping
  keeps the 500-row command table unchanged and is directly testable.
