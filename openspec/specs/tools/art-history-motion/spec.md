# tools/art-history-motion Specification

## Purpose
Keeps Art History Brush motion responsive on large documents: one-pass, byte-identical stroke stamping, and a motion budget relative to the History Brush (#1238).

## Requirements

### Requirement: An Art History stroke paints its tips in one pass

Each stroke an Art History dab scatters SHALL stamp all its tips in one pass
over the rows they cover, applying the tips to each row in the order the stroke
lays them. The tip profile SHALL be derived once per stroke, and a row SHALL
evaluate only the pixels within the tip's reach of each tip centre. The
resulting pixels and the dirty rectangle SHALL be byte-identical to stamping
the tips one after another with the per-pixel tip coverage, for every style,
tip shape, opacity, flow and transparency lock.

#### Scenario: One-pass strokes match tip-by-tip stamping [ahm_one_pass_identical]

- **WHEN** a sequence of Art History dabs runs once with one-pass strokes and once with tip-by-tip stamping, over the same source, layer, options and seed
- **THEN** both leave the same pixels and report the same dirty rectangle for every dab

### Requirement: Art History motion stays within the History Brush budget

On the profiler's `art-history-textured` workload at 8000×6000 with 60 motion
events, Art History's motion p95 SHALL be within 1.25× the motion p95 of
`history-brush-textured`, measured on the same machine and build. The
comparison SHALL be relative, never a fixed number of milliseconds.

#### Scenario: The textured stroke moves within 1.25× History Brush [ahm_budget]

- **WHEN** `profile-tools.py --size 8000x6000 --gpu on --moves 60` runs `art-history-textured` and `history-brush-textured` in one app
- **THEN** Art History's motion p95 is at most 1.25× History Brush's
