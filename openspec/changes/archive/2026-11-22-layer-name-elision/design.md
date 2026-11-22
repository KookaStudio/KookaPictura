# Design

## Problem

`LayerRowDelegate::nameRect` right-elides the name to the available label band
and returns a rect whose width is the elided text's advance. `paint()` then
calls `elidedText` a second time on the full name, using `nameRect`'s width as
the limit.

For a name that fits, the first pass returns the full name and the width is its
exact advance. `QFontMetrics::elidedText` compares the text's advance to the
limit with layout rounding and decides the full name does not fit, so it returns
a shorter prefix plus an ellipsis. Examples from Qt 6.11:

| Name | First pass | Width | Second pass |
|---|---|---|---|
| `r1 r2` | `r1 r2` | 20 | `r1 …` |
| `f2-1` | `f2-1` | 16 | `f2…` |
| `f2` | `f2` | 8 | `…` |

The result is a visibly truncated name with unused row space.

## Fix

Elide once. `nameRect` gains an optional `QString* drawnText` out-parameter that
receives the text it already computed. Both paint paths (regular and synthetic
rows) draw that string. The rect width remains the elided text's advance, so the
`#896` rename hit area still stops at the painted label and rendering and
hit-testing share one elision.

This is deliberately a single-elision change: an overlong name still elides to
the band and ends in an ellipsis. Only the accidental second pass is removed.
