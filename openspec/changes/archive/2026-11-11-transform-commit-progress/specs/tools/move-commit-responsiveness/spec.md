## MODIFIED Requirements

### Requirement: A large transform commit leaves the GUI thread

A Free Transform commit SHALL resample the transform across rows in parallel,
treating pixels outside the source layer rect as transparent so an opaque edge
anti-aliases when it is rotated or scaled. When the transformed layer rect
reaches 4 MP, the engine transform SHALL run as a cancellable job (see
`document/long-operation-progress`). This includes a lifted selection transform
and a warp. The job's progress SHALL count the resampled output rows. The
transform handles and image SHALL stay presented while the job runs. On
completion, the landing SHALL install the transformed layers, record one history
state under the session's label, and leave the region composite and the exact
pyramid to a worker when its dirty union reaches 16 MP. The transform image SHALL
remain presented until `moveCommitted`. On cancel, the transform session SHALL
resume with its handles, and the document and history SHALL be unchanged. A
smaller transform SHALL commit synchronously, and a commit that does not defer
SHALL report `moveCommitted` before returning.

#### Scenario: A large transform release returns to Move [mcr_transform_deferred]

- **WHEN** a canvas-sized layer's Free Transform is committed on a document over 16 MP while the job is held by its test seam
- **THEN** the `jobProgress` window appears, its value rises, and on landing the history gains one `"Free Transform"` state, the transform handles are gone, and the transform image stays until `moveCommitted`

#### Scenario: Cancelling a large transform resumes the session [mcr_transform_cancel]

- **WHEN** a large Free Transform commit is cancelled from its progress window
- **THEN** the transform session is active again with the same handles, the document is pixel-identical to before the commit, and the history count is unchanged

#### Scenario: A large lifted transform runs as a job [mcr_transform_lifted_job]

- **WHEN** a Free Transform of selected pixels over 4 MP is committed and then cancelled
- **THEN** the lifted session resumes, and after it is committed again the history gains one state and the pixels are merged into the layer
