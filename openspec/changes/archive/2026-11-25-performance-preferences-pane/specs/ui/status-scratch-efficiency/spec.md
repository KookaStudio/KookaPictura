# Spec Delta

## Purpose

The status bar can show CS6's Scratch Sizes and Efficiency readouts, read
from the memory governor and the scratch store's Efficiency counters.

## ADDED Requirements

### Requirement: The status bar offers Scratch Sizes

The status bar's readout menu SHALL offer Scratch Sizes. When chosen, the
readout SHALL show the memory Pictura uses over the memory it may use, both
in CS6's short form (`312.5M/2.04G`), and SHALL show "—" for a figure the
governor cannot read. It SHALL show without a document.

#### Scenario: Sizes are formatted like CS6 [sse_scratch_format]

- **WHEN** 327 680 000 bytes used of 2 190 433 320 are formatted, and an
  unknown ceiling
- **THEN** the text is `312.5M/2.04G`, and `312.5M/—`

#### Scenario: Choosing Scratch Sizes shows it [sse_scratch_menu]

- **WHEN** Scratch Sizes is chosen from the status bar's readout menu
- **THEN** the readout shows a used/available pair

### Requirement: The status bar offers Efficiency

The readout menu SHALL offer Efficiency. When chosen, the readout SHALL show
compute time over compute plus scratch I/O time, as a whole percentage, over
the interval since the previous reading in which time was measured. It SHALL
start at 100 % and SHALL keep its last value while nothing is measured.

#### Scenario: Efficiency covers the latest interval [sse_efficiency]

- **WHEN** readings move from (0, 0) to (900 ms compute, 100 ms I/O), then
  stay there, then move to (1000, 200)
- **THEN** the readouts are 90 %, 90 %, and 50 %

#### Scenario: Choosing Efficiency shows it [sse_efficiency_menu]

- **WHEN** Efficiency is chosen from the readout menu with nothing measured
- **THEN** the readout shows `100%`
