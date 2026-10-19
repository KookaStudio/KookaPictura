# Design

## Phase weights

The worker reports only phase boundaries. The weights are chosen so the bar
moves where the time goes on a large PSD: the decode and composite dominate.

| Open kind | Phase | Percent |
|---|---|---|
| PSD/PSB | Reading file | 5 |
| | Decoding | 25 |
| | Compositing | 65 |
| | Preparing layers | 90 |
| Raster | Reading file | 5 |
| | Decoding | 25 |
| | Compositing | 70 |
| | Preparing layers | 90 |
| Smart object | Reading file | 5 |
| | Decoding | 35 |
| | Compositing | 70 |
| | Preparing layers | 90 |
| Photoshop PDF probe | Reading file | 5 |
| | Checking PDF | 15 |
| | Decoding | 35 |
| | Compositing | 65 |
| | Preparing layers | 90 |
| PDF page / image | Rendering page / image | 10 |
| | Preparing layers | 80 |
| | Compositing | 90 |
| | Preparing layers | 95 |
| EPS | Rendering EPS | 10 |
| | Preparing layers | 80 |
| | Compositing | 90 |
| | Preparing layers | 95 |

The values are not measured. A phase that takes seconds holds its percentage;
the tab spinner and the card's phase text show that work is still happening.

## Worker callback

The decode/import helpers gain a `&dyn Fn(u8, &str)` parameter:

- `decode_psd`, `decode_image`, `decode_smart_object`, `decode_pdf_psd`,
  `imported_document`
- the synchronous `open` / `open_image` / `open_as_smart_object` / `pdf_open`
  and the synchronous import bridge pass `&|_, _| {}`.

`open_jobs::start` creates the callback from the view's `CxxQtThread`:

```rust
let progress_thread = qt_thread.clone();
let progress = move |percent: u8, phase: &str| {
    let phase = phase.to_string();
    progress_thread.queue(move |view| {
        if !view.rust().open_cancelled {
            view.open_progress(i32::from(percent), QString::from(phase.as_str()));
        }
    }).ok();
};
```

The queued `openProgress` signal is delivered on the GUI thread before the
queued `openFinished` signal, because both are posted by the same worker in
order. The `open_pdf_start` probe builds the same callback; progress before
`openDetected` has no tab and is ignored.

## Signal and frame wiring

`PictureView` gains:

```rust
#[qsignal]
#[cxx_name = "openProgress"]
fn open_progress(self: Pin<&mut Self>, percent: i32, phase: QString);
```

`wireOpenSignals` connects it to `onOpenProgress(view, percent, phase)`, which
finds the loading entry by view and calls `LoadingCard::setProgress`. No token
is needed: the view identifies the tab, and a probe without a tab has no entry.

## The card

`LoadingCard` is a `QFrame` (`objectName` `loadingCard`) with:

- the file name, bold and word-wrapped;
- the phase label (`Starting…`, then the worker's phase);
- a `QProgressBar` (0–100, text hidden, 6 px high);
- a footer with the percentage on the left and a `Cancel` button on the right.

The card is centered in the loading page by a `QStackedLayout`: page 0 is a
wrapper with stretches above and below the card; page 1 is the canvas host added
on finish. `finishLoadingDocument` hides/deletes the card, adds the canvas host,
and switches the stack. `removeDocument` deletes the page, which owns the card
and the canvas.

The card's Cancel and the tab spinner use the same deferred
`cancelOpenForView` call, so deleting the card inside its own click handler is
safe. The card uses the existing `QProgressBar` style (`surface-input` track,
`selection` chunk); the frame rule uses `surface`, `border`, `content`, and
`content-body`.

## No generic-PDF card

`startOpenPdfPath` probes on a temporary view with no tab. `openProgress`
events before `openDetected` find no `DocEntry` and are ignored. The generic
PDF's Import PDF dialog therefore has no card; the first page/image render
creates a tab and a card at 10%.

## Test seam

`setOpenDelayForTest(ms)` already holds completion after the decode. The phase
updates are queued before that hold, so a test can observe the card at 90%
(`Preparing layers…`) while the tab is still loading. `LoadingCard` exposes
`progressForTest()`, `phaseForTest()`, and `percentForTest()`; the Cancel button
is `loadingCardCancel`.

## Ceilings (`ponytail:`)

- Phase weights only; no measured sub-progress.
- A long decode holds at 25% (raster) or 25% (PSD) while the spinner and phase
  text show liveness.
- No modal dialog; the card is inside the loading tab.
- No progress for synchronous Revert/Edit Contents wrappers.
