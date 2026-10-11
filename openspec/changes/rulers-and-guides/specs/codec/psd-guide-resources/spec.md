## ADDED Requirements

### Requirement: Guides in PSD

A PSD/PSB save SHALL write the document's guides as the grid-and-guides image
resource (1032): version 1, the horizontal and vertical grid cycles, the guide
count, then per guide a big-endian 32-bit location in 1/32 document pixels and
a direction byte (0 vertical, 1 horizontal). A read SHALL restore the guides
from that resource. A document whose guides still match its stored resource
SHALL keep its bytes unchanged (including its grid cycles); a document with no
guides SHALL write no 1032 resource; other resources SHALL be preserved.

#### Scenario: psd-tools reads the written guides

- **WHEN** the `guide_resource_oracle` test writes a 200×100 document with a vertical guide at 40 and a horizontal guide at 12.5 and reads it with psd-tools
- **THEN** psd-tools' `GridGuidesInfo` lists version 1 and the guides `(1280, 0)` and `(400, 1)` in order

#### Scenario: Round trip and verbatim re-emission

- **WHEN** the codec unit tests write and read guides, re-emit a section whose guides are unchanged, and delete every guide
- **THEN** the guides read back exactly, the unchanged section is byte-identical, and deleting the guides removes the resource while another resource survives
