# Proposal: Size the document budget from the host's memory

## Why

The decode budget (#844) capped one document at a fixed eight planes, 16 GiB.
A large host could not open bigger documents, and a small or busy one could
still be pushed past its memory before the limit was reached.

## What changes

- `ImageBudget` gains `max_document_bytes`. By default it is three quarters
  of the memory available when the budget is read (Linux `MemAvailable`,
  macOS free + inactive + purgeable pages, Windows `ullAvailPhys`), and never
  less than one 2 GiB plane. If the host's memory cannot be read, it falls
  back to the previous 16 GiB.
- The PSD/PSB decode budget and the PDF per-file inflate budget both use it.
- The per-allocation cap stays 2 GiB, since it also sets Qt's limit and the
  import probe.

## Capabilities

- Modified: `codec/untrusted-input-limits` (the document budget requirement).

## Impact

- `pictura-codec`: `budget.rs`, `probe.rs`, `pdf/import/objects.rs`, and
  `ImageBudget` literals in tests.
- Dependency: `windows-sys` on Windows (already in `Cargo.lock`) for
  `GlobalMemoryStatusEx`; macOS uses the existing `libc`.
