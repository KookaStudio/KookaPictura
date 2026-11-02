# Design

Every write clears a plane's stamp, so a nonzero stamp held by both sides cannot
describe different bytes. Keeping it is safe.

A duplicated layer's planes carry their source's stamp. `planes_moved` needs
stamps unique per plane, so a stamp seen earlier in the same pass is replaced
with a fresh one on both sides, at the same index.

`Plane::ident` would also survive a capture, but the residency contract is that
an unstamped plane is never cached, so the keys stay on stamps.
