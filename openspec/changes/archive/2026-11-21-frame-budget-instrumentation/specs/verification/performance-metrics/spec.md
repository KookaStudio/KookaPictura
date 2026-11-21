# Spec Delta

## ADDED Requirements

### Requirement: Canvas frame timing measures through paint completion

When `PICTURA_METRICS` is on, each completed document canvas paint SHALL
produce one record with boundary `canvas_paint`. This includes paints caused
by panning, zooming, rotating, and the arrival of visible tiles. Explicit
control screenshot renders SHALL produce no record and SHALL NOT consume pending
input or GPU samples or advance the canvas cadence baseline. Each record
SHALL identify the canvas and report the count and summed duration of pending
viewport input operations handled for that canvas before the paint. Input
operations coalesced into one paint SHALL be counted once each. A paint with
no pending viewport input SHALL report zero input operations and zero input
time.

The record SHALL report elapsed paint time, time spent gathering and assembling
visible tiles during the paint, and the remaining Qt draw time. Tile gather
time SHALL include synchronous crop and cache work and exclude asynchronous
work outside the paint. Qt draw time SHALL equal paint time minus tile gather
time. Signed CPU budget slack SHALL equal 16.6 ms minus input time and paint
time. Paint-to-paint interval SHALL measure the time since the previous
recorded completion on the same canvas and SHALL be absent for the first paint
of a capture. Display presentation time SHALL be marked unavailable.

#### Scenario: Viewport input coalesces into one timed paint [fbi_coalesced_input]

- **WHEN** metrics are on and several pan, zoom, or rotate operations update
  the same document canvas before Qt handles one paint
- **THEN** that paint produces one record containing those operations, with CPU
  phase durations and signed budget slack consistent with the measured boundary

#### Scenario: Paint cadence belongs to the canvas [fbi_cadence]

- **WHEN** two canvases paint in alternation and one paints again with no new input
- **THEN** its next record measures the interval from its own prior paint,
  reports zero input, and marks display presentation time unavailable

#### Scenario: Late tiles produce a separate paint [fbi_late_tiles]

- **WHEN** a viewport update paints while needed tiles are building asynchronously,
  and the completed tiles trigger a later paint
- **THEN** the first record excludes the worker's time, and the later paint has
  its own record without counting the viewport input again

#### Scenario: Screenshot renders do not consume a frame [fbi_screenshot]

- **WHEN** a control screenshot grabs either the frame or canvas while input and
  a GPU sample are pending
- **THEN** its offscreen paint produces no frame and changes no cadence baseline,
  and the next eligible canvas paint consumes the pending timings

### Requirement: GPU composite duration follows the painted result

An optional frame device duration SHALL report timestamped GPU composite
work whose result was newly applied to pixels drawn by that paint. The interval
starts at the beginning of the first blend pass and ends at the end of the last
blend pass. It excludes later planar conversion, readback copy, and window
presentation. Attribution
SHALL intersect actual supplied source spans with the effective paint event
region and content clip after the relevant transforms. A GPU
duration sample SHALL be counted at most once, including when one composite
result supplies multiple drawn tiles. If several samples qualify, their
durations SHALL be summed. The GPU value SHALL be absent when no attributable
sample is available, including reused pixels, CPU fallback, unavailable
timestamps, or results rejected or superseded before drawing. Device duration
SHALL be reported independently of CPU phase times and budget slack.

#### Scenario: A visible GPU result carries its timing once [fbi_gpu_result]

- **WHEN** one timestamped GPU composite supplies multiple newly drawn tiles
  and the canvas subsequently repaints those tiles from cache
- **THEN** the first paint reports the device duration once and the cache-only
  paint omits it

#### Scenario: Partial damage leaves disjoint samples pending [fbi_gpu_partial]

- **WHEN** a paint event draws a region disjoint from a pending repaired source
  span in the same visible tile or crop, and a later paint draws that span
- **THEN** the first paint omits the sample and the later paint reports it once

#### Scenario: Reduced stand-in attribution [fbi_gpu_standin]

- **WHEN** a timestamped reduced-level stand-in is accepted and drawn, or is
  overwritten or never drawn
- **THEN** only the paint drawing accepted stand-in pixels reports its sample

#### Scenario: Unrelated and unavailable GPU timings are absent [fbi_gpu_absent]

- **WHEN** a canvas paints after CPU fallback, unavailable timestamps, or GPU
  work whose result was unrelated, rejected, or superseded
- **THEN** its frame record omits GPU composite duration

### Requirement: Canvas frame timing is inert when disabled

With `PICTURA_METRICS` off, the frame instrumentation SHALL read no clocks,
retain no timing state, and emit no frame records. The metrics setting SHALL
preserve canvas pixels, composited pixels, and saved bytes.

#### Scenario: Disabled canvas frame capture [fbi_disabled]

- **WHEN** equivalent viewport operations and an edit run with metrics off and
  on
- **THEN** the drawn pixels and saved bytes match across runs, and the disabled
  run retains no frame records
