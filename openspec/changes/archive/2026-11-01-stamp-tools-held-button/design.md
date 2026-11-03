# Design

## Held dabs re-dab the last sample

`DabPlacer::feed` places nothing on a zero-length segment, so re-feeding the pointer
position cannot paint. `DabPlacer::repeat_last` returns the last sample without
touching the carried residual, so path spacing after a hold is unchanged. The last
sample is already the pointer position, so the handler keeps no position of its own.
`Stroke::hold` shares the raster step with `Stroke::sample`, so jitter, Dissolve,
banded raster and per-dab engines behave the same for both.

## Rate

`airbrush-and-flow.md` requires a bounded rate and lists the value as undocumented.
Spacing is a distance and cannot supply it. The handler uses a 50 ms `QTimer`; a
timer does not queue ticks, so slow dabs lower the rate and never stack.

## Option storage

The flags live in `StampOptions`, one per tool, not in `PaintOptions`, which the
Brush shares and which would imply a Brush option that does not exist yet.

## Recording

Held dabs are not recorded. An action replays the stroke's path only, and an Art
History stroke's dab count depends on time held, so "a seed replays its stroke"
holds for the sampled dabs alone.

## App focus

Losing application focus during a press stops the timer, because the release may
never arrive. The stroke stays open until the release or a tool switch.
