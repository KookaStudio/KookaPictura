# Design: Harden the untrusted-document edges

## D1. One budget, charged before allocating

`DecodeBudget` (`pictura-codec/src/budget.rs`) wraps the default
`ImageBudget`. `charge(what, factors)` multiplies the factors with checked
arithmetic. It refuses the charge if the product is over `max_alloc_bytes`
(2 GiB by default) or over what the document has left, which starts at eight
times that (16 GiB). The PSD reader charges each composite plane right after
the header and each layer or mask channel right before it is decoded:

- a native 16/32-bit plane and its 8-bit narrowing as two allocations;
- a depth-1 plane as its single three-plane RGB expansion plus the copy.

The per-allocation cap still admits a 30000² 8- or 16-bit plane. The total
admits a 16000²-class RGB document with about a dozen full-canvas layers.
`read_psd_within` takes the budget explicitly; the existing entry points use
the default.

ponytail: the total is a fixed multiple, not a measure of the machine's
memory.

## D2. PDF

`PdfObjects` carries `stream_limit` and an `inflate_budget` cell.
`decode_stream` uses `read_bounded`. Predictor `/Columns` is capped at 2^24
and `/Colors` at 32, and the TIFF row size uses checked arithmetic that
refuses a zero-width row. `/N` of an object stream never sizes an
allocation, and negative or overflowing offsets are skipped.
The lexer's `depth` caps array and dictionary nesting at 256. The page and
form walks stop at depth 64, and `private_data` keeps a visited set. AESV2
with `/Length` other than 128 bits returns `Unsupported`, and AES-CBC decrypt
refuses keys that are not 16 or 32 bytes. The Photoshop editing-data rebuild
caps the composite at 30 000 px a side (the PSD version-1 limit) and refuses
any section length that does not fit its `u32` field.

## D3. Ghostscript

`run_bounded` spawns the child with its output piped. Two threads drain the
pipes, each keeping at most 16 MiB. The caller polls `try_wait` until the
deadline and then kills the child. On Unix, `pre_exec` sets `RLIMIT_AS`
(8 GiB), `RLIMIT_CPU` (the timeout), and `RLIMIT_FSIZE` (4 GiB) with
`setrlimit` only, which is async-signal-safe. `locate` runs `gs --version`
and refuses anything older than 9.55. `--permit-file-read` splits its
argument on the path-list separator, so an input path that contains `:`
(`;` on Windows) is copied into the private scratch directory and the
arguments are rewritten to use the copy.

## D4. Qt and the dialog

`decode_image_rgba` reads through a `QImageReader`. It checks
`reader.size()` against the budget before calling `read()`, checks the
decoded size again, and sets Qt's allocation limit to 2048 MB. The constants
mirror `ImageBudget::default()`. `unpackThumbnails` refuses a side over
30 000 and compares sizes as 64-bit values against the bytes left.

## D5. Control token

`listen()` writes 32 random bytes (`QRandomGenerator::system`), hex-encoded,
to `<socket>.token`. On Windows, where the socket is a named pipe, the file
goes in the temp dir. On POSIX, whatever entry exists at that name is
unlinked and the file is created with
`O_CREAT | O_EXCL | O_NOFOLLOW` and mode `0600`. A socket request without
the matching `token` member gets `unauthorized`, compared in constant time.
The check runs after the shape checks, so a malformed line still gets
`bad_request`. `dispatch()` stays tokenless because it runs in-process.
`shutdown()` removes the file.

ponytail: a same-user process that can read the owner's files can still read
the token. The token keeps the socket from being a cheaper path than the
filesystem itself.

## D6. Saves and entropy

`safe_io::write_atomically` writes `<path>.<16 hex>.tmp` with `create_new`,
optionally syncs it, renames it over the target, and removes the temp file on
failure. `pictura_codec::os_random` (`getrandom`) replaces the SipHash-based
entropy in the PDF writer and the Ghostscript scratch names.

## D7. Tests

Each finding has a crafted-input test. `tests/hostile_inputs.rs` installs a
tracking global allocator that records each thread's largest request and
refuses any request over 1 GiB. The crafted files must be refused with peak
requests under 4 MiB. A fixed-seed xorshift mutation pass runs over valid PSD
and PDF seeds (and PNG, in `png.rs`). It fails on any panic, and on any
allocation over 64 MiB under a 1 MiB per-plane budget. GUI edges are tested
in `tst_untrusted_input`.
