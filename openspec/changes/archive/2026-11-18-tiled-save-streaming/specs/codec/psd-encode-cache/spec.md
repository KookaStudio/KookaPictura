## MODIFIED Requirements

### Requirement: Layer channels encode in parallel

The writer SHALL encode the rows of each band of a layer or composite channel
on multiple worker threads, while emitting records, channels and rows in the
same order as a serial encode. The parallel output SHALL be byte-identical to
the serial output. A failing channel SHALL fail the encode and produce no
output.

#### Scenario: Parallel output equals serial output [pec_parallel_identical]

- **WHEN** a multi-layer document is encoded with parallel row encoding and with a single-threaded encode
- **THEN** both outputs are byte-identical

#### Scenario: A channel error fails the encode [pec_parallel_error]

- **WHEN** a layer channel cannot be encoded, such as a channel length mismatch or an oversized synthetic RLE scanline
- **THEN** the encode returns an error and produces no output

## ADDED Requirements

### Requirement: The encode cache holds a bounded number of bytes

The encode cache SHALL keep a newly encoded stream only while the cache's
resident bytes plus the stream fit its budget, and while the memory
governor's room exceeds the stream. The budget SHALL be the smaller of a fixed
cap and a share of the governor's ceiling, or the fixed cap when the host
reports no ceiling. A stream over the budget SHALL be written and not cached.
Before each encode the cache SHALL drop entries until it fits the current
budget. Budgeting SHALL NOT change the bytes written.

#### Scenario: A stream over the budget is not cached [pec_budget]

- **WHEN** a document is encoded through a cache whose budget is smaller than one of its encoded layer streams, and encoded again
- **THEN** the cache's resident bytes never exceed the budget, that layer's channels miss on the second encode, and both encodes are byte-identical to an uncached encode

#### Scenario: A lower budget trims the cache [pec_budget_trim]

- **WHEN** a document is encoded through a cache, its budget is lowered below its resident bytes, and the document is encoded again
- **THEN** after the second encode the cache's resident bytes fit the lower budget
