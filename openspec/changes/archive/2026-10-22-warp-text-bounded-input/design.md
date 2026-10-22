# Design

The clamp sits at the decode boundary, mirroring the one `StyleWarp::new`
already applies to geometry, so a file can never carry an out-of-range warp
into the model. The extent guard is relative: a preset at ±100 % moves the box
by roughly its own size, so a warped extent past 4× the unwarped span (+64 px)
can only come from corrupt geometry. It is refused the way empty text is: no
placement, no plane.
