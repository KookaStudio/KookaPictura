# Design

## Reader

`read_layer_section` already isolates the trailing blocks. When `info_len` is 0
it splits out the block keyed for the document depth and runs `read_layer_info`
on a `Reader` over that block. Channel offsets are relative to the reader's data,
so the existing bound on declared channels applies to the block.

The block is removed from `layer_section_extra`; keeping it would write a stale
copy beside the rewritten layers. A key that does not match the depth is left as a
preserved block. A malformed block fails the open, as a malformed subsection does.

## Writer

At output depth 16 or 32 with layers, `write_psd` emits a zero `info_len`, the
global mask, then the tagged block ahead of the other trailing blocks. Without
this, opening and saving a Photoshop file would move its layers to a layout
Photoshop does not write and which cannot be tested against Photoshop here.
