# Proposal

Tracking issue: #1126.

## Why

`stamp_agreed` gives every tracked plane a fresh stamp on each history capture
and restore. GPU residency keys are built from those stamps, so an unchanged
layer misses after every committed edit and is uploaded again. On a 16507×16196
document with GPU compute on, repeated moves of a full-size layer recorded no
residency hits and seven misses.

## What Changes

- `stamp_agreed` keeps a stamp both sides already agree on, and issues a fresh
  one for a written plane or a repeated stamp.

## Capabilities

### New Capabilities

- `document/history-plane-identity`

## Impact

`pictura-app` history only. No format, golden or document behavior change.
